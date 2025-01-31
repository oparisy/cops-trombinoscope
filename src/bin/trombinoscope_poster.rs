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

    // Generate PDFs
    let pdfium = Pdfium::default();
    poster::generate(&pdfium, &pictures, nb_rows, nb_columns).unwrap();
}
