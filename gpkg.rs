use crate::models::{GpkgDatabase, GpkgEntry, Manifest};
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use tar::{Archive, Builder};
use tempfile::NamedTempFile;
use tokio::process::Command;

const DB_PATH: &str = "/var/lib/gvalli/gpkg.json";

fn get_db_path() -> PathBuf {
    PathBuf::from(DB_PATH)
}

pub fn load_db() -> GpkgDatabase {
    let path = get_db_path();
    if let Ok(data) = fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        GpkgDatabase::default()
    }
}

pub fn save_db(db: &GpkgDatabase) -> Result<()> {
    let path = get_db_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Failed to create db parent directories")?;
    }
    let data = serde_json::to_string_pretty(db)?;
    fs::write(path, data)?;
    Ok(())
}

fn compute_sha256(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn sanitize_extract_path(base: &Path, rel_path: &Path) -> Result<PathBuf> {
    let mut target = base.to_path_buf();
    for comp in rel_path.components() {
        match comp {
            Component::Normal(c) => target.push(c),
            Component::ParentDir => {
                if target != base {
                    target.pop();
                }
            }
            _ => {} // Ignore Prefix, RootDir, CurDir securely
        }
    }
    if !target.starts_with(base) {
        anyhow::bail!("Zip-Slip security alert: invalid trajectory detected {:?}", rel_path);
    }
    Ok(target)
}

fn parse_gpkgm_string(content: &str) -> Result<Manifest> {
    let mut values = HashMap::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            values.insert(key.trim().to_lowercase(), value.trim().to_string());
        }
    }

    let name = values.get("name").context("Missing 'name' field in GPKGM")?.clone();
    let version = values.get("version").context("Missing 'version' field in GPKGM")?.clone();
    
    Ok(Manifest {
        name: name.clone(),
        version,
        description: values.get("description").cloned().unwrap_or_default(),
        maintainer: values.get("maintainer").cloned().unwrap_or_default(),
        maintainer_email: values.get("email").cloned().unwrap_or_default(),
        github_repo: values.get("github").or_else(|| values.get("repository")).cloned().unwrap_or_default(),
        exec_binary: values.get("exec").cloned().unwrap_or(name),
        dependencies: vec![],
    })
}

pub fn is_root() -> bool {
    std::process::Command::new("id").arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
}

/// Automatically escalates privileges for CLI if required.
async fn ensure_root_escalation() -> Result<()> {
    if !is_root() {
        let args: Vec<String> = std::env::args().collect();
        if args.len() <= 1 {
            anyhow::bail!("Root privileges required. (TUI handles this implicitly)");
        }
        
        println!("🔑 Privilege escalation required. Automatically invoking sudo...");
        let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("gvalli"));
        
        let mut child = Command::new("sudo")
            .arg(current_exe)
            .args(&args[1..])
            .spawn()
            .context("Failed to spawn sudo process")?;
            
        let status = child.wait().await?;
        if status.success() {
            std::process::exit(0);
        } else {
            anyhow::bail!("Sudo escalation cancelled or failed.");
        }
    }
    Ok(())
}

