use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;
use tar::{Builder, Archive};
use tempfile::NamedTempFile;

const DB_PATH: &str = "/var/lib/gvalli/gpkg.json";

#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub maintainer: String,
    #[serde(default)]
    pub maintainer_email: String,
    #[serde(default)]
    pub github_repo: String,
    pub exec_binary: String,
    pub dependencies: Vec<String>,
    pub installed_files: Vec<String>,
}

#[derive(Deserialize, Debug, Default)]
struct CargoToml {
    #[serde(default)]
    package: CargoPackage,
    #[serde(default, rename = "bin")]
    bins: Vec<CargoBin>,
}

#[derive(Deserialize, Debug, Default, Clone)]
struct CargoPackage {
    #[serde(default)]
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    authors: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Default)]
struct CargoBin {
    #[serde(default)]
    name: String,
    #[serde(default)]
    path: String,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct GpkgDatabase {
    packages: HashMap<String, GpkgEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GpkgEntry {
    pub version: String,
    pub files: Vec<String>,
    #[serde(default)]
    pub github_repo: Option<String>,
    #[serde(default)]
    pub exec_binary: Option<String>,
}

fn load_db() -> GpkgDatabase {
    if let Ok(data) = fs::read_to_string(DB_PATH) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        GpkgDatabase::default()
    }
}

fn save_db(db: &GpkgDatabase) {
    if let Some(parent) = Path::new(DB_PATH).parent() {
        fs::create_dir_all(parent).unwrap_or_default();
    }
    let data = serde_json::to_string_pretty(db).unwrap();
    fs::write(DB_PATH, data).expect("❌ Не удалось сохранить базу данных gpkg");
}

pub fn current_gvalli_path() -> String {
    std::env::current_exe()
        .ok()
        .filter(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "gvalli".to_string())
}

pub fn get_gpkg_info(name: &str) -> Option<(GpkgEntry, usize)> {
    let db = load_db();
    db.packages.get(name).map(|e| (e.clone(), e.files.len()))
}

pub fn list_gpkg_detailed() -> Vec<(String, String, usize)> {
    let db = load_db();
    let mut result: Vec<(String, String, usize)> = db
        .packages
        .iter()
        .map(|(k, e)| (k.clone(), e.version.clone(), e.files.len()))
        .collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

pub async fn update_all_gpkg() -> Vec<(String, String, String)> {
    let db = load_db();
    let mut updated = Vec::new();

    for (name, entry) in db.packages.iter() {
        let Some(repo) = entry.github_repo.clone() else {
            println!("   • {}: нет github-репозитория — пропускаем", name);
            continue;
        };

        println!("🔁 Обновление {} из {}", name, repo);
        let temp_dir = format!("/tmp/gvalli-update-{}", name);
        let _ = fs::remove_dir_all(&temp_dir);

        let status = Command::new("git")
            .args(["clone", "--depth=1", &repo, &temp_dir])
            .status()
            .await;
        if !status.map_or(false, |s| s.success()) {
            eprintln!("   ⚠ Не удалось клонировать репозиторий {} для {}", repo, name);
            continue;
        }

        let new_version = load_manifest_from_directory(Path::new(&temp_dir))
            .map(|m| m.version)
            .unwrap_or_default();

        if !new_version.is_empty() && new_version == entry.version {
            println!("   • {} уже актуален (v{})", name, new_version);
            let _ = fs::remove_dir_all(&temp_dir);
            continue;
        }

        if let Some(gpkg_path) = create_package(&temp_dir, false, false).await {
            println!("⚡ Установка обновления {}...", name);
            install_package(&gpkg_path).await;
            updated.push((name.clone(), entry.version.clone(), new_version));
            let _ = fs::remove_file(&gpkg_path);
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    updated
}

pub fn is_gpkg_installed(name: &str) -> bool {
    load_db().packages.contains_key(name)
}

pub fn list_gpkg_packages() -> Vec<String> {
    let db = load_db();
    let mut names: Vec<String> = db.packages.keys().cloned().collect();
    names.sort();
    names
}

pub fn find_gpkg_packages(query: &str) -> Vec<(String, bool)> {
    let db = load_db();
    let q = query.to_lowercase();
    let mut results = Vec::new();
    for name in db.packages.keys() {
        let nl = name.to_lowercase();
        if nl == q {
            results.push((name.clone(), true));
        } else if nl.contains(&q) {
            results.push((name.clone(), false));
        }
    }
    results
}

pub async fn remove_gpkg(name: &str) {
    if std::env::var("USER").unwrap_or_default() != "root" {
        println!("🔑 Для удаления пакета Gpkg требуются права root. Вызов sudo...");
        let _ = Command::new("sudo")
            .args([current_gvalli_path().as_str(), "remove", name])
            .status()
            .await;
        return;
    }

    let mut db = load_db();
    if let Some(entry) = db.packages.remove(name) {
        println!("🗑 Удаление файлов пакета {} (v{})...", name, entry.version);
        for file in entry.files {
            let p = Path::new(&file);
            if p.exists() && p.is_file() {
                fs::remove_file(p).unwrap_or_default();
                println!("  - Удален: {}", file);
            }
        }
        
        let share_dir = Path::new("/usr/share").join(name);
        if share_dir.exists() && share_dir.is_dir() {
            let _ = fs::remove_dir_all(&share_dir);
            println!("  - Удалена директория ассетов: {:?}", share_dir);
        }

        save_db(&db);
        println!("✅ Пакет {} полностью удален из системы.", name);
    }
}

fn append_file_to_tar(
    builder: &mut Builder<GzEncoder<File>>,
    src_path: &Path,
    dest_path: &str,
    mode: u32,
    installed_files: &mut Vec<String>,
) -> io::Result<()> {
    let file = File::open(src_path)?;
    let mut header = tar::Header::new_gnu();
    header.set_size(file.metadata()?.len());
    header.set_mode(mode);
    header.set_cksum();
    
    let tar_path = format!("files{}", dest_path); 
    builder.append_data(&mut header, tar_path, file)?;
    
    installed_files.push(dest_path.to_string());
    Ok(())
}

fn add_directory_recursive(
    builder: &mut Builder<GzEncoder<File>>,
    src_dir: &Path,
    base_dest_dir: &str,
    installed_files: &mut Vec<String>,
) -> io::Result<()> {
    if src_dir.is_dir() {
        for entry in fs::read_dir(src_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name().unwrap().to_string_lossy();
                let new_dest = format!("{}/{}", base_dest_dir, dir_name);
                add_directory_recursive(builder, &path, &new_dest, installed_files)?;
            } else {
                let file_name = path.file_name().unwrap().to_string_lossy();
                let dest_path = format!("{}/{}", base_dest_dir, file_name);
                append_file_to_tar(builder, &path, &dest_path, 0o644, installed_files)?;
            }
        }
    }
    Ok(())
}

fn scan_for_files_with_ext(dir: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions.contains(&ext) {
                        results.push(path);
                    }
                }
            }
        }
    }
    results
}

