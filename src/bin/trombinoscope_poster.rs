use pdfium_render::prelude::*;
use std::f64::consts::SQRT_2;

use trombinoscope::poster;
use trombinoscope::tools;

fn main() -> () {
    // Read archive contents
    let fname = std::path::Path::new("COPS selection PNJ.zip");
    let pictures: Vec<(String, Vec<u8>)> = tools::load_images_from_archive(fname).unwrap();
    let nb_pics = pictures.len() as i32;

    // Define a grid size with a ratio near A3 proportions
    let nb_columns: i32 = (f64::sqrt(f64::from(nb_pics) * SQRT_2)).round() as i32;
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
        inner_margin: PdfPoints::from_mm(10.).value,
        target_dpi: None,
    };

    // Generate PDFs at different target DPIs
    for dpi in [300, 600, 1200, 0] {
        let config = poster::RenderConfig {
            target_dpi: if dpi > 0 { Some(dpi) } else { None },
            ..base_config
        };

        let filename: String = match config.target_dpi {
            Some(dpi) => format!("trombinoscope-poster-{dpi}dpi.pdf"),
            None => "trombinoscope-poster.pdf".to_string()
        };

        poster::generate(&pdfium, &pictures, nb_rows, nb_columns, &config, &filename).unwrap();
    }
}