pub async fn install_package(target: &str) -> Result<()> {
    ensure_root_escalation().await?;
    println!("🔍 Preparing package installation from: {}", target);

    let mut temp_file = NamedTempFile::new()?;
    if target.starts_with("http://") || target.starts_with("https://") {
        let client = reqwest::Client::builder().user_agent("GValli").build()?;
        let mut response = client.get(target).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("HTTP download failed with status {}", response.status());
        }
        while let Some(chunk) = response.chunk().await? {
            temp_file.write_all(&chunk)?;
        }
    } else {
        let mut src = File::open(target).context(format!("Failed to read archive: {}", target))?;
        io::copy(&mut src, &mut temp_file)?;
    }
    temp_file.flush()?;

    let mut manifest_opt = None;
    let pass1_file = File::open(temp_file.path())?;
    let mut archive = Archive::new(GzDecoder::new(pass1_file));
    for entry in archive.entries()? {
        let mut file = entry?;
        if file.path()?.to_string_lossy() == "GPKGM" {
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            manifest_opt = Some(parse_gpkgm_string(&content)?);
            break;
        }
    }

    let manifest = manifest_opt.context("Invalid .gpkg archive: GPKGM manifest not found")?;
    println!("📦 Verified Manifest: {} v{}", manifest.name, manifest.version);

    let staging_id = uuid::Uuid::new_v4().to_string();
    let staging_dir = PathBuf::from(format!("/tmp/gvalli_staging_{}", staging_id));
    fs::create_dir_all(&staging_dir)?;

    let pass2_file = File::open(temp_file.path())?;
    let mut archive = Archive::new(GzDecoder::new(pass2_file));
    let mut staged_files = Vec::new();

    for entry in archive.entries()? {
        let mut file = entry?;
        let path_str = file.path()?.to_string_lossy().to_string();

        let mut sys_target_opt = None;
        let mut is_exec = false;

        if let Some(sys_path) = path_str.strip_prefix("files/") {
            sys_target_opt = Some(Path::new("/").join(sys_path));
        } else if let Some(sys_path) = path_str.strip_prefix("bin/") {
            sys_target_opt = Some(Path::new("/usr/bin").join(sys_path));
            is_exec = true;
        } else if let Some(sys_path) = path_str.strip_prefix("src/") {
            sys_target_opt = Some(Path::new("/usr/share").join(&manifest.name).join("src").join(sys_path));
        }

        if let Some(sys_path) = sys_target_opt {
            let rel_path = sys_path.strip_prefix("/").unwrap_or(&sys_path);
            let stage_path = sanitize_extract_path(&staging_dir, rel_path)?;

            if file.header().entry_type().is_dir() {
                fs::create_dir_all(&stage_path)?;
                fs::create_dir_all(&sys_path)?;
                continue;
            }

            if let Some(parent) = stage_path.parent() {
                fs::create_dir_all(parent)?;
            }
            file.unpack(&stage_path)?;

            if stage_path.is_dir() {
                fs::create_dir_all(&sys_path)?;
                continue;
            }

            if is_exec {
                if let Ok(meta) = fs::metadata(&stage_path) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o755);
                    let _ = fs::set_permissions(&stage_path, perms);
                }
            }

            staged_files.push((stage_path, sys_path));
        }
    }

    println!("⚡ Performing atomic file commit...");
    let mut installed_files = Vec::new();
    let mut checksums = HashMap::new();
    let mut rollback_history = Vec::new();

    for (stage_path, sys_path) in staged_files {
        if stage_path.is_dir() {
            let _ = fs::create_dir_all(&sys_path);
            continue;
        }

        if let Some(parent) = sys_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("❌ Error creating target directory {:?}: {}. Reverting transaction...", parent, e);
                for p in rollback_history { let _ = fs::remove_file(p); }
                let _ = fs::remove_dir_all(&staging_dir);
                anyhow::bail!("Installation aborted; state successfully rolled back.");
            }
        }
        
        let file_hash = compute_sha256(&stage_path)?;

        let copy_result = if sys_path.exists() {
            let file_name = sys_path.file_name().unwrap_or_default().to_string_lossy();
            let tmp_sys_path = sys_path.with_file_name(format!(".{}.tmp_gvalli_{}", file_name, uuid::Uuid::new_v4().simple()));
            
            match fs::copy(&stage_path, &tmp_sys_path).and_then(|_| fs::rename(&tmp_sys_path, &sys_path)) {
                Ok(_) => Ok(()),
                Err(_) => {
                    let _ = fs::remove_file(&sys_path);
                    fs::copy(&stage_path, &sys_path).map(|_| ())
                }
            }
        } else {
            fs::copy(&stage_path, &sys_path).map(|_| ())
        };

        match copy_result {
            Ok(_) => {
                rollback_history.push(sys_path.clone());
                installed_files.push(sys_path.to_string_lossy().to_string());
                checksums.insert(sys_path.to_string_lossy().to_string(), file_hash);
                
                let perms = fs::metadata(&stage_path)?.permissions();
                let _ = fs::set_permissions(&sys_path, perms);
            }
            Err(e) => {
                eprintln!("❌ Failed writing file {:?}: {}. Reverting transaction...", sys_path, e);
                for p in rollback_history { let _ = fs::remove_file(p); }
                let _ = fs::remove_dir_all(&staging_dir);
                anyhow::bail!("Installation aborted; state successfully rolled back.");
            }
        }
    }

    let _ = fs::remove_dir_all(&staging_dir);

    let mut db = load_db();
    db.packages.insert(manifest.name.clone(), GpkgEntry {
        version: manifest.version.clone(),
        files: installed_files,
        checksums,
        github_repo: if manifest.github_repo.is_empty() { None } else { Some(manifest.github_repo) },
        exec_binary: Some(manifest.exec_binary.clone()),
    });
    save_db(&db)?;

    println!("🎉 Installed {} v{} successfully!", manifest.name, manifest.version);

    // Auto-restart logic
    let exec_bin = manifest.exec_binary;
    let args: Vec<String> = std::env::args().collect();
    let is_batch_update = args.iter().any(|a| a == "update" || a == "u");

    if exec_bin == "gvalli" {
        if !is_batch_update {
            println!("🔄 GValli updated successfully. Hot-reloading process...");
            let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("gvalli"));
            use std::os::unix::process::CommandExt;
            let _ = std::process::Command::new(current_exe).args(&args[1..]).exec();
        }
    } else {
        if let Ok(out) = std::process::Command::new("pidof").arg(&exec_bin).output() {
            if out.status.success() && !out.stdout.is_empty() {
                println!("🔄 Executable '{}' was running. Auto-restarting safely...", exec_bin);
                let _ = std::process::Command::new("killall").arg("-q").arg(&exec_bin).status();
                
                std::thread::sleep(std::time::Duration::from_millis(300));
                
                let launch_cmd = format!("nohup {} > /dev/null 2>&1 &", exec_bin);
                if let Ok(sudo_user) = std::env::var("SUDO_USER") {
                    let _ = std::process::Command::new("su")
                        .args(["-c", &launch_cmd, &sudo_user])
                        .spawn();
                } else {
                    let _ = std::process::Command::new("sh")
                        .args(["-c", &launch_cmd])
                        .spawn();
                }
            }
        }
    }

    Ok(())
}

