//! FrameMover: move image files by filename suffix with deduplication.
//! If CLI args (--source, --dest, --suffixes) are provided, runs headless and exits.

use clap::Parser;
use photo_suffix_mover::engine;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

#[derive(Parser, Debug)]
#[command(name = "FrameMover")]
#[command(about = "Move image files by filename suffix with deduplication")]
struct Cli {
    #[arg(long)]
    source: Option<PathBuf>,
    #[arg(long)]
    dest: Option<PathBuf>,
    #[arg(long)]
    suffixes: Option<String>,
    #[arg(long)]
    dry_run: bool,
    #[arg(long, short = 'v')]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();
    let run_cli = cli.source.is_some() && cli.dest.is_some() && cli.suffixes.is_some();

    if run_cli {
        let source = cli.source.unwrap();
        let dest = cli.dest.unwrap();
        let suffixes = cli.suffixes.unwrap_or_default();
        if !source.is_dir() {
            eprintln!("Error: source is not a directory: {}", source.display());
            std::process::exit(1);
        }
        if dest.exists() && !dest.is_dir() {
            eprintln!("Error: dest exists and is not a directory: {}", dest.display());
            std::process::exit(1);
        }
        let cancel = AtomicBool::new(false);
        let progress: Option<Box<dyn Fn(engine::ProgressEvent) + Send>> = Some(Box::new(|ev: engine::ProgressEvent| {
            let phase = match &ev.phase {
                engine::Phase::ScanningSource => "scanning",
                engine::Phase::IndexingDestination => "indexing",
                engine::Phase::Moving => "moving",
                engine::Phase::Done => "done",
            };
            if let Some(ref f) = ev.current_file {
                let short: String = if f.len() > 60 {
                    format!("...{}", &f[f.len().saturating_sub(57)..])
                } else {
                    f.clone()
                };
                print!("\r[{}] {}% | moved: {} dup: {} err: {} | {}", phase, ev.percent as u32, ev.moved, ev.skipped_duplicates, ev.errors, short);
            } else {
                print!("\r[{}] {}% | moved: {} dup: {} err: {}   ", phase, ev.percent as u32, ev.moved, ev.skipped_duplicates, ev.errors);
            }
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }));
        let result = engine::run(
            &source,
            &dest,
            &suffixes,
            cli.dry_run,
            cli.verbose,
            &cancel,
            progress,
        );
        println!();
        if result.errors > 0 {
            std::process::exit(1);
        }
        std::process::exit(0);
    }

    photo_suffix_mover::run();
}
