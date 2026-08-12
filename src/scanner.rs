use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::task;
use walkdir::WalkDir;

pub struct Scanner;

impl Scanner {
    pub fn start_scan(tx: mpsc::UnboundedSender<PathBuf>) {
        task::spawn_blocking(move || {
            let walker = WalkDir::new("/").into_iter().filter_entry(|e| {
                let p = e.path();
                // Strictly exclude virtual/pseudofs paths, network mounts, and temp dirs
                !(p.starts_with("/proc") 
               || p.starts_with("/sys") 
               || p.starts_with("/dev")  
               || p.starts_with("/run") 
               || p.starts_with("/tmp")  
               || p.starts_with("/mnt") 
               || p.starts_with("/media")
               || p.starts_with("/var/tmp"))
            });

            for entry in walker.flatten() {
                if entry.file_type().is_file() {
                    if let Some(ext) = entry.path().extension() {
                        if ext == "gpkg" {
                            let _ = tx.send(entry.path().to_path_buf());
                        }
                    }
                }
            }
        });
    }
}