pub fn extract_package(target: &str, dest_dir: &Path) -> Result<()> {
    println!("🗜 Extracting package archive '{}' to '{:?}'...", target, dest_dir);
    let file = File::open(target).context("Failed to open archive file")?;
    let mut archive = Archive::new(GzDecoder::new(file));

    fs::create_dir_all(dest_dir)?;
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        let sanitized = sanitize_extract_path(dest_dir, &path)?;
        if let Some(parent) = sanitized.parent() {
            fs::create_dir_all(parent)?;
        }
        entry.unpack(&sanitized)?;
    }
    println!("✅ Package extracted successfully.");
    Ok(())
}

pub async fn remove_package(identifier: &str) -> Result<()> {
    ensure_root_escalation().await?;
    let mut db = load_db();
    
    let mut target_name = None;
    for (name, entry) in db.packages.iter() {
        if name == identifier 
            || entry.exec_binary.as_deref() == Some(identifier)
            || entry.files.iter().any(|f| Path::new(f).file_stem().and_then(|s| s.to_str()) == Some(identifier))
        {
            target_name = Some(name.clone());
            break;
        }
    }

    let name = target_name.context(format!("Package or alias '{}' is not installed.", identifier))?;
    let entry = db.packages.remove(&name).context("DB concurrency mismatch.")?;

    println!("🗑 Removing package {} (v{})...", name, entry.version);
    
    let prerm_path = format!("/usr/share/{}/prerm", name);
    if Path::new(&prerm_path).exists() {
        println!("⚡ Executing pre-remove script...");
        let _ = std::process::Command::new("sh").arg("-c").arg(format!("set +e; {}", prerm_path)).status();
    }

    let mut dirs_to_check = HashSet::new();
    for file in &entry.files {
        let p = Path::new(&file);
        if p.exists() && (p.is_file() || p.is_symlink()) {
            let _ = fs::remove_file(p);
        }
        if let Some(parent) = p.parent() {
            dirs_to_check.insert(parent.to_path_buf());
        }
    }

    let share_dir = Path::new("/usr/share").join(&name);
    if share_dir.exists() && share_dir.is_dir() {
        let _ = fs::remove_dir_all(&share_dir);
    }
    
    let postrm_path = format!("/usr/share/{}/postrm", name);
    if Path::new(&postrm_path).exists() {
        println!("⚡ Executing post-remove script...");
        let _ = std::process::Command::new("sh").arg("-c").arg(format!("set +e; {}", postrm_path)).status();
    }

    let mut sorted_dirs: Vec<_> = dirs_to_check.into_iter().collect();
    sorted_dirs.sort_by_key(|a| std::cmp::Reverse(a.components().count()));
    for dir in sorted_dirs {
        if dir.exists() && fs::read_dir(&dir).map(|mut i| i.next().is_none()).unwrap_or(false) {
            let _ = fs::remove_dir(&dir);
        }
    }

    save_db(&db)?;
    println!("✅ Package {} uninstalled completely.", name);
    Ok(())
}

