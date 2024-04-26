use image::io::Reader as ImageReader;
use std::io::BufReader;
use std::path::{Path, PathBuf};

fn ensure_resized_dir(original_dir_path: &str) -> PathBuf {
    let resized_dir_path = PathBuf::from(original_dir_path).join("resized");
    if !resized_dir_path.exists() {
        std::fs::create_dir(&resized_dir_path).expect("Failed to create resized directory");
    }
    resized_dir_path
}

fn get_file_paths(dir_path: &str) -> Vec<PathBuf> {
    std::fs::read_dir(dir_path)
        .map(|entries| {
            entries
                .filter_map(|entry| {
                    entry.ok().and_then(|entry| {
                        entry
                            .path()
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| ext.to_lowercase())
                            .filter(|ext| ext == "jpg" || ext == "jpeg")
                            .map(|_| entry.path())
                    })
                })
                .collect()
        })
        .unwrap_or_else(|_| {
            eprintln!("Error reading directory {}", dir_path);
            Vec::new()
        })
}

fn get_img_orientation(file_path: &Path) -> Option<u32> {
    let file = std::fs::File::open(file_path).expect("Failed to open file");
    let exif = exif::Reader::new()
        .read_from_container(&mut BufReader::new(&file))
        .expect("Failed to read EXIF metadata");
    exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .map(|field| field.value.as_uint().map_or(None, |u| u.get(0)))
        .flatten()
}

fn apply_orientation(img: image::DynamicImage, orientation: u32) -> image::DynamicImage {
    match orientation {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.rotate90().fliph(),
        6 => img.rotate90(),
        7 => img.rotate270().fliph(),
        8 => img.rotate270(),
        _ => img,
    }
}

fn resize(file_path: &Path, resized_file_path: &Path) {
    println!("Resizing {}...", file_path.display());
    let img = ImageReader::open(file_path).expect("Failed to open image");
    let _ = img
        .decode()
        .map(|img| {
            img.resize(
                img.width() / 2,
                img.height() / 2,
                image::imageops::FilterType::Lanczos3,
            )
        })
        .map(|img| match get_img_orientation(file_path) {
            Some(orientation) => apply_orientation(img, orientation),
            None => img,
        })
        .map(|img| img.save(resized_file_path.to_str().unwrap()))
        .inspect(|_| {
            println!(
                "Resized {} => {}",
                file_path.display(),
                resized_file_path.display()
            )
        })
        .map_err(|err| eprintln!("Error resizing {}: {}", file_path.display(), err));
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: img-resize <path to directory with JPG files>");
        return;
    }

    let dir_path = &args[1];
    let _ = std::fs::metadata(dir_path)
        .map(|m| {
            if m.is_dir() {
                let resized_dir_path = ensure_resized_dir(dir_path);
                get_file_paths(dir_path).iter().for_each(|path| {
                    resize(path, &resized_dir_path.join(path.file_name().unwrap()))
                });
            } else {
                eprintln!("{} is not a directory", dir_path);
            }
        })
        .map_err(|err| eprintln!("Error reading from {}: {}", dir_path, err));
}