fn load_manifest_from_directory(project_path: &Path) -> Result<Manifest, String> {
    let metadata_path = project_path.join("GPKGM");
    if !metadata_path.exists() {
        return Err(format!("❌ Обязательный файл GPKGM не найден: {:?}", metadata_path));
    }

    let content = fs::read_to_string(&metadata_path)
        .map_err(|e| format!("❌ Не удалось прочитать GPKGM: {e}"))?;

    let mut values = HashMap::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        values.insert(key.trim().to_lowercase(), value.trim().to_string());
    }

    let name = values.get("name").cloned().ok_or_else(|| "❌ Отсутствует поле name в GPKGM".to_string())?;
    let version = values.get("version").cloned().ok_or_else(|| "❌ Отсутствует поле version в GPKGM".to_string())?;
    let description = values.get("description").cloned().unwrap_or_else(|| "Нет описания".to_string());
    let maintainer = values.get("maintainer").cloned().unwrap_or_else(|| "Unknown".to_string());
    let maintainer_email = values.get("email").cloned().unwrap_or_default();
    let github_repo = values.get("github").or_else(|| values.get("repository")).cloned().unwrap_or_default();
    let exec_binary = values.get("exec").or_else(|| values.get("terminal_name")).cloned().unwrap_or_else(|| name.clone());

    Ok(Manifest {
        name: name.clone(),
        version: version.clone(),
        description,
        maintainer,
        maintainer_email,
        github_repo,
        exec_binary,
        dependencies: vec![],
        installed_files: vec![],
    })
}

