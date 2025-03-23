use crate::tools;
use image::imageops::FilterType;
use image::ImageReader;
use image::{self, DynamicImage, GenericImageView};
use pdfium_render::prelude::*;
use std::fs;
use std::io::BufReader;
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

        // Decode JPEG data
        let buff_reader = Cursor::new(pict_data);
        let src_reader = ImageReader::new(buff_reader).with_guessed_format().unwrap();
        let src_image = match src_reader.decode() {
            Ok(img) => img,
            Err(ref e) => {
                // Print error, load a placeholder and move on
                println!("An error occured when decoding {name}: {e}");
                let fname = std::path::Path::new("logos/COPS_logo.png");
                let file = fs::File::open(fname).unwrap();
                let reader = BufReader::new(file);
                let image = ImageReader::new(reader)
                    .with_guessed_format()
                    .unwrap()
                    .decode()
                    .unwrap();
                image
            }
        };

        let (src_width, src_height) = src_image.dimensions();
        let image_ratio = src_height as f32 / src_width as f32;

        // First crop image to make sure it will fill cell completely
        let cropped: DynamicImage;
        {
            let x: u32;
            let y: u32;
            let width: u32;
            let height: u32;
            if cell_ratio > image_ratio {
                // Cell is proportionally taller than image => need to crop image left and/or right
                height = src_height as u32;
                width = (height as f32 / cell_ratio) as u32;
                x = (src_width as u32 - width) / 2;
                y = 0;
            } else {
                // Need to crop image top and/or bottom
                width = src_width as u32;
                height = (width as f32 * cell_ratio) as u32;
                x = 0;
                y = (src_height as u32 - height) / 2;
            }
            cropped = src_image.crop_imm(x, y, width, height);
        }

        // Image was cropped => it trivially fits cell bounds
        let image_width = cell_width;
        let image_height = cell_height;

        let dpi = tools::compute_dpi(src_width as usize, PdfPoints::new(image_width).to_cm());
        println!("Resolution of {name}: {dpi} DPI");

        // Resize the image if needed to target max DPI
        let mut resized: DynamicImage = cropped;
        match config.max_dpi {
            Some(max_dpi) => {
                if dpi > max_dpi {
                    let dpi_ratio: f32 = max_dpi as f32 / dpi as f32;
                    let dst_width = (src_width as f32 * dpi_ratio) as u32;
                    let dst_height = (src_height as f32 * dpi_ratio) as u32;
                    println!("Need resizing to ({dst_width}, {dst_height}) to reach target resolution ({max_dpi} DPI)");

                    // Resize image
                    resized = resized.resize(dst_width, dst_height, FilterType::Lanczos3);
                }
            }
            None => { /* Nothing to do, image does not reach target DPI */ }
        }

        // Get JPEG-encoded data
        let mut bytes: Vec<u8> = Vec::new();
        let encoding_result = resized
            .into_rgb8() // Avoid JPEG encoding error when an alpha channel is present in source image
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Jpeg);
        if let Err(e) = encoding_result {
            panic!("An error occured when encoding {name} to JPEG: {e}");
        }

        // Center image horizontally, but keep it at cell bottom
        let img_left = cell_left + (cell_width - image_width) / 2.0;
        let img_bottom = cell_bottom;

        // Build a PDF image object with DCTDecode (JPEG-encoded) data
        let mut object = PdfPageImageObject::new_from_jpeg_reader(&document, Cursor::new(&bytes))?;

        // Expected transformations order in PDF is "scaling, then rotation, then translation"
        // "The returned page object will have its width and height both set to 1.0 points"
        object.scale(image_width, image_height)?;
        object.translate(PdfPoints::new(img_left), PdfPoints::new(img_bottom))?;
        page.objects_mut().add_image_object(object)?;
    }

    document.save_to_file(&filename)?;

    println!("Done.");
    Ok(())
}
