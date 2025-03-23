use pdfium_render::prelude::*;

use trombinoscope::poster;
use trombinoscope::tools;

fn main() -> () {
    // Read archive contents
    let fname = std::path::Path::new("PJ illustrés 2024 V3.zip");
    let pictures: Vec<(String, Vec<u8>)> = tools::load_images_from_archive(fname).unwrap();
    let nb_pics = pictures.len() as i32;

    // Define a grid size; we specify colums to have some control over ratio
    let nb_columns: i32 = 19;
    let mut nb_rows: i32 = nb_pics / nb_columns;
    if nb_columns * nb_rows < nb_pics {
        nb_rows += 1;
    }
    println!("{nb_pics} pictures to layout in a ({nb_columns} x {nb_rows}) grid");

    let pdfium = Pdfium::default();
    let base_config = poster::RenderConfig {
        // Easier to express margins in mm
        page_hmargin: PdfPoints::from_mm(10.).value,
        page_vmargin: PdfPoints::from_mm(10.).value,
        inner_hmargin: PdfPoints::from_mm(1.).value,
        inner_vmargin: PdfPoints::from_mm(5.).value,
        max_dpi: None,
    };

    // Generate PDFs at different target DPIs
    for dpi in [600]
    /*[300, 600, 1200, 0]*/
    {
        let config = poster::RenderConfig {
            max_dpi: if dpi > 0 { Some(dpi) } else { None },
            ..base_config
        };

        let filename: String = match config.max_dpi {
            Some(dpi) => format!("trombinoscope-poster-{dpi}dpi.pdf"),
            None => "trombinoscope-poster.pdf".to_string(),
        };

        poster::generate(&pdfium, &pictures, nb_rows, nb_columns, &config, &filename).unwrap();
    }
}
