use image::{GenericImageView, ImageReader};
use pdfium_render::prelude::*;
use std::io::Cursor;
use crate::tools;

pub fn generate(
    pdfium: &Pdfium,
    pictures: &Vec<(String, Vec<u8>)>,
    nb_rows: i32,
    nb_columns: i32,
) -> Result<(), PdfiumError> {
    let mut document = pdfium.create_new_pdf()?;

    let mut page = document
        .pages_mut()
        .create_page_at_start(PdfPagePaperSize::a3().landscape())?;

    let _font = document.fonts_mut().times_roman();

    // Do calculations in PDF points (natural PDF unit)
    let page_width = page.width().value;

    let page_height = page.height().value;

    // Easier to express margins in mm
    // TODO Those should be passed as parameters
    let page_hmargin: f32 = PdfPoints::from_mm(10.).value;
    let page_vmargin: f32 = PdfPoints::from_mm(10.).value;
    let inner_margin: f32 = PdfPoints::from_mm(10.).value; // This is the margin between cells

    let cell_width: f32 = ((page_width - page_hmargin * 2.)
        - (inner_margin * (nb_columns - 1) as f32))
        / nb_columns as f32;
    let cell_height: f32 = ((page_height - page_vmargin * 2.)
        - (inner_margin * (nb_rows - 1) as f32))
        / nb_rows as f32;

    // Place cells. Note that origin is at bottom left in PDF coordinates system
    for (i, (name, pict_data)) in pictures.iter().enumerate() {
        let row: i32 = i as i32 / nb_columns;
        let column: i32 = i as i32 % nb_columns;
        let cell_top: f32 = page_height - (page_vmargin + row as f32 * (cell_width + inner_margin));
        let cell_left: f32 = page_hmargin + column as f32 * (cell_height + inner_margin);

        let mut object = PdfPageImageObject::new_from_jpeg_reader(
            &document,
            Cursor::new(pict_data),
        )?;

        let image = object.get_raw_image()?;

        let (jpeg_width, jpeg_height) = image.dimensions();
        let image_ratio = jpeg_height as f32 / jpeg_width as f32;
        let image_width = cell_width; 
        let image_height = image_width * image_ratio;

        let dpi = tools::compute_dpi(jpeg_width, PdfPoints::new(image_width).to_cm());
        println!("Resolution of {name}: {dpi} DPI");

        // TODO Resize the contents of pict_data with https://github.com/Cykooz/fast_image_resize,
        // re-encode to JPEG, then reload with code above? (or, just do a first read with ImageReader::with_format,
        // see previous versions of this code)

        // Expected transformations order in PDF is "scaling, then rotation, then translation"
        // "The returned page object will have its width and height both set to 1.0 points"
        object.scale(image_width, image_height)?;
        object.translate(PdfPoints::new(cell_left), PdfPoints::new(cell_top))?;
        page.objects_mut().add_image_object(object)?;
    }

    document.save_to_file("trombinoscope-poster.pdf")?;

    println!("Done.");
    Ok(())
}
