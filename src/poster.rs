use crate::tools;
use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder};
use image::{GenericImageView, ImageReader};
use pdfium_render::prelude::*;
use std::io::Cursor;
use std::time::Instant;
use turbojpeg;

use fast_image_resize::images::Image;
use fast_image_resize::{IntoImageView, Resizer};

pub struct RenderConfig {
    pub page_hmargin: f32,
    pub page_vmargin: f32,
    pub inner_margin: f32,       // This is the margin between cells
    pub target_dpi: Option<u32>, // None means "no images downsizing" (max possible DPI)
}

pub fn generate(
    pdfium: &Pdfium,
    pictures: &Vec<(String, Vec<u8>)>,
    nb_rows: i32,
    nb_columns: i32,
    config: &RenderConfig,
    filename: &String
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
        - (config.inner_margin * (nb_columns - 1) as f32))
        / nb_columns as f32;
    let cell_height: f32 = ((page_height - config.page_vmargin * 2.)
        - (config.inner_margin * (nb_rows - 1) as f32))
        / nb_rows as f32;

    // Place cells. Note that origin is at bottom left in PDF coordinates system
    for (i, (name, pict_data)) in pictures.iter().enumerate() {
        let row: i32 = i as i32 / nb_columns;
        let column: i32 = i as i32 % nb_columns;
        let cell_bottom: f32 =
            page_height - (config.page_vmargin + cell_height + row as f32 * (cell_height + config.inner_margin));
        let cell_left: f32 =
            config.page_hmargin + column as f32 * (cell_width + config.inner_margin);

        let t1 = Instant::now();
        let mut object =
            PdfPageImageObject::new_from_jpeg_reader(&document, Cursor::new(pict_data))?;
        let elapsed_new_from_jpeg = t1.elapsed().as_millis();
        println!("new_from_jpeg_reader took {elapsed_new_from_jpeg}ms");

        // Read the JPEG header to compute image DPI at this print size
        // (significantly less expensive than object.get_raw_image)
        let t3 = Instant::now();
        let mut decompressor = turbojpeg::Decompressor::new().unwrap();
        let header = decompressor.read_header(&pict_data).unwrap();
        let elapsed_read_header = t3.elapsed().as_millis();
        println!("read_reader took {elapsed_read_header}ms");

        let t2 = Instant::now();
        let image = object.get_raw_image()?;
        let elapsed_get_raw_image = t2.elapsed().as_millis();
        println!("get_raw_image took {elapsed_get_raw_image}ms");

        let (jpeg_width, jpeg_height) = image.dimensions();
        let image_ratio = jpeg_height as f32 / jpeg_width as f32;
        let image_width = cell_width;
        let image_height = image_width * image_ratio;
        assert_eq!(
            (header.width, header.height),
            (jpeg_width as usize, jpeg_height as usize)
        );

        let dpi = tools::compute_dpi(jpeg_width, PdfPoints::new(image_width).to_cm());
        println!("Resolution of {name}: {dpi} DPI");

        // TODO Resize the contents of pict_data with https://github.com/Cykooz/fast_image_resize,
        // re-encode to JPEG, then reload with code above? (or, just do a first read with ImageReader::with_format,
        // see previous versions of this code)
        match config.target_dpi {
            Some(target_dpi) => {
                if dpi > target_dpi {
                    let dpi_ratio: f32 = target_dpi as f32 / dpi as f32;
                    let dst_width = (jpeg_width as f32 * dpi_ratio) as u32;
                    let dst_height = (jpeg_height as f32 * dpi_ratio) as u32;
                    println!("Need resizing to ({dst_width}, {dst_height}) to reach target resolution ({target_dpi} DPI)");
                }
            }
            None => { /* Nothing to do, image does not reach target DPI */}
        }

        // Expected transformations order in PDF is "scaling, then rotation, then translation"
        // "The returned page object will have its width and height both set to 1.0 points"
        object.scale(image_width, image_height)?;
        object.translate(PdfPoints::new(cell_left), PdfPoints::new(cell_bottom))?;
        page.objects_mut().add_image_object(object)?;
    }

    document.save_to_file(&filename)?;

    println!("Done.");
    Ok(())
}