pub async fn create_package(project_path: &str, require_metadata: bool, install_after: bool) -> Option<String> {
    let path = Path::new(project_path);
    let resolved_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    let cargo_toml_path = resolved_path.join("Cargo.toml");

    if !cargo_toml_path.exists() {
        eprintln!("❌ Ошибка: Cargo.toml не найден в {:?}", resolved_path);
        return None;
    }

    let manifest = match load_manifest_from_directory(&resolved_path) {
        Ok(m) => m,
        Err(err) if require_metadata => {
            eprintln!("{}", err);
            return None;
        }
        Err(err) => {
            eprintln!("{}", err);
            return None;
        }
    };

    println!("📦 Чтение метаданных из GPKGM...");
    println!("   • name: {}", manifest.name);
    println!("   • version: {}", manifest.version);
    println!("   • exec: {}", manifest.exec_binary);

    println!("🛠 Компиляция проекта (cargo build --release)...");
    let status = Command::new("cargo").args(["build", "--release"]).current_dir(&resolved_path).status().await.unwrap();
    if !status.success() {
        eprintln!("❌ Ошибка компиляции проекта.");
        return None;
    }

    let cargo_toml: CargoToml = fs::read_to_string(&cargo_toml_path)
        .ok()
        .and_then(|c| toml::from_str(&c).ok())
        .unwrap_or_default();

    let release_dir = resolved_path.join("target").join("release");

    let mut candidates: Vec<PathBuf> = vec![release_dir.join(&manifest.exec_binary)];
    for bin in &cargo_toml.bins {
        if !bin.name.is_empty() {
            candidates.push(release_dir.join(&bin.name));
        }
    }
    if !cargo_toml.package.name.is_empty() {
        candidates.push(release_dir.join(&cargo_toml.package.name));
    }
    candidates.push(release_dir.join(&manifest.name));

    let binary_path = candidates
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| release_dir.join(&manifest.exec_binary));

    if !binary_path.exists() {
        eprintln!("❌ Бинарный файл не найден: {:?}", binary_path);
        eprintln!("   Проверенные кандидаты: exec='{}', cargo package='{}', cargo bins={:?}, gpkgm name='{}'",
            manifest.exec_binary,
            cargo_toml.package.name,
            cargo_toml.bins.iter().map(|b| b.name.clone()).collect::<Vec<_>>(),
            manifest.name);
        return None;
    }

    let package_name = format!("{}-{}.gpkg", manifest.name, manifest.version);
    let package_path = resolved_path.join(&package_name);
    
    println!("🗜 Упаковка файлов и ассетов в архив {}...", package_name);

    let tar_gz = File::create(&package_path).unwrap();
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut builder = Builder::new(enc);
    let mut installed_files = Vec::new();

    let dest_bin = format!("/usr/bin/{}", manifest.exec_binary);
    append_file_to_tar(&mut builder, &binary_path, &dest_bin, 0o755, &mut installed_files).unwrap();

    let mut desktop_files = scan_for_files_with_ext(&resolved_path, &["desktop"]);
    let assets_dir = resolved_path.join("assets");
    if assets_dir.exists() {
        desktop_files.extend(scan_for_files_with_ext(&assets_dir, &["desktop"]));
    }
    for d_file in desktop_files {
        let file_name = d_file.file_name().unwrap().to_string_lossy();
        let dest_desktop = format!("/usr/share/applications/{}", file_name);
        append_file_to_tar(&mut builder, &d_file, &dest_desktop, 0o644, &mut installed_files).unwrap();
    }

    let icon_files = scan_for_files_with_ext(&resolved_path, &["png", "svg"]);
    for i_file in icon_files {
        let file_name = i_file.file_name().unwrap().to_string_lossy();
        let ext = i_file.extension().unwrap().to_str().unwrap();
        
        if ext == "png" {
            let dest_icon = format!("/usr/share/pixmaps/{}", file_name);
            append_file_to_tar(&mut builder, &i_file, &dest_icon, 0o644, &mut installed_files).unwrap();
        } else if ext == "svg" {
            let dest_icon = format!("/usr/share/icons/hicolor/scalable/apps/{}", file_name);
            append_file_to_tar(&mut builder, &i_file, &dest_icon, 0o644, &mut installed_files).unwrap();
        }
    }

    let share_dest = format!("/usr/share/{}", manifest.name);
    if assets_dir.exists() && assets_dir.is_dir() {
        add_directory_recursive(&mut builder, &assets_dir, &share_dest, &mut installed_files).unwrap();
    }
    let share_dir = resolved_path.join("share");
    if share_dir.exists() && share_dir.is_dir() {
        add_directory_recursive(&mut builder, &share_dir, &share_dest, &mut installed_files).unwrap();
    }

    let mut manifest_for_archive = manifest;
    manifest_for_archive.installed_files = installed_files;
    let manifest_json = serde_json::to_string_pretty(&manifest_for_archive).unwrap();
    let mut header = tar::Header::new_gnu();
    header.set_size(manifest_json.len() as u64);
    header.set_cksum();
    builder.append_data(&mut header, "manifest.json", manifest_json.as_bytes()).unwrap();

    builder.finish().unwrap();

    println!("✅ Пакет успешно собран: {:?}", package_path);

    if install_after {
        println!("📦 Установка собранного пакета...");
        install_package(&package_path.to_string_lossy()).await;
    }

    Some(package_path.to_string_lossy().to_string())
}

