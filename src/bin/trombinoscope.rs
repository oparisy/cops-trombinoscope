use pdfium_render::prelude::*;
use std::f64::consts::SQRT_2;
//use zip::read::ZipFile;
use std::fs;
//use std::intrinsics::sqrtf64;
use std::io::BufReader;
use std::path::Path;


fn main() -> Result<(), PdfiumError> {
    // Read archive contents
    let fname = std::path::Path::new("COPS selection PNJ.zip");
    let file = fs::File::open(fname).unwrap();
    let reader = BufReader::new(file);

    let mut archive = zip::ZipArchive::new(reader).unwrap();

    let mut files: Vec<(String, usize)> = Vec::new();

    for i in 0..archive.len() {
        let file = archive.by_index(i).unwrap();
        if let None = file.enclosed_name() {
            println!("Entry {} has a suspicious path", file.name());
            continue;
        }

        let filepath = String::from_utf8(file.name_raw().to_vec()).unwrap();
        let filename = String::from(Path::new(&filepath).file_name().unwrap().to_str().unwrap());

        if file.is_dir() || filepath.starts_with("__MACOSX") {
            continue;
        }
        
        println!(
            "Entry {} is a file with name \"{}\" ({} bytes)",
            i,
            filename,
            file.size()
        );

        files.push((filename, i));
    }

    let nb_pics = files.len() as i32;

    // Define a grid size with a ratio near A4 proportions
    let nb_colums: i32 = (f64::sqrt(f64::from(nb_pics) / SQRT_2)).round() as i32;
    let mut nb_rows: i32 = nb_pics / nb_colums;
    if nb_colums * nb_rows < nb_pics {
        nb_rows += 1;
    }
    println!("{nb_pics} pictures to layout in a ({nb_colums} x {nb_rows}) grid");

    // Generate PDFs
    let pdfium = Pdfium::default();
    generate_page(&pdfium, true)?;
    generate_page(&pdfium, false)
}

fn generate_page(pdfium: &Pdfium, debug: bool) -> Result<(), PdfiumError> {
    //let pdfium = Pdfium::default();

    let mut document = pdfium.create_new_pdf()?;

    let mut page = document
        .pages_mut()
        .create_page_at_start(PdfPagePaperSize::a3().landscape())?;

    let _font = document.fonts_mut().times_roman();

    let page_width = page.width();

    let page_height = page.height();

    if debug {
        println!("Page size (in PdfPoints): w={page_width}, h={page_height}");
    }

    // ... add some path objects to the page...

    let debug_color = PdfColor::GREY_60;
    let left_right_ext_margin = PdfPoints::from_mm(15.0);
    let top_bottom_ext_margin = PdfPoints::from_mm(15.0);

    // Mark center of page
    if debug {
        let page_center_x = page_width / 2.;
        page.objects_mut()
            .create_path_object_line(
                PdfPoints::new(page_center_x.value),
                PdfPoints::new(0.),
                PdfPoints::new(page_center_x.value),
                PdfPoints::new(page_height.value),
                debug_color,
                PdfPoints::new(1.0),
            )?
            .set_dash_array(
                &[PdfPoints::new(2.0), PdfPoints::new(1.0)],
                PdfPoints::zero(),
            )?;
    }

    // Mark exterior margins
    if debug {
        // Left
        page.objects_mut().create_path_object_rect(
            PdfRect::new(
                PdfPoints::new(0.),
                PdfPoints::new(0.),
                PdfPoints::new(page_height.value),
                PdfPoints::new(left_right_ext_margin.value),
            ),
            Some(debug_color),
            Some(PdfPoints::new(1.0)),
            Some(debug_color),
        )?;

        // Right
        page.objects_mut().create_path_object_rect(
            PdfRect::new(
                PdfPoints::new(0.),
                PdfPoints::new(page_width.value - left_right_ext_margin.value),
                PdfPoints::new(page_height.value),
                PdfPoints::new(page_width.value),
            ),
            Some(debug_color),
            Some(PdfPoints::new(1.0)),
            Some(debug_color),
        )?;

        // Top
        page.objects_mut().create_path_object_rect(
            PdfRect::new(
                PdfPoints::new(page_height.value - top_bottom_ext_margin.value),
                PdfPoints::new(0.),
                PdfPoints::new(page_height.value),
                PdfPoints::new(page_width.value),
            ),
            Some(debug_color),
            Some(PdfPoints::new(1.0)),
            Some(debug_color),
        )?;

        // Bottom
        page.objects_mut().create_path_object_rect(
            PdfRect::new(
                PdfPoints::new(0.),
                PdfPoints::new(0.),
                PdfPoints::new(top_bottom_ext_margin.value),
                PdfPoints::new(page_width.value),
            ),
            Some(debug_color),
            Some(PdfPoints::new(1.0)),
            Some(debug_color),
        )?;
    }

    document.save_to_file(if debug {
        "trombinoscope-debug.pdf"
    } else {
        "trombinoscope.pdf"
    })?;

    println!("Done.");
    Ok(())
}
