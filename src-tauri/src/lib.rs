pub mod engine;
mod hasher;
mod mover;
mod scanner;
mod suffix_parser;

use engine::{run as engine_run, ProgressEvent};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

/// Shared state for cancellation.
struct CancelState {
    cancel: Arc<AtomicBool>,
}

#[tauri::command]
fn start_move(
    app: AppHandle,
    source: String,
    dest: String,
    suffix_input: String,
    dry_run: bool,
    verbose: bool,
) -> Result<(), String> {
    let state = app.state::<CancelState>();
    state.cancel.store(false, std::sync::atomic::Ordering::Relaxed);

    let source_path = PathBuf::from(&source);
    let dest_path = PathBuf::from(&dest);
    if !source_path.is_dir() {
        return Err("Source is not a directory".to_string());
    }
    if dest_path.exists() && !dest_path.is_dir() {
        return Err("Destination exists and is not a directory".to_string());
    }

    let cancel = state.cancel.clone();
    let app_emit = app.clone();
    std::thread::spawn(move || {
        let progress: Option<Box<dyn Fn(ProgressEvent) + Send>> = Some(Box::new(move |ev| {
            let _ = app_emit.emit("progress", &ev);
        }));
        engine_run(
            &source_path,
            &dest_path,
            &suffix_input,
            dry_run,
            verbose,
            &cancel,
            progress,
        );
    });
    Ok(())
}

#[tauri::command]
fn cancel_move(app: AppHandle) -> Result<(), String> {
    let state = app.state::<CancelState>();
    state.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(CancelState {
            cancel: Arc::new(AtomicBool::new(false)),
        })
        .invoke_handler(tauri::generate_handler![start_move, cancel_move])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