pub async fn install_package(target: &str) {
    if std::env::var("USER").unwrap_or_default() != "root" {
        let abs_target = if target.starts_with("http") {
            target.to_string()
        } else {
            fs::canonicalize(target).unwrap_or_else(|_| PathBuf::from(target)).to_string_lossy().to_string()
        };
        
        println!("🔑 Требуются права root. Вызов sudo...");
        let _ = Command::new("sudo")
            .args([current_gvalli_path().as_str(), "gpkg", "install", &abs_target])
            .status()
            .await;
        return;
    }

    println!("🔍 Подготовка к установке: {}", target);
    let mut temp_file = match NamedTempFile::new() {
        Ok(f) => f,
        Err(e) => { eprintln!("❌ Не удалось создать временный файл: {}", e); return; }
    };
    
    if target.starts_with("http://") || target.starts_with("https://") {
        let client = reqwest::Client::builder().user_agent("GValli").build().unwrap();
        let mut response = match client.get(target).send().await {
            Ok(r) => r,
            Err(e) => { eprintln!("❌ Ошибка загрузки: {}", e); return; }
        };

        if !response.status().is_success() {
            eprintln!("❌ Ошибка: Сервер вернул статус {}", response.status());
            return;
        }

        let total_size = response.content_length().unwrap_or(0);
        println!("⬇️ Загрузка пакета... (Ожидается байт: {})", total_size);

        while let Some(chunk) = match response.chunk().await {
            Ok(c) => c,
            Err(e) => { eprintln!("❌ Ошибка при чтении потока данных: {}", e); return; }
        } {
            if let Err(e) = temp_file.write_all(&chunk) {
                eprintln!("❌ Ошибка записи во временный файл: {}", e); return;
            }
        }
    } else {
        let mut src = match File::open(target) {
            Ok(f) => f,
            Err(e) => { eprintln!("❌ Не удалось открыть файл '{}': {}", target, e); return; }
        };
        if let Err(e) = io::copy(&mut src, &mut temp_file) {
            eprintln!("❌ Ошибка копирования: {}", e); return;
        }
    }
    temp_file.flush().unwrap_or_default();

    let mut archive = Archive::new(GzDecoder::new(File::open(temp_file.path()).unwrap()));
    let mut manifest_opt = None;

    for file in archive.entries().unwrap() {
        let mut file = file.unwrap();
        if file.path().unwrap().to_string_lossy() == "manifest.json" {
            manifest_opt = Some(serde_json::from_reader::<_, Manifest>(&mut file).unwrap());
            break;
        }
    }

    let manifest = match manifest_opt {
        Some(m) => m,
        None => { eprintln!("❌ Ошибка: В архиве нет manifest.json"); return; }
    };

    println!("✅ Установка пакета: {} v{}", manifest.name, manifest.version);

    let mut archive = Archive::new(GzDecoder::new(File::open(temp_file.path()).unwrap()));

    for entry in archive.entries().unwrap() {
        let mut file = match entry {
            Ok(f) => f,
            Err(e) => { eprintln!("⚠ Пропуск поврежденной записи архива: {}", e); continue; }
        };
        let path_str = file.path().unwrap().to_string_lossy().to_string();

        if path_str.starts_with("files/") {
            let sys_path = path_str.strip_prefix("files/").unwrap();
            let target_path = Path::new("/").join(sys_path);
            
            if let Some(parent) = target_path.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    eprintln!("❌ Не удалось создать директорию {:?}: {}", parent, e);
                    continue;
                }
            }
            
            match file.unpack(&target_path) {
                Ok(_) => println!("  -> Распакован: {:?}", target_path),
                Err(e) => eprintln!("❌ Ошибка распаковки {:?}: {}", target_path, e),
            }
        }
    }

    let mut db = load_db();
    db.packages.insert(manifest.name.clone(), GpkgEntry {
        version: manifest.version,
        files: manifest.installed_files,
        github_repo: if manifest.github_repo.is_empty() {
            None
        } else {
            Some(manifest.github_repo)
        },
        exec_binary: Some(manifest.exec_binary),
    });
    save_db(&db);

    println!("🎉 Пакет {} полностью интегрирован в систему!", manifest.name);
}

