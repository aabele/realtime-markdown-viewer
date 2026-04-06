use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};

pub enum WatchEvent {
    FileChanged(PathBuf),
    Rescan,
}

pub fn discover_md_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_md_files(root, &mut files);
    files.sort();
    files
}

fn collect_md_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_md_files(&path, files);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            files.push(path);
        }
    }
}

pub fn start_watcher(
    root: &Path,
) -> Result<
    (
        mpsc::Receiver<WatchEvent>,
        notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
    ),
    notify::Error,
> {
    let (tx, rx) = mpsc::sync_channel(64);

    let sender = tx.clone();
    let mut debouncer = new_debouncer(
        Duration::from_millis(300),
        move |res: DebounceEventResult| {
            if let Ok(events) = res {
                let mut needs_rescan = false;
                for event in &events {
                    let is_md = event.path.extension().and_then(|e| e.to_str()) == Some("md");
                    if is_md && event.path.exists() {
                        let _ = sender.send(WatchEvent::FileChanged(event.path.clone()));
                    } else {
                        needs_rescan = true;
                    }
                }
                if needs_rescan {
                    let _ = sender.send(WatchEvent::Rescan);
                }
            }
        },
    )
    .map_err(|e| notify::Error::generic(&format!("{}", e)))?;

    debouncer.watcher().watch(root, RecursiveMode::Recursive)?;

    Ok((rx, debouncer))
}
