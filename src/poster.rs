use image::{GenericImageView, ImageReader};
use pdfium_render::prelude::*;
use std::io::Cursor;

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

        let to_decode = ImageReader::with_format(Cursor::new(pict_data), image::ImageFormat::Jpeg)
            .with_guessed_format()
            .unwrap();
        let image = match to_decode.decode() {
            Ok(decoded) => decoded,
            Err(msg) => panic!("An error occured while decoding image #{} ({name}: {msg}", i+1)
        };
        let dimensions = image.dimensions();
        let image_ratio = dimensions.0 as f32 / dimensions.1 as f32;
        let image_width = cell_width; 
        let image_height = image_width * image_ratio;

        page.objects_mut().create_image_object(
            PdfPoints::new(cell_left),
            PdfPoints::new(cell_top),
            &image,
            Some(PdfPoints::new(image_width)),
            Some(PdfPoints::new(image_height)),
        )?;
    }

    document.save_to_file("trombinoscope-poster.pdf")?;

    println!("Done.");
    Ok(())
}
