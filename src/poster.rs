use crate::tools;
use image;
use image::imageops::FilterType;
use image::ImageReader;
use pdfium_render::prelude::*;
use std::io::Cursor;
use turbojpeg;

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

        // Read the JPEG header to compute image DPI at this print size
        // (significantly less expensive than object.get_raw_image)
        let mut decompressor = turbojpeg::Decompressor::new().unwrap();
        let header = decompressor.read_header(&pict_data).unwrap();

        let jpeg_width = header.width;
        let jpeg_height = header.height;
        let image_ratio = jpeg_height as f32 / jpeg_width as f32;

        // Compare ratios to make sure image stays in cell bounds
        let image_width: f32;
        let image_height: f32;
        if cell_ratio > image_ratio {
            // Cell is proportionally taller than image => limiting factor is cell width
            image_width = cell_width;
            image_height = image_width * image_ratio;
        } else {
            image_height = cell_height;
            image_width = image_height / image_ratio;
        }

        let dpi = tools::compute_dpi(jpeg_width, PdfPoints::new(image_width).to_cm());
        println!("Resolution of {name}: {dpi} DPI");

        // Resize the image if needed to target max DPI
        let mut final_data = pict_data;
        let mut bytes: Vec<u8> = Vec::new();
        match config.max_dpi {
            Some(max_dpi) => {
                if dpi > max_dpi {
                    let dpi_ratio: f32 = max_dpi as f32 / dpi as f32;
                    let dst_width = (jpeg_width as f32 * dpi_ratio) as u32;
                    let dst_height = (jpeg_height as f32 * dpi_ratio) as u32;
                    println!("Need resizing to ({dst_width}, {dst_height}) to reach target resolution ({max_dpi} DPI)");

                    // Decode JPEG data
                    let src_reader = ImageReader::new(Cursor::new(pict_data));
                    let src_image = src_reader.with_guessed_format().unwrap().decode().unwrap();

                    // Resize image
                    let resized = src_image.resize(dst_width,
                        dst_height,
                        FilterType::Lanczos3);

                    resized
                        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Jpeg)
                        .unwrap();
                    final_data = &bytes;
                }
            }
            None => {
                /* Nothing to do, image does not reach target DPI */
            }
        }

        // Center image horizontally, but keep it at cell bottom
        let img_left = cell_left + (cell_width - image_width) / 2.0;
        let img_bottom = cell_bottom;

        // Build a PDF image object with DCTDecode (JPEG-encoded) data
        let mut object =
            PdfPageImageObject::new_from_jpeg_reader(&document, Cursor::new(final_data))?;

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
