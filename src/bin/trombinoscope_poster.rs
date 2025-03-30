use pdfium_render::prelude::*;

use trombinoscope::poster;
use trombinoscope::tools;

use regex::Regex;

use std::fs;

fn main() -> () {
    // Create caching folder, if not already present
    let cache_dir = std::path::Path::new("cache");
    fs::create_dir_all(cache_dir).unwrap();

    // Read archive contents
    let fname = std::path::Path::new("PJ illustrés 2024 V5.zip");
    let mut pictures: Vec<(String, Vec<u8>)> = tools::load_images_from_archive(fname).unwrap();
    let nb_pics = pictures.len() as i32;

    // Sort images per prefix & character name
    pictures.sort_by(|a, b| a.0.cmp(&b.0));

    // Remove prefix and file extension
    let re = Regex::new(r"^(?:[^_]+)_([^.]+)\..+$").unwrap();

    for entry in pictures.iter_mut() {
        let caps = re.captures(&entry.0).unwrap();
        entry.0 = caps[1].to_string();
    }

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

    let title = fname.file_stem().unwrap().to_str().unwrap().into();

    // Generate PDFs at different target DPIs
    for dpi in [300, 600, 1200, 0] {
        let config = poster::RenderConfig {
            max_dpi: if dpi > 0 { Some(dpi) } else { None },
            ..base_config
        };

        let filename: String = match config.max_dpi {
            Some(dpi) => format!("trombinoscope-poster-{dpi}dpi.pdf"),
            None => "trombinoscope-poster.pdf".to_string(),
        };

        poster::generate(
            &pdfium, &pictures, nb_rows, nb_columns, &config, &filename, cache_dir, &title,
        )
        .unwrap();
    }
}
