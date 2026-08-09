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
    pub exec_binary: String,
    pub dependencies: Vec<String>,
    pub installed_files: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct CargoToml { package: CargoPackage }

#[derive(Deserialize, Debug)]
struct CargoPackage { 
    name: String, 
    version: String, 
    description: Option<String>, 
    authors: Option<Vec<String>> 
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct GpkgDatabase {
    packages: HashMap<String, GpkgEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GpkgEntry {
    version: String,
    files: Vec<String>,
}

// ==========================================
// УПРАВЛЕНИЕ БАЗОЙ ДАННЫХ .GPKG
// ==========================================

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

pub fn is_gpkg_installed(name: &str) -> bool {
    load_db().packages.contains_key(name)
}

pub async fn remove_gpkg(name: &str) {
    if std::env::var("USER").unwrap_or_default() != "root" {
        println!("🔑 Для удаления пакета Gpkg требуются права root. Вызов sudo...");
        let _ = Command::new("sudo").args(["gvalli", "remove", name]).status().await;
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
        
        // Опционально: Пытаемся удалить пустые директории, которые могли остаться (например /usr/share/pkgname)
        let share_dir = Path::new("/usr/share").join(name);
        if share_dir.exists() && share_dir.is_dir() {
            let _ = fs::remove_dir_all(&share_dir);
            println!("  - Удалена директория ассетов: {:?}", share_dir);
        }

        save_db(&db);
        println!("✅ Пакет {} полностью удален из системы.", name);
    }
}

// ==========================================
// ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ ДЛЯ СБОРКИ АРХИВА
// ==========================================

fn append_file_to_tar(
    builder: &mut Builder<GzEncoder<File>>,
    src_path: &Path,
    dest_path: &str, // Абсолютный путь в системе, например "/usr/bin/app"
    mode: u32,
    installed_files: &mut Vec<String>,
) -> io::Result<()> {
    let mut file = File::open(src_path)?;
    let mut header = tar::Header::new_gnu();
    header.set_size(file.metadata()?.len());
    header.set_mode(mode);
    header.set_cksum();
    
    // Внутри архива все системные файлы лежат в папке "files/"
    let tar_path = format!("files{}", dest_path); 
    builder.append_data(&mut header, tar_path, file)?;
    
    // Сохраняем абсолютный системный путь для манифеста
    installed_files.push(dest_path.to_string());
    Ok(())
}

fn add_directory_recursive(
    builder: &mut Builder<GzEncoder<File>>,
    src_dir: &Path,
    base_dest_dir: &str, // Абсолютный путь, например "/usr/share/app"
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

// ==========================================
// ОСНОВНЫЕ КОМАНДЫ GPKG
// ==========================================

pub async fn create_package(project_path: &str) -> Option<String> {
    let path = Path::new(project_path);
    let cargo_toml_path = path.join("Cargo.toml");

    if !cargo_toml_path.exists() {
        eprintln!("❌ Ошибка: Cargo.toml не найден в {:?}", cargo_toml_path);
        return None;
    }

    println!("📦 Чтение метаданных из Cargo.toml...");
    let cargo_parsed: CargoToml = toml::from_str(&fs::read_to_string(&cargo_toml_path).unwrap()).unwrap();
    let pkg = cargo_parsed.package;

    let mut manifest = Manifest {
        name: pkg.name.clone(),
        version: pkg.version.clone(),
        description: pkg.description.unwrap_or_else(|| "Нет описания".to_string()),
        maintainer: pkg.authors.and_then(|a| a.first().cloned()).unwrap_or_else(|| "Unknown".to_string()),
        exec_binary: pkg.name.clone(),
        dependencies: vec![],
        installed_files: vec![],
    };

    println!("🛠 Компиляция проекта (cargo build --release)...");
    let status = Command::new("cargo").args(["build", "--release"]).current_dir(path).status().await.unwrap();
    if !status.success() {
        eprintln!("❌ Ошибка компиляции проекта.");
        return None;
    }

    let binary_path = path.join("target").join("release").join(&manifest.exec_binary);
    if !binary_path.exists() {
        eprintln!("❌ Бинарный файл не найден: {:?}", binary_path);
        return None;
    }

    let current_dir = std::env::current_dir().unwrap();
    let package_name = format!("{}-{}.gpkg", manifest.name, manifest.version);
    let package_path = current_dir.join(&package_name);
    
    println!("🗜 Упаковка файлов и ассетов в архив {}...", package_name);

    let tar_gz = File::create(&package_path).unwrap();
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut builder = Builder::new(enc);
    let mut installed_files = Vec::new();

    // 1. Упаковка бинарного файла (Права: 0755 - rwxr-xr-x)
    let dest_bin = format!("/usr/bin/{}", manifest.exec_binary);
    append_file_to_tar(&mut builder, &binary_path, &dest_bin, 0o755, &mut installed_files).unwrap();

    // 2. Упаковка .desktop лаунчеров (Ищем в корне и в assets/)
    let mut desktop_files = scan_for_files_with_ext(path, &["desktop"]);
    let assets_dir = path.join("assets");
    if assets_dir.exists() {
        desktop_files.extend(scan_for_files_with_ext(&assets_dir, &["desktop"]));
    }
    for d_file in desktop_files {
        let file_name = d_file.file_name().unwrap().to_string_lossy();
        let dest_desktop = format!("/usr/share/applications/{}", file_name);
        append_file_to_tar(&mut builder, &d_file, &dest_desktop, 0o644, &mut installed_files).unwrap();
    }

    // 3. Упаковка Иконок (Ищем в корне проекта)
    let icon_files = scan_for_files_with_ext(path, &["png", "svg"]);
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

    // 4. Упаковка дополнительных ассетов (Рекурсивно папки assets/ или share/)
    let share_dest = format!("/usr/share/{}", manifest.name);
    if assets_dir.exists() && assets_dir.is_dir() {
        add_directory_recursive(&mut builder, &assets_dir, &share_dest, &mut installed_files).unwrap();
    }
    let share_dir = path.join("share");
    if share_dir.exists() && share_dir.is_dir() {
        add_directory_recursive(&mut builder, &share_dir, &share_dest, &mut installed_files).unwrap();
    }

    // 5. Формирование и добавление manifest.json
    manifest.installed_files = installed_files;
    let manifest_json = serde_json::to_string_pretty(&manifest).unwrap();
    let mut header = tar::Header::new_gnu();
    header.set_size(manifest_json.len() as u64);
    header.set_cksum();
    builder.append_data(&mut header, "manifest.json", manifest_json.as_bytes()).unwrap();

    builder.finish().unwrap();

    println!("✅ Пакет успешно собран: {:?}", package_path);
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
        let _ = Command::new("sudo").args(["gvalli", "gpkg", "install", &abs_target]).status().await;
        return;
    }

    println!("🔍 Подготовка к установке: {}", target);
    let mut temp_file = NamedTempFile::new().unwrap();
    
    if target.starts_with("http://") || target.starts_with("https://") {
        let response = reqwest::get(target).await.unwrap();
        temp_file.write_all(&response.bytes().await.unwrap()).unwrap();
    } else {
        io::copy(&mut File::open(target).unwrap(), &mut temp_file).unwrap();
    }

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

    for file in archive.entries().unwrap() {
        let mut file = file.unwrap();
        let path_str = file.path().unwrap().to_string_lossy().to_string();

        if path_str.starts_with("files/") {
            let sys_path = path_str.strip_prefix("files/").unwrap();
            let target_path = Path::new("/").join(sys_path);
            
            if let Some(parent) = target_path.parent() { fs::create_dir_all(parent).unwrap(); }
            
            // Распаковка с сохранением оригинальных прав (0755 для бинарников)
            file.unpack(&target_path).unwrap();
            println!("  -> Распакован: {:?}", target_path);
        }
    }

    // Сохраняем в реестр точный список файлов, сгенерированный во время `create`
    let mut db = load_db();
    db.packages.insert(manifest.name.clone(), GpkgEntry {
        version: manifest.version,
        files: manifest.installed_files,
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
    if let Some(gpkg_path) = create_package(&temp_dir).await {
        println!("⚡ Установка только что собранного пакета...");
        install_package(&gpkg_path).await;
        
        let _ = fs::remove_file(&gpkg_path);
    }
    
    let _ = fs::remove_dir_all(&temp_dir);
}