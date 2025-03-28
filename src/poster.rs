use crate::tools;
use image::imageops::FilterType;
use image::{self, DynamicImage};
use imagesize;
use pdfium_render::prelude::*;
use std::io::Cursor;

pub struct RenderConfig {
    pub page_hmargin: f32,
    pub page_vmargin: f32,
    pub inner_hmargin: f32, // This is the margin between cells
    pub inner_vmargin: f32,
    pub max_dpi: Option<u32>, // None means "no images downsizing" (max possible DPI)
}

pub fn generate(
    pdfium: &Pdfium,
    pictures: &Vec<(String, Vec<u8>)>,
    nb_rows: i32,
    nb_columns: i32,
    config: &RenderConfig,
    filename: &String,
    cache_dir: &std::path::Path,
) -> Result<(), PdfiumError> {
    let mut document = pdfium.create_new_pdf()?;

    let mut page = document
        .pages_mut()
        .create_page_at_start(PdfPagePaperSize::a3().landscape())?;

    let _font = document.fonts_mut().times_roman();

    // Do calculations in PDF points (natural PDF unit)
    let page_width = page.width().value;
    let page_height = page.height().value;

    let cell_width: f32 = ((page_width - config.page_hmargin * 2.)
        - (config.inner_hmargin * (nb_columns - 1) as f32))
        / nb_columns as f32;
    let cell_height: f32 = ((page_height - config.page_vmargin * 2.)
        - (config.inner_vmargin * (nb_rows - 1) as f32))
        / nb_rows as f32;
    let cell_ratio = cell_height / cell_width;

    //let font = document.fonts_mut().courier_bold();

    let font = document
        .fonts_mut()
        .load_true_type_from_bytes(include_bytes!("../font/Chandler42 Regular.otf"), true)?;

    // Place cells. Note that origin is at bottom left in PDF coordinates system
    for (i, (name, pict_data)) in pictures.iter().enumerate() {
        let row: i32 = i as i32 / nb_columns;
        let column: i32 = i as i32 % nb_columns;
        let cell_bottom: f32 = page_height
            - (config.page_vmargin
                + cell_height
                + row as f32 * (cell_height + config.inner_vmargin));
        let cell_left: f32 =
            config.page_hmargin + column as f32 * (cell_width + config.inner_hmargin);

        // Get image dimensions
        let image_size = imagesize::blob_size(pict_data).expect("Could not guess image size");
        let src_width = image_size.width as u32;
        let src_height = image_size.height as u32;

        // First compute the cropping required to make sure the imnage will fill cell completely
        let crop = crop_to_fit_cell(src_width, src_height, cell_ratio);

        // Once image is cropped, it trivially fits cell bounds
        let image_width = cell_width;
        let image_height = cell_height;

        let dpi = tools::compute_dpi(src_width as usize, PdfPoints::new(image_width).to_cm());

        let cached_name = get_cached_name(name, &crop, config);
        let cached_path = cache_dir.join(tools::sanitize_filename(&cached_name));

        // Try to load from cached bytes;
        // otherwise perform image transforms and rencoding
        let bytes = match tools::load_bytes_from_disk(&cached_path) {
            Some(b) => {
                println!("Using cached image for {name}");
                b
            }
            None => {
                // Actually decode JPEG data
                let src_image = tools::decode_image(pict_data, name);

                // Actually crop image data
                let cropped = src_image.crop_imm(crop.x, crop.y, crop.width, crop.height);

                // Resize the image if needed to target max DPI
                let mut resized: DynamicImage = cropped;
                match config.max_dpi {
                    Some(max_dpi) => {
                        if dpi > max_dpi {
                            let dpi_ratio: f32 = max_dpi as f32 / dpi as f32;
                            let dst_width = (src_width as f32 * dpi_ratio) as u32;
                            let dst_height = (src_height as f32 * dpi_ratio) as u32;
                            println!("Resolution of {name}: {dpi} DPI");
                            println!("Need resizing to ({dst_width}, {dst_height}) to reach target resolution ({max_dpi} DPI)");

                            // Resize image
                            resized = resized.resize(dst_width, dst_height, FilterType::Lanczos3);
                        }
                    }
                    None => { /* Nothing to do, image does not reach target DPI */ }
                }

                // Get JPEG-encoded data
                let bytes = tools::encode_to_jpeg(resized, name);

                // Cache final image data on disk
                tools::save_bytes_to_disk(&cached_path, &bytes);

                bytes
            }
        };

        // Center image horizontally, but keep it at cell bottom
        let img_left = cell_left + (cell_width - image_width) / 2.0;
        let img_bottom = cell_bottom;

        // Build a PDF image object with DCTDecode (JPEG-encoded) data
        let mut image_object =
            PdfPageImageObject::new_from_jpeg_reader(&document, Cursor::new(&bytes))?;

        // Expected transformations order in PDF is "scaling, then rotation, then translation"
        // "The returned page object will have its width and height both set to 1.0 points"
        image_object.scale(image_width, image_height)?;
        image_object.translate(PdfPoints::new(img_left), PdfPoints::new(img_bottom))?;
        page.objects_mut().add_image_object(image_object)?;

        // Emit the label
        let font_size = 5.0;

        let mut text_object = PdfPageTextObject::new(
            &document,
            tools::normalize_unicode(name),
            font,
            PdfPoints::new(font_size),
        )?;

        //object.set_fill_color(PdfColor::new(random(), random(), random(), 255))?;

        let text_bounds = text_object.bounds().unwrap();
        let text_width = text_bounds.x3.value - text_bounds.x1.value;
        let hspace = cell_width - text_width;

        text_object.translate(
            PdfPoints::new(img_left + hspace / 2.),
            PdfPoints::new(cell_bottom - config.inner_vmargin / 2.),
        )?;

        // Add the object to the page, triggering content regeneration.
        page.objects_mut().add_text_object(text_object)?;
    }

    document.save_to_file(&filename)?;

    println!("Done.");
    Ok(())
}

/// Compute how to crop image to make sure it will fill cell completely
fn crop_to_fit_cell(src_width: u32, src_height: u32, cell_ratio: f32) -> Rectangle {
    let x: u32;
    let y: u32;
    let width: u32;
    let height: u32;
    let image_ratio = src_height as f32 / src_width as f32;
    if cell_ratio > image_ratio {
        // Cell is proportionally taller than image => need to crop image left and/or right
        height = src_height as u32;
        width = (height as f32 / cell_ratio) as u32;
        x = (src_width as u32 - width) / 2;
        y = 0;
    } else {
        // Need to crop image top and/or bottom
        // To respect faces, crop bottom (less chance to cut top of hair)
        width = src_width as u32;
        height = (width as f32 * cell_ratio) as u32;
        x = 0;
        y = 0; // (src_height as u32 - height) / 2;
    }
    return Rectangle {
        x,
        y,
        width,
        height,
    };
}

struct Rectangle {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

fn get_cached_name(name: &String, crop: &Rectangle, config: &RenderConfig) -> String {
    return format!(
        "{name}-cropped({},{},{},{})-dpi({}).jpg",
        crop.x,
        crop.y,
        crop.width,
        crop.height,
        config
            .max_dpi
            .map(|n| u32::to_string(&n))
            .unwrap_or(String::from("native"))
    );
}
