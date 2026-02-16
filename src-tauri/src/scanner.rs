//! Scan directories for image files and filter by filename suffix.

use std::collections::HashSet;
use std::path::Path;
use walkdir::WalkDir;

/// Image extensions (lowercase) we consider for matching and hashing.
pub const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "heic", "gif", "tiff", "tif", "webp"];

fn is_image_extension(ext: &std::ffi::OsStr) -> bool {
    let ext = ext.to_string_lossy().to_lowercase();
    IMAGE_EXTENSIONS.contains(&ext.as_str())
}

/// Check if the file's stem (filename without extension) ends with any of the suffix numbers.
pub fn stem_ends_with_suffix(stem: &str, suffixes: &HashSet<u32>) -> bool {
    for & suffix in suffixes {
        if stem.ends_with(&suffix.to_string()) {
            return true;
        }
    }
    false
}

/// One candidate image file (path relative to source root is computed by caller if needed).
#[derive(Clone, Debug)]
pub struct ImageEntry {
    pub path: std::path::PathBuf,
}

/// Recursively scan `source_dir` for image files whose stem ends with any of `suffixes`.
/// Returns paths in arbitrary order.
pub fn scan_source_for_suffixes(
    source_dir: &Path,
    suffixes: &HashSet<u32>,
) -> std::io::Result<Vec<ImageEntry>> {
    let mut out = Vec::new();
    for entry in WalkDir::new(source_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = match path.extension() {
            Some(e) => e,
            None => continue,
        };
        if !is_image_extension(ext) {
            continue;
        }
        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s,
            None => continue,
        };
        if stem_ends_with_suffix(stem, suffixes) {
            out.push(ImageEntry {
                path: path.to_path_buf(),
            });
        }
    }
    Ok(out)
}

/// Recursively list all image files under `dir` (for building destination hash index).
pub fn list_images_under(dir: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut out = Vec::new();
    for entry in WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = match path.extension() {
            Some(e) => e,
            None => continue,
        };
        if is_image_extension(ext) {
            out.push(path.to_path_buf());
        }
    }
    Ok(out)
}
