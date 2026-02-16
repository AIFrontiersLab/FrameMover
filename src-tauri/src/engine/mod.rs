//! Core engine: scan source, index destination, move matching files with progress and cancellation.

use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::hasher;
use crate::mover;
use crate::scanner;
use crate::suffix_parser;

/// Progress phase for UI/CLI.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    ScanningSource,
    IndexingDestination,
    Moving,
    Done,
}

/// Progress event payload for frontend.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub phase: Phase,
    pub current_file: Option<String>,
    pub scanned: u64,
    pub matched: u64,
    pub moved: u64,
    pub skipped_duplicates: u64,
    pub errors: u64,
    pub percent: f64,
}

/// Result of a single run.
#[derive(Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResult {
    pub scanned: u64,
    pub matched: u64,
    pub moved: u64,
    pub skipped_duplicates: u64,
    pub errors: u64,
}

/// Callback for progress (GUI: emit event; CLI: print).
pub type ProgressFn = Box<dyn Fn(ProgressEvent) + Send>;

/// Run the move operation. If `dry_run` is true, no files are moved.
/// `cancel` is checked periodically; when true, the run stops gracefully.
/// `progress` is called with updates; in CLI mode it can print to stdout.
pub fn run(
    source_dir: &Path,
    dest_dir: &Path,
    suffix_input: &str,
    dry_run: bool,
    verbose: bool,
    cancel: &AtomicBool,
    progress: Option<ProgressFn>,
) -> RunResult {
    let suffixes = suffix_parser::parse_suffixes(suffix_input);
    if suffixes.is_empty() {
        let ev = ProgressEvent {
            phase: Phase::Done,
            current_file: None,
            scanned: 0,
            matched: 0,
            moved: 0,
            skipped_duplicates: 0,
            errors: 1,
            percent: 100.0,
        };
        if let Some(ref p) = progress {
            p(ev);
        }
        return RunResult {
            errors: 1,
            ..Default::default()
        };
    }

    let emit = |ev: ProgressEvent| {
        if let Some(ref p) = progress {
            p(ev);
        }
    };

    // Ensure destination exists
    if let Err(e) = std::fs::create_dir_all(dest_dir) {
        emit(ProgressEvent {
            phase: Phase::Done,
            current_file: None,
            scanned: 0,
            matched: 0,
            moved: 0,
            skipped_duplicates: 0,
            errors: 1,
            percent: 100.0,
        });
        if verbose {
            eprintln!("Destination create error: {}", e);
        }
        return RunResult {
            errors: 1,
            ..Default::default()
        };
    }

    // Phase 1: scan source for matching files
    emit(ProgressEvent {
        phase: Phase::ScanningSource,
        current_file: None,
        scanned: 0,
        matched: 0,
        moved: 0,
        skipped_duplicates: 0,
        errors: 0,
        percent: 0.0,
    });

    let candidates = match scanner::scan_source_for_suffixes(source_dir, &suffixes) {
        Ok(c) => c,
        Err(e) => {
            emit(ProgressEvent {
                phase: Phase::Done,
                current_file: None,
                scanned: 0,
                matched: 0,
                moved: 0,
                skipped_duplicates: 0,
                errors: 1,
                percent: 100.0,
            });
            if verbose {
                eprintln!("Scan error: {}", e);
            }
            return RunResult {
                errors: 1,
                ..Default::default()
            };
        }
    };

    let matched_count = candidates.len() as u64;
    emit(ProgressEvent {
        phase: Phase::IndexingDestination,
        current_file: None,
        scanned: matched_count,
        matched: matched_count,
        moved: 0,
        skipped_duplicates: 0,
        errors: 0,
        percent: 5.0,
    });

    if cancel.load(Ordering::Relaxed) {
        emit(ProgressEvent {
            phase: Phase::Done,
            current_file: None,
            scanned: matched_count,
            matched: matched_count,
            moved: 0,
            skipped_duplicates: 0,
            errors: 0,
            percent: 100.0,
        });
        return RunResult {
            scanned: matched_count,
            matched: matched_count,
            ..Default::default()
        };
    }

    // Phase 2: build destination hash index (only image files under dest)
    let dest_files = match scanner::list_images_under(dest_dir) {
        Ok(f) => f,
        Err(e) => {
            if verbose {
                eprintln!("Destination list error: {}", e);
            }
            vec![]
        }
    };

    let mut dest_hash_index = HashSet::new();
    for (i, path) in dest_files.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        if (i % 50 == 0 || i == dest_files.len() - 1) && i < dest_files.len() {
            let pct = 5.0 + (i as f64 / dest_files.len().max(1) as f64) * 15.0;
            emit(ProgressEvent {
                phase: Phase::IndexingDestination,
                current_file: Some(path.display().to_string()),
                scanned: matched_count,
                matched: matched_count,
                moved: 0,
                skipped_duplicates: 0,
                errors: 0,
                percent: pct,
            });
        }
        if let Ok(h) = hasher::hash_file(path) {
            dest_hash_index.insert(h);
        }
    }

    emit(ProgressEvent {
        phase: Phase::Moving,
        current_file: None,
        scanned: matched_count,
        matched: matched_count,
        moved: 0,
        skipped_duplicates: 0,
        errors: 0,
        percent: 20.0,
    });

    let total = candidates.len().max(1);
    let mut moved = 0u64;
    let mut skipped_duplicates = 0u64;
    let mut errors = 0u64;

    for (i, entry) in candidates.into_iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }

        let src = &entry.path;
        let dest = mover::dest_path_for(source_dir, dest_dir, src);

        let percent = 20.0 + (i as f64 / total as f64) * 80.0;
        emit(ProgressEvent {
            phase: Phase::Moving,
            current_file: Some(src.display().to_string()),
            scanned: matched_count,
            matched: matched_count,
            moved,
            skipped_duplicates,
            errors,
            percent,
        });

        if dry_run {
            if dest_hash_index.contains(&match hasher::hash_file(src) {
                Ok(h) => h,
                Err(_) => {
                    errors += 1;
                    continue;
                }
            }) {
                skipped_duplicates += 1;
            } else {
                moved += 1;
            }
            if verbose {
                println!("[dry-run] would move {} -> {}", src.display(), dest.display());
            }
            continue;
        }

        match mover::move_file(src, &dest, &dest_hash_index) {
            Ok(mover::MoveResult::Moved(actual_dest)) => {
                moved += 1;
                let new_hash = hasher::hash_file(&actual_dest).ok();
                if let Some(h) = new_hash {
                    dest_hash_index.insert(h);
                }
            }
            Ok(mover::MoveResult::SkippedDuplicate) => {
                skipped_duplicates += 1;
            }
            Err(e) => {
                errors += 1;
                if verbose {
                    eprintln!("Move error {} -> {}: {}", src.display(), dest.display(), e);
                }
            }
        }
    }

    emit(ProgressEvent {
        phase: Phase::Done,
        current_file: None,
        scanned: matched_count,
        matched: matched_count,
        moved,
        skipped_duplicates,
        errors,
        percent: 100.0,
    });

    RunResult {
        scanned: matched_count,
        matched: matched_count,
        moved,
        skipped_duplicates,
        errors,
    }
}
