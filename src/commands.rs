use crate::gpkg;
use crate::search;
use anyhow::Result;
use tokio::process::Command;

pub async fn package_info(package: &str) -> Result<()> {
    println!("ℹ️ Package Info Query: '{}'", package);
    let db = gpkg::load_db();
    
    if let Some(entry) = db.packages.get(package) {
        println!("\n[Local GPKG Database]");
        println!("  Package:     {}", package);
        println!("  Version:     {}", entry.version);
        println!("  File Count:  {}", entry.files.len());
        if let Some(repo) = &entry.github_repo { 
            println!("  GitHub Repo: {}", repo); 
        }
        println!("  Installed Files:");
        for file in &entry.files {
            println!("    - {}", file);
        }
        return Ok(());
    }

    if let Some(gos) = search::get_gos_package(package).await {
        println!("\n[G OS Remote Repository]");
        println!("  Name:        {}", gos.name);
        println!("  Version:     {}", gos.version);
        println!("  Description: {}", gos.description);
        println!("  Creator:     {}", gos.creator);
        println!("  URL:         {}", gos.url);
        return Ok(());
    }

    println!("❌ Package '{}' not found in database or remote repository.", package);
    Ok(())
}

pub async fn list_packages() -> Result<()> {
    let db = gpkg::load_db();
    println!("📋 Installed Local GPKG Packages:\n");
    if db.packages.is_empty() {
        println!("  (no packages installed)");
    } else {
        for (name, entry) in db.packages.iter() {
            println!("  • {} v{} ({} files)", name, entry.version, entry.files.len());
        }
    }
    Ok(())
}

pub async fn doctor() -> Result<()> {
    println!("🩺 Running G OS System Diagnostic...\n");
    
    let is_root = gpkg::is_root();
    println!("  [{}] Root Privileges", if is_root { "✔" } else { "✘" });
    
    let db_path = "/var/lib/gvalli/gpkg.json";
    let db_exists = std::path::Path::new(db_path).exists();
    println!("  [{}] GPKG Database File ({})", if db_exists { "✔" } else { "✘" }, db_path);

    let repo_reachable = search::check_gos_reachable().await;
    println!("  [{}] G OS Remote Repository Network Status", if repo_reachable { "✔" } else { "✘" });

    let bin_path = "/usr/bin";
    println!("  [{}] Target Binary Path ({})", if std::path::Path::new(bin_path).exists() { "✔" } else { "✘" }, bin_path);

    println!("\n🩺 Diagnostics complete.");
    Ok(())
}

pub async fn run_gpkg(package: &str) -> Result<()> {
    let db = gpkg::load_db();
    let exec = if let Some(entry) = db.packages.get(package) {
        entry.exec_binary.clone().unwrap_or_else(|| package.to_string())
    } else {
        package.to_string()
    };

    println!("🚀 Launching binary '{}'...", exec);
    let status = Command::new("sh").arg("-c").arg(&exec).status().await?;
    if !status.success() {
        anyhow::bail!("Command terminated with non-zero exit code.");
    }
    Ok(())
}

pub async fn clean_system(deep: bool) -> Result<()> {
    println!("🧹 Cleaning temporary GValli caches...");
    if let Ok(entries) = std::fs::read_dir("/tmp") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("gvalli") {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }

    if deep {
        println!("🧹 Performing deep workspace cleanup...");
        if let Some(home) = std::env::var_os("HOME") {
            let cache = std::path::Path::new(&home).join(".cache").join("gvalli");
            if cache.exists() {
                let _ = std::fs::remove_dir_all(cache);
            }
        }
    }

    println!("✅ Cleanup complete.");
    Ok(())
}

fn compare_versions(v1: &str, v2: &str) -> std::cmp::Ordering {
    let v1_parts: Vec<&str> = v1.split('.').collect();
    let v2_parts: Vec<&str> = v2.split('.').collect();
    for i in 0..std::cmp::max(v1_parts.len(), v2_parts.len()) {
        let val1 = v1_parts.get(i).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        let val2 = v2_parts.get(i).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        match val1.cmp(&val2) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

pub async fn update_system() -> Result<()> {
    println!("🔄 Checking remote G OS repository for system package updates...");
    let db = gpkg::load_db();
    let remote_pkgs = search::load_gos_repo().await;

    let mut updated_count = 0;
    let mut needs_self_restart = false;

    for (name, entry) in db.packages.iter() {
        if let Some(remote) = remote_pkgs.iter().find(|p| p.name == *name) {
            if compare_versions(&remote.version, &entry.version) == std::cmp::Ordering::Greater {
                println!("⚡ Updating {} from v{} -> v{}...", name, entry.version, remote.version);
                if let Err(e) = gpkg::install_package(&remote.url).await {
                    eprintln!("❌ Error updating package {}: {}", name, e);
                } else {
                    updated_count += 1;
                    if name == "gvalli" {
                        needs_self_restart = true;
                    }
                }
            }
        }
    }

    if updated_count == 0 {
        println!("✅ All installed packages are up to date.");
    } else {
        println!("✅ Updated {} package(s).", updated_count);
    }

    if needs_self_restart {
        println!("\n🔄 GValli package manager has been successfully updated!");
        println!("Press [Enter] to seamlessly restart the application and apply changes...");
        let mut buf = String::new();
        let _ = std::io::stdin().read_line(&mut buf);
        
        use std::os::unix::process::CommandExt;
        let current_exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("gvalli"));
        let args: Vec<String> = std::env::args().skip(1).collect();
        
        let err = std::process::Command::new(current_exe).args(args).exec();
        eprintln!("❌ Failed to restart automatically: {}", err);
    }

    Ok(())
}