pub fn verify_packages() -> Result<()> {
    let db = load_db();
    let mut all_valid = true;

    if db.packages.is_empty() {
        println!("✅ No GPKG packages currently installed.");
        return Ok(());
    }

    for (name, entry) in db.packages.iter() {
        let mut errors = Vec::new();
        for file in &entry.files {
            let path = Path::new(file);
            if !path.exists() {
                errors.push(format!("File missing: {}", file));
            } else if let Some(expected_hash) = entry.checksums.get(file) {
                if let Ok(actual_hash) = compute_sha256(path) {
                    if actual_hash != *expected_hash {
                        errors.push(format!("SHA256 mismatch: {}", file));
                    }
                } else {
                    errors.push(format!("Read error: {}", file));
                }
            }
        }
        if errors.is_empty() {
            println!("  ✔ {} (v{}) — Integrity intact.", name, entry.version);
        } else {
            all_valid = false;
            println!("  m {} (v{}) — Corruption detected:", name, entry.version);
            for err in errors {
                println!("      - {}", err);
            }
        }
    }

    if all_valid {
        println!("✅ System package verification passed cleanly.");
    } else {
        anyhow::bail!("System verification discovered damaged or missing files.");
    }
    Ok(())
}

pub async fn inspect_package(package_or_file: &str) -> Result<()> {
    if Path::new(package_or_file).exists() {
        println!("🔍 Inspecting local archive: {}", package_or_file);
        let file = File::open(package_or_file)?;
        let mut archive = Archive::new(GzDecoder::new(file));
        for entry in archive.entries()? {
            let mut entry = entry?;
            if entry.path()?.to_string_lossy() == "GPKGM" {
                let mut content = String::new();
                entry.read_to_string(&mut content)?;
                let m = parse_gpkgm_string(&content)?;
                println!("  Name:        {}", m.name);
                println!("  Version:     {}", m.version);
                println!("  Description: {}", m.description);
                println!("  Maintainer:  {}", m.maintainer);
                println!("  Exec Binary: {}", m.exec_binary);
                return Ok(());
            }
        }
        anyhow::bail!("No GPKGM manifest found inside archive.");
    } else {
        let db = load_db();
        if let Some(entry) = db.packages.get(package_or_file) {
            println!("🔍 Inspecting installed package: {}", package_or_file);
            println!("  Version:     {}", entry.version);
            println!("  Total Files: {}", entry.files.len());
            println!("  Exec Binary: {}", entry.exec_binary.as_deref().unwrap_or(package_or_file));
            if let Some(repo) = &entry.github_repo {
                println!("  GitHub Repo: {}", repo);
            }
            return Ok(());
        }
        anyhow::bail!("Target '{}' is neither an existing file nor an installed package.", package_or_file);
    }
}

