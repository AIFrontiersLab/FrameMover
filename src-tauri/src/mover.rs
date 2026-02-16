//! Move files with collision handling and cross-volume fallback.

use std::fs;
use std::path::Path;

use crate::hasher;

/// Move `src` to `dest`. If same volume, uses atomic rename; otherwise copy+sync+delete.
/// If `src` hash already exists anywhere in destination (dest_hash_index), skip as duplicate.
/// If `dest` already exists:
/// - If same content (hash), skip (caller should treat as duplicate).
/// - Else rename to dest with "-1", "-2", ... before extension until available.
/// Returns: Ok(()) if moved or skipped-as-duplicate, Err on failure.
pub fn move_file(
    src: &Path,
    dest: &Path,
    dest_hash_index: &std::collections::HashSet<String>,
) -> Result<MoveResult, std::io::Error> {
    let src_hash = hasher::hash_file(src)?;

    if dest_hash_index.contains(&src_hash) {
        return Ok(MoveResult::SkippedDuplicate);
    }

    // If destination path exists, check content
    if dest.exists() {
        if let Ok(existing_hash) = hasher::hash_file(dest) {
            if existing_hash == src_hash {
                return Ok(MoveResult::SkippedDuplicate);
            }
        }
        // Different content: find unique name
        let (stem, ext) = split_stem_ext(dest);
        for i in 1.. {
            let candidate = if ext.is_empty() {
                format!("{}-{}", stem, i)
            } else {
                format!("{}-{}.{}", stem, i, ext)
            };
            let candidate_path = dest.parent().unwrap().join(&candidate);
            if !candidate_path.exists() {
                return do_move(src, &candidate_path).map(|_| MoveResult::Moved(candidate_path));
            }
        }
    }

    // Ensure parent dir exists
    if let Some(p) = dest.parent() {
        fs::create_dir_all(p)?;
    }
    do_move(src, dest).map(|_| MoveResult::Moved(dest.to_path_buf()))
}

fn split_stem_ext(path: &Path) -> (String, String) {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    (stem, ext)
}

fn do_move(src: &Path, dest: &Path) -> Result<(), std::io::Error> {
    // Try atomic rename first (same volume)
    if fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    // Cross-volume: copy then delete
    fs::copy(src, dest)?;
    if let Ok(f) = fs::File::open(dest) {
        f.sync_all().ok();
    }
    fs::remove_file(src)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveResult {
    /// File was moved; path is the actual destination (may be with -1, -2 if collision).
    Moved(std::path::PathBuf),
    SkippedDuplicate,
}

/// Build destination path preserving structure: source_root + rel => dest_root + rel.
pub fn dest_path_for(source_root: &Path, dest_root: &Path, file_path: &Path) -> std::path::PathBuf {
    let rel = file_path.strip_prefix(source_root).unwrap_or(file_path);
    dest_root.join(rel)
}
