use image::DynamicImage;
use image::ImageReader;
use std::fs;
use std::io;
use std::io::Write;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;
use unicode_normalization::UnicodeNormalization;

/// Load an archive, return (name, bytes) tuples
pub fn load_images_from_archive(archive_path: &Path) -> io::Result<Vec<(String, Vec<u8>)>> {
    // Read archive contents
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

/// Load an image from the provided path
pub fn load_image_from_disk(fname: &Path) -> DynamicImage {
    let file = fs::File::open(fname).unwrap();
    let reader = BufReader::new(file);
    let image = ImageReader::new(reader)
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();
    image
}

/// Load bytes from a file, if it ests at this path. Return `None` otherwise.
pub fn load_bytes_from_disk(fpath: &Path) -> Option<Vec<u8>> {
    return match std::fs::read(fpath) {
        Ok(bytes) => Some(bytes),
        Err(_) => None,
    };
}

/// Save provided bytes to the path specified
pub fn save_bytes_to_disk(fpath: &Path, bytes: &Vec<u8>) {
    let file = match std::fs::File::create(&fpath) {
        Ok(f) => f,
        Err(e) => panic!(
            "Error with path {} ({e})",
            fpath.as_os_str().to_str().unwrap()
        ),
    };
    std::io::BufWriter::new(file).write_all(&bytes).unwrap();
}

/// Decode provided image data
pub fn decode_image(pict_data: &Vec<u8>, name: &String) -> DynamicImage {
    let buff_reader = Cursor::new(pict_data);
    let src_reader = ImageReader::new(buff_reader).with_guessed_format().unwrap();
    return match src_reader.decode() {
        Ok(img) => img,
        Err(ref e) => {
            // More explicit error message
            panic!("An error occured when decoding {name}: {e}");
        }
    };
}

/// Get JPEG-encoded data for an image (which is consumed)
pub fn encode_to_jpeg(image: DynamicImage, name: &String) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::new();
    let encoding_result = image
        .into_rgb8() // Avoid JPEG encoding error when an alpha channel is present in source image
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Jpeg);
    if let Err(e) = encoding_result {
        panic!("An error occured when encoding {name} to JPEG: {e}");
    }
    return bytes;
}

/// Replace characters invalid in a (Windows) filename
pub fn sanitize_filename(filename: &String) -> String {
    return str::replace(filename.as_str(), '"', "_");
}

/// Compute a resolution in DPI (PPI actually) from a definition (pixel size) and its printed size (in cm)
pub fn compute_dpi(pixel_size: usize, cm_size: f32) -> u32 {
    return (pixel_size as f32 * 2.54 / cm_size) as u32;
}

/// Compose diacritics (those are not supported by pdfium-render)
pub fn normalize_unicode(to_normalize: &String) -> String {
    return to_normalize.nfc().collect();
}