pub async fn get_package(url: &str) -> Result<()> {
    println!("🌐 Fetching repository from: {}", url);
    let temp_dir = format!("/tmp/gvalli-get-{}", uuid::Uuid::new_v4());
    
    let status = tokio::process::Command::new("git").args(["clone", "--depth=1", url, &temp_dir]).status().await?;
    if !status.success() {
        anyhow::bail!("Git clone operation failed.");
    }

    println!("🚀 Building package archive...");
    if let Ok(gpkg_path) = create_package(&temp_dir, false).await {
        println!("⚡ Installing built archive...");
        install_package(&gpkg_path).await?;
        let _ = fs::remove_file(&gpkg_path);
    }
    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

pub async fn create_package(project_path: &str, require_metadata: bool) -> Result<String> {
    let path = Path::new(project_path);
    let resolved_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    let metadata_path = resolved_path.join("GPKGM");
    if !metadata_path.exists() && require_metadata {
        anyhow::bail!("Required GPKGM file not found in {:?}", resolved_path);
    }

    let manifest = if metadata_path.exists() {
        let content = fs::read_to_string(&metadata_path)?;
        parse_gpkgm_string(&content)?
    } else {
        Manifest {
            name: resolved_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
            version: "1.0.0".to_string(),
            description: "".to_string(),
            maintainer: "".to_string(),
            maintainer_email: "".to_string(),
            github_repo: "".to_string(),
            exec_binary: resolved_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
            dependencies: vec![],
        }
    };

    let cargo_toml = resolved_path.join("Cargo.toml");
    if cargo_toml.exists() {
        println!("🛠 Building Rust binary (cargo build --release)...");
        let status = tokio::process::Command::new("cargo").args(["build", "--release"]).current_dir(&resolved_path).status().await?;
        if !status.success() {
            anyhow::bail!("Cargo build failed.");
        }
    }

    let release_dir = resolved_path.join("target").join("release");
    let binary_path = release_dir.join(&manifest.exec_binary);

    if !binary_path.exists() && cargo_toml.exists() {
        anyhow::bail!("Target binary {:?} was not produced during build.", binary_path);
    }

    let package_name = format!("{}-{}.gpkg", manifest.name, manifest.version);
    let package_path = resolved_path.join(&package_name);
    
    println!("🗜 Creating archive: {}...", package_name);
    let tar_gz = File::create(&package_path)?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut builder = Builder::new(enc);

    if binary_path.exists() {
        let file = File::open(&binary_path)?;
        let mut header = tar::Header::new_gnu();
        header.set_size(file.metadata()?.len());
        header.set_mode(0o755);
        header.set_cksum();
        builder.append_data(&mut header, format!("files/usr/bin/{}", manifest.exec_binary), file)?;
    }

    let gpkgm_content = fs::read_to_string(&metadata_path).unwrap_or_else(|_| {
        format!("name={}\nversion={}\ndescription={}\nexec={}\n", manifest.name, manifest.version, manifest.description, manifest.exec_binary)
    });
    let mut header = tar::Header::new_gnu();
    header.set_size(gpkgm_content.len() as u64);
    header.set_cksum();
    builder.append_data(&mut header, "GPKGM", gpkgm_content.as_bytes())?;

    builder.finish()?;
    println!("✅ .gpkg archive created: {:?}", package_path);
    Ok(package_path.to_string_lossy().to_string())
}