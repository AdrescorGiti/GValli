use tokio::process::Command;
use std::io::{self, Write};

pub async fn remove_package(package: &str, noconfirm: bool) {
    println!("🔍 Поиск '{}' среди установленных...", package);

    if crate::gpkg::is_gpkg_installed(package) {
        crate::gpkg::remove_gpkg(package).await;
        return;
    }

    let mut matches = Vec::new();

    if let Ok(out) = Command::new("pacman").arg("-Qq").output().await {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            if line.to_lowercase() == package.to_lowercase() {
                matches.push((line.to_string(), "Pacman/AUR", true));
            } else if line.to_lowercase().contains(&package.to_lowercase()) {
                matches.push((line.to_string(), "Pacman/AUR", false));
            }
        }
    }

    if let Ok(out) = Command::new("flatpak").args(["list", "--app", "--columns=application,name"]).output().await {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let (app_id, name) = (parts[0].trim(), parts[1].trim());
                if app_id.to_lowercase() == package.to_lowercase() || name.to_lowercase() == package.to_lowercase() {
                    matches.push((app_id.to_string(), "Flatpak", true));
                } else if app_id.to_lowercase().contains(&package.to_lowercase()) || name.to_lowercase().contains(&package.to_lowercase()) {
                    matches.push((app_id.to_string(), "Flatpak", false));
                }
            }
        }
    }

    if matches.is_empty() {
        eprintln!("❌ Пакет '{}' не найден в системе (ни в Gpkg, ни в Pacman, ни во Flatpak).", package);
        return;
    }

    let target = if matches.len() == 1 || matches.iter().any(|m| m.2) {
        matches.iter().find(|m| m.2).unwrap_or(&matches[0]).clone()
    } else {
        println!("💡 Найдено несколько совпадений. Выберите пакет для удаления:");
        for (i, m) in matches.iter().enumerate() {
            println!("  {}) {} [{}]", i + 1, m.0, m.1);
        }
        print!("Введите номер (или 0 для отмены): ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        if let Ok(num) = input.trim().parse::<usize>() {
            if num == 0 || num > matches.len() { return; }
            matches[num - 1].clone()
        } else {
            return;
        }
    };

    println!("🗑 Удаление пакета: {} [{}]", target.0, target.1);

    if target.1 == "Pacman/AUR" {
        let mut args = vec!["pacman", "-Rns", &target.0];
        if noconfirm { args.push("--noconfirm"); }
        let _ = Command::new("sudo").args(&args).status().await;
    } else {
        let mut args = vec!["uninstall", &target.0];
        if noconfirm { args.push("-y"); }
        let _ = Command::new("flatpak").args(&args).status().await;
    }
}

pub async fn update_system(noconfirm: bool) {
    println!("🔄 Обновление системных пакетов (Pacman/AUR)...");
    let mut args = vec!["pacman", "-Syu"];
    if noconfirm { args.push("--noconfirm"); }
    let _ = Command::new("sudo").args(&args).status().await;

    println!("📦 Обновление Flatpak...");
    let mut flat_args = vec!["update"];
    if noconfirm { flat_args.push("-y"); }
    let _ = Command::new("flatpak").args(&flat_args).status().await;
}

pub async fn clean_system(noconfirm: bool) {
    println!("🧹 Очистка кэша пакетов Pacman...");
    let mut pac_args = vec!["pacman", "-Sc"];
    if noconfirm { pac_args.push("--noconfirm"); }
    let _ = Command::new("sudo").args(&pac_args).status().await;

    println!("🧹 Удаление неиспользуемых Flatpak-пакетов (orphan)...");
    let mut flat_args = vec!["uninstall", "--unused"];
    if noconfirm { flat_args.push("-y"); }
    let _ = Command::new("flatpak").args(&flat_args).status().await;
    
    println!("✅ Очистка завершена.");
}