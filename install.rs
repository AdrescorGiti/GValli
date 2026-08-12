use std::env;
use tokio::process::Command;
use crate::models::PackageSource;

#[derive(Debug, Clone, PartialEq)]
pub enum SourceKind {
    Pacman,
    Flatpak,
    Gos(String),
}

impl SourceKind {
    pub fn label(&self) -> &'static str {
        match self {
            SourceKind::Pacman => "Pacman",
            SourceKind::Flatpak => "Flatpak",
            SourceKind::Gos(_) => "G OS",
        }
    }
}

pub async fn smart_install(package: &str, noconfirm: bool, force_source: Option<PackageSource>) {
    let source = resolve_source(package, force_source).await;
    match &source {
        Some(SourceKind::Gos(url)) => {
            println!("🚀 Установка из G OS repository: {}", package);
            crate::gpkg::install_package(url).await;
        }
        Some(SourceKind::Pacman) => {
            install_pacman(package, noconfirm).await;
        }
        Some(SourceKind::Flatpak) => {
            install_flatpak(package, noconfirm).await;
        }
        None => {
            eprintln!("❌ Пакет '{}' не найден ни в одном источнике.", package);
        }
    }
}

async fn resolve_source(package: &str, force_source: Option<PackageSource>) -> Option<SourceKind> {
    if let Some(src) = force_source {
        return match src {
            PackageSource::Gos => {
                let repo_pkg = crate::search::get_gos_package(package).await?;
                Some(SourceKind::Gos(repo_pkg.url.clone()))
            }
            PackageSource::Pacman => Some(SourceKind::Pacman),
            PackageSource::Flatpak => Some(SourceKind::Flatpak),
        };
    }

    if let Some(repo_pkg) = crate::search::get_gos_package(package).await {
        return Some(SourceKind::Gos(repo_pkg.url.clone()));
    }

    if Command::new("pacman").args(["-Si", package]).output().await.map_or(false, |o| o.status.success()) {
        return Some(SourceKind::Pacman);
    }

    let flat_res = crate::search::search_flatpak(package).await;
    if flat_res.iter().any(|p| p.name.to_lowercase() == package.to_lowercase()) {
        return Some(SourceKind::Flatpak);
    }

    None
}

pub async fn install_gos_package(repo_pkg: &crate::search::GosPackage) {
    println!("🚀 Установка из G OS repository: {} v{}", repo_pkg.name, repo_pkg.version);
    crate::gpkg::install_package(&repo_pkg.url).await;
}

async fn ensure_root() -> bool {
    if env::var("USER").unwrap_or_default() != "root" {
        println!("🔑 Для операции требуются права root. Вызов sudo...");
        let status = Command::new("sudo")
            .args([crate::gpkg::current_gvalli_path().as_str(), "install"])
            .status()
            .await;
        return status.map_or(false, |s| s.success());
    }
    true
}

async fn install_pacman(package: &str, noconfirm: bool) {
    if !ensure_root().await { return; }
    println!("🚀 Установка из Pacman: {}", package);
    let mut args = vec!["-S", package];
    if noconfirm { args.push("--noconfirm"); }
    let _ = Command::new("pacman").args(&args).status().await;
}

async fn install_flatpak(package: &str, noconfirm: bool) {
    println!("🚀 Установка из Flatpak: {}", package);
    let mut args = vec!["install", package];
    if noconfirm { args.push("-y"); }
    let _ = Command::new("flatpak").args(&args).status().await;
}