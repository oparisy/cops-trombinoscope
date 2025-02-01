use pdfium_render::prelude::*;
use std::f64::consts::SQRT_2;

use trombinoscope::tools;

fn main() -> Result<(), PdfiumError> {
    // Read archive contents
    let fname = std::path::Path::new("COPS selection PNJ.zip");
    let files: Vec<(String, Vec<u8>)> = tools::load_images_from_archive(fname).unwrap();

    // Generate PDFs
    let pdfium = Pdfium::default();
    generate_page(&pdfium, true, &files)?;
    generate_page(&pdfium, false, &files)
}

fn draw_debug_line(
    page: &mut PdfPage,
    debug_color: PdfColor,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    debug: bool,
) -> Result<(), PdfiumError> {
    if !debug {
        return Ok(());
    }

    page.objects_mut()
        .create_path_object_line(
            PdfPoints::new(x1),
            PdfPoints::new(y1),
            PdfPoints::new(x2),
            PdfPoints::new(y2),
            debug_color,
            PdfPoints::new(1.0),
        )?
        .set_dash_array(
            &[PdfPoints::new(2.0), PdfPoints::new(1.0)],
            PdfPoints::zero(),
        )?;

    return Ok(());
}

fn generate_page(
    pdfium: &Pdfium,
    debug: bool,
    files: &Vec<(String, Vec<u8>)>,
) -> Result<(), PdfiumError> {
    println!("Generating PDF with debug={debug}");

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

    // ... add objects to the page...

    let debug_color_dark = PdfColor::GREY_60;
    let debug_color_light = PdfColor::GREY_80;
    let line_debug_color = PdfColor::GREY_40;
    let left_right_ext_margin = PdfPoints::from_mm(15.0);
    let top_bottom_ext_margin = PdfPoints::from_mm(15.0);
    let center_margin = PdfPoints::from_mm(15.0);

    // Define a grid size with a ratio near A4 proportions
    let nb_pics = files.len() as i32;
    let nb_colums: i32 = (f64::sqrt(f64::from(nb_pics) / SQRT_2)).round() as i32;
    let mut nb_rows: i32 = nb_pics / nb_colums;
    if nb_colums * nb_rows < nb_pics {
        nb_rows += 1;
    }
    println!("{nb_pics} pictures to layout in a ({nb_colums} x {nb_rows}) grid");

    // A cell is the full area allocated to the picture + text + margins
    let nb_rows_first_page = (nb_rows + 1) / 2;
    let cell_width =
        (page_width.value - left_right_ext_margin.value * 2.0 - center_margin.value * 2.0)
            / (nb_colums as f32 * 2.0);
    let cell_height =
        (page_height.value - top_bottom_ext_margin.value * 2.0) / nb_rows_first_page as f32;

    // Show the debug grid
    for idx in 0..nb_pics {
        let on_left_page: bool = idx < nb_rows_first_page * nb_colums;

        let col: i32 = idx % nb_colums;
        let row: i32 = (idx / nb_colums) % nb_rows_first_page;

        let left: f32 = if on_left_page {
            left_right_ext_margin.value + col as f32 * cell_width
        } else {
            page_width.value / 2.0 + center_margin.value + col as f32 * cell_width
        };
        let top: f32 = page_height.value - (top_bottom_ext_margin.value + row as f32 * cell_height);

        if debug {
            let cell_color = if idx % 2 == 0 {
                debug_color_dark
            } else {
                debug_color_light
            };
            page.objects_mut().create_path_object_rect(
                PdfRect::new(
                    PdfPoints::new(top - cell_height), // Bottom left is (0,0)!
                    PdfPoints::new(left),
                    PdfPoints::new(top),
                    PdfPoints::new(left + cell_width),
                ),
                Some(cell_color),
                Some(PdfPoints::new(1.0)),
                Some(cell_color),
            )?;

            let font = document.fonts_mut().courier();
            let font_size = 10.0;
            let mut idx_text = PdfPageTextObject::new(
                &document,
                format!("{}", idx + 1),
                font,
                PdfPoints::new(font_size),
            )?;
            idx_text.translate(PdfPoints::new(left), PdfPoints::new(top - font_size))?;
            page.objects_mut().add_text_object(idx_text)?;
        }
    }

    // Mark center of page
    let page_center_x = page_width / 2.;
    draw_debug_line(
        &mut page,
        line_debug_color,
        page_center_x.value,
        0.,
        page_center_x.value,
        page_height.value,
        debug,
    )?;

    // Left exterior margin
    draw_debug_line(
        &mut page,
        line_debug_color,
        left_right_ext_margin.value,
        page_height.value,
        left_right_ext_margin.value,
        0.,
        debug,
    )?;

    // Right exterior margin
    draw_debug_line(
        &mut page,
        line_debug_color,
        page_width.value - left_right_ext_margin.value,
        page_height.value,
        page_width.value - left_right_ext_margin.value,
        0.,
        debug,
    )?;

    // Top exterior margin
    draw_debug_line(
        &mut page,
        line_debug_color,
        0.,
        page_height.value - top_bottom_ext_margin.value,
        page_width.value,
        page_height.value - top_bottom_ext_margin.value,
        debug,
    )?;

    // Bottom exterior margin
    draw_debug_line(
        &mut page,
        line_debug_color,
        0.,
        top_bottom_ext_margin.value,
        page_width.value,
        top_bottom_ext_margin.value,
        debug,
    )?;

    // Center margin (left)
    draw_debug_line(
        &mut page,
        line_debug_color,
        page_center_x.value - center_margin.value,
        page_height.value,
        page_center_x.value - center_margin.value,
        0.,
        debug,
    )?;

    // Center margin (right)
    draw_debug_line(
        &mut page,
        line_debug_color,
        page_center_x.value + center_margin.value,
        page_height.value,
        page_center_x.value + center_margin.value,
        0.,
        debug,
    )?;

    // Work around the fact that for some reason, the last drawn line is not dashed :)
    draw_debug_line(
        &mut page,
        PdfColor::WHITE.with_alpha(0),
        0.,
        0.,
        0.,
        0.,
        debug,
    )?;

    document.save_to_file(if debug {
        "trombinoscope-debug.pdf"
    } else {
        "trombinoscope.pdf"
    })?;

    println!("Done.");
    Ok(())
}
