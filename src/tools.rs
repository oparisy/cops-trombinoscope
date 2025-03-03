use std::fs;
use std::io;
use std::io::{BufReader, Read};
use std::path::Path;

pub fn load_images_from_archive(archive_path: &Path) -> io::Result<Vec<(String, Vec<u8>)>> {
    // Read archive contents
    //let fname = std::path::Path::new("COPS selection PNJ.zip");
    let file = fs::File::open(archive_path).unwrap();
    let reader = BufReader::new(file);

    let mut archive = zip::ZipArchive::new(reader).unwrap();

    let mut files: Vec<(String, Vec<u8>)> = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        if let None = file.enclosed_name() {
            println!("Entry {} has a suspicious path", file.name());
            continue;
        }

        let filepath = String::from_utf8(file.name_raw().to_vec()).unwrap();
        let filename = String::from(Path::new(&filepath).file_name().unwrap().to_str().unwrap());

        if file.is_dir() || filepath.starts_with("__MACOSX") || (filename == ".DS_Store") {
            continue;
        }
        
        println!(
            "Entry {} is a file with name \"{}\" ({} bytes)",
            i,
            filename,
            file.size()
        );

        // Read whole file contents
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        assert!(file.size() == buffer.len().try_into().unwrap());

        files.push((filename, buffer));
    }

    return Ok(files);
}

/** Compute a resolution in DPI (PPI actually) from a definition (pixel size) and its printed size (in cm) */
pub fn compute_dpi(pixel_size: u32, cm_size: f32) -> u32 {
    return (pixel_size as f32 * 2.54 / cm_size) as u32;
}