pub async fn get_package(url: &str) {
    println!("🌐 Скачивание исходного кода с GitHub: {}", url);
    let temp_dir = format!("/tmp/gvalli-get-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    
    let status = Command::new("git").args(["clone", "--depth=1", url, &temp_dir]).status().await.unwrap();
    if !status.success() {
        eprintln!("❌ Ошибка при клонировании репозитория.");
        return;
    }

    println!("🚀 Сборка пакета из исходников...");
    if let Some(gpkg_path) = create_package(&temp_dir, false, false).await {
        println!("⚡ Установка только что собранного пакета...");
        install_package(&gpkg_path).await;
        
        let _ = fs::remove_file(&gpkg_path);
    }
    
    let _ = fs::remove_dir_all(&temp_dir);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parses_gpkg_manifest_from_metadata_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let metadata_path = temp_dir.path().join("GPKGM");
        fs::write(&metadata_path, "name=demo\nversion=1.0.0\ndescription=Demo package\nexec=demo\nmaintainer=Test User\nemail=test@example.com\ngithub=https://github.com/example/demo\n").unwrap();

        let manifest = load_manifest_from_directory(temp_dir.path()).unwrap();
        assert_eq!(manifest.name, "demo");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.description, "Demo package");
        assert_eq!(manifest.exec_binary, "demo");
        assert_eq!(manifest.maintainer, "Test User");
        assert_eq!(manifest.maintainer_email, "test@example.com");
        assert_eq!(manifest.github_repo, "https://github.com/example/demo");
    }
}