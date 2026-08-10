use tokio::process::Command;
use std::io::{self, Write};

fn norm_name(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

#[derive(Debug, Clone)]
struct InstallMatch {
    name: String,
    kind: &'static str,
    exact: bool,
}

async fn run_sudo(args: &[&str]) -> bool {
    let no_tty = !std::io::IsTerminal::is_terminal(&std::io::stdin());
    let cached = Command::new("sudo")
        .args(["-n", "true"])
        .output()
        .await
        .map_or(false, |o| o.status.success());

    if cached || !no_tty {
        return Command::new("sudo")
            .args(args)
            .status()
            .await
            .map_or(false, |s| s.success());
    }

    eprintln!("🔑 Требуется sudo, но нет терминала для запроса пароля.\n   Выполните команду в терминале или настройте права sudo.");
    false
}

async fn find_installed_matches(package: &str) -> Vec<InstallMatch> {
    let q = norm_name(package);
    let q_empty = q.is_empty();
    let mut matches = Vec::new();

    for gpkg_name in crate::gpkg::list_gpkg_packages() {
        let n = norm_name(&gpkg_name);
        if n == q {
            matches.push(InstallMatch { name: gpkg_name, kind: "Gpkg", exact: true });
        } else if !q_empty && n.contains(&q) {
            matches.push(InstallMatch { name: gpkg_name, kind: "Gpkg", exact: false });
        }
    }

    if let Ok(out) = Command::new("pacman").arg("-Qq").output().await {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let n = norm_name(line);
            if n == q {
                matches.push(InstallMatch { name: line.to_string(), kind: "Pacman", exact: true });
            } else if !q_empty && n.contains(&q) && n.len() >= q.len() + 3 {
                matches.push(InstallMatch { name: line.to_string(), kind: "Pacman", exact: false });
            }
        }
    }

    if let Ok(out) = Command::new("flatpak").args(["list", "--app", "--columns=application,name"]).output().await {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let mut parts = line.splitn(2, '\t');
            let (app_id, name) = (parts.next().unwrap_or("").trim(), parts.next().unwrap_or("").trim());
            if norm_name(app_id) == q || norm_name(name) == q {
                matches.push(InstallMatch { name: app_id.to_string(), kind: "Flatpak", exact: true });
            } else if !q_empty && (norm_name(app_id).contains(&q) || norm_name(name).contains(&q)) {
                matches.push(InstallMatch { name: app_id.to_string(), kind: "Flatpak", exact: false });
            }
        }
    }

    if let Ok(out) = Command::new("snap").arg("list").output().await {
        for line in String::from_utf8_lossy(&out.stdout).lines().skip(1) {
            let snap_name = line.split_whitespace().next().unwrap_or("").to_string();
            if snap_name.is_empty() { continue; }
            let n = norm_name(&snap_name);
            if n == q {
                matches.push(InstallMatch { name: snap_name, kind: "Snap", exact: true });
            } else if !q_empty && n.contains(&q) {
                matches.push(InstallMatch { name: snap_name, kind: "Snap", exact: false });
            }
        }
    }

    let mut path_dirs: Vec<std::path::PathBuf> = vec![
        std::path::PathBuf::from("/usr/local/bin"),
        std::path::PathBuf::from("/usr/bin"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        path_dirs.push(std::path::Path::new(&home).join(".local").join("bin"));
    }
    for dir in path_dirs {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if norm_name(&fname) == q {
                    matches.push(InstallMatch { name: fname, kind: "PATH", exact: true });
                }
            }
        }
    }

    for base in ["/opt", "/usr/share"] {
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let Ok(ft) = entry.file_type() else { continue };
                if !ft.is_dir() { continue; }
                let dname = entry.file_name().to_string_lossy().to_string();
                let n = norm_name(&dname);
                if n == q {
                    matches.push(InstallMatch {
                        name: dname.clone(),
                        kind: if base == "/opt" { "/opt" } else { "/usr/share" },
                        exact: true,
                    });
                } else if !q_empty && n.len() >= q.len() + 1 && n.contains(&q) {
                    matches.push(InstallMatch {
                        name: dname.clone(),
                        kind: if base == "/opt" { "/opt" } else { "/usr/share" },
                        exact: false,
                    });
                }
            }
        }
    }

    let desktop_dirs: Vec<std::path::PathBuf> = {
        let mut dirs = vec![
            std::path::PathBuf::from("/usr/share/applications"),
            std::path::PathBuf::from("/usr/local/share/applications"),
        ];
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(std::path::Path::new(&home).join(".local").join("share").join("applications"));
        }
        dirs
    };
    for dir in desktop_dirs {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if !fname.ends_with(".desktop") { continue; }
                let stem = fname.trim_end_matches(".desktop");
                if norm_name(stem) == q {
                    matches.push(InstallMatch { name: fname, kind: ".desktop", exact: true });
                    continue;
                }
                let Ok(content) = std::fs::read_to_string(entry.path()) else { continue };
                if let Some(exec) = content.lines().find(|l| l.starts_with("Exec=")) {
                    let cmd = exec.trim_start_matches("Exec=").split_whitespace().next().unwrap_or("");
                    let cmd = cmd.trim_matches('"');
                    let exe = std::path::Path::new(cmd)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if norm_name(&exe) == q {
                        matches.push(InstallMatch { name: fname, kind: ".desktop", exact: true });
                    }
                }
            }
        }
    }

    if let Ok(out) = Command::new("systemctl").args(["list-unit-files", "--no-legend"]).output().await {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let unit = line.split_whitespace().next().unwrap_or("").to_string();
            if unit.is_empty() { continue; }
            let stem = unit.split('.').next().unwrap_or("").to_string();
            if norm_name(&stem) == q {
                matches.push(InstallMatch { name: unit.clone(), kind: "systemd", exact: true });
            } else if !q_empty && stem.contains(&q) && stem.len() >= q.len() + 1 {
                matches.push(InstallMatch { name: unit.clone(), kind: "systemd", exact: false });
            }
        }
    }

    let mut seen = std::collections::HashSet::new();
    matches.retain(|m| seen.insert(norm_name(&m.name)));

    matches
}

pub async fn remove_package(package: &str, noconfirm: bool, remove_all: bool) {
    if remove_all {
        remove_all_packages(noconfirm).await;
        return;
    }

    println!("🔍 Поиск '{}' среди всего установленного ПО...", package);

    let matches = find_installed_matches(package).await;

    if matches.is_empty() {
        eprintln!("❌ '{}' не найден в системе.", package);
        return;
    }

    let target = if matches.len() == 1 || matches.iter().any(|m| m.exact) {
        matches.iter().find(|m| m.exact).unwrap_or(&matches[0]).clone()
    } else {
        println!("💡 Найдено несколько совпадений. Выберите, что удалить:");
        for (i, m) in matches.iter().enumerate() {
            println!("  {}) {} [{}]", i + 1, m.name, m.kind);
        }
        print!("Введите номер (или 0 для отмены): ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        match input.trim().parse::<usize>() {
            Ok(num) if num > 0 && num <= matches.len() => matches[num - 1].clone(),
            _ => return,
        }
    };

    println!("🗑 Удаление: {} [{}]", target.name, target.kind);
    remove_match(target, noconfirm).await;
}

async fn remove_match(target: InstallMatch, noconfirm: bool) {
    match target.kind {
        "Gpkg" => crate::gpkg::remove_gpkg(&target.name).await,
        "Pacman" => {
            let mut args = vec!["pacman", "-Rns", target.name.as_str()];
            if noconfirm { args.push("--noconfirm"); }
            run_sudo(&args).await;
        }
        "Flatpak" => {
            let mut args = vec!["uninstall", target.name.as_str()];
            if noconfirm { args.push("-y"); }
            let _ = Command::new("flatpak").args(&args).status().await;
        }
        "Snap" => {
            let args = vec!["snap", "remove", target.name.as_str()];
            run_sudo(&args).await;
        }
        "PATH" => remove_path_binary(&target.name).await,
        "/opt" | "/usr/share" => remove_unmanaged_app(&target.name, noconfirm).await,
        ".desktop" => remove_desktop_files(&target.name, noconfirm).await,
        "systemd" => remove_systemd_unit(&target.name, noconfirm).await,
        _ => {}
    }
}

async fn remove_path_binary(name: &str) {
    let mut removed_any = false;
    let home = std::env::var_os("HOME").unwrap_or_default();

    let mut dirs: Vec<std::path::PathBuf> = vec![
        std::path::PathBuf::from("/usr/local/bin"),
        std::path::PathBuf::from("/usr/bin"),
    ];
    dirs.push(std::path::Path::new(&home).join(".local").join("bin"));

    for dir in dirs {
        let path = dir.join(name);
        if !path.exists() { continue; }
        let is_user = path.starts_with(&home);
        if is_user {
            if std::fs::remove_file(&path).is_ok() {
                println!("  - Удален: {}", path.display());
                removed_any = true;
            }
        } else if run_sudo(&["rm", "-f", path.to_str().unwrap_or(name)]).await {
            println!("  - Удален: {}", path.display());
            removed_any = true;
        }
    }

    if !removed_any {
        eprintln!("⚠ Бинарник '{}' не найден в PATH.", name);
    }
}

async fn remove_unmanaged_app(name: &str, noconfirm: bool) {
    println!("  Удаление файлов приложения '{}':", name);
    let home = std::env::var_os("HOME").unwrap_or_default();

    let mut roots: Vec<std::path::PathBuf> = vec![
        std::path::PathBuf::from("/opt").join(name),
        std::path::PathBuf::from("/usr/share").join(name),
        std::path::PathBuf::from("/etc").join(name),
        std::path::PathBuf::from("/var/lib").join(name),
    ];
    roots.push(std::path::Path::new(&home).join(".config").join(name));
    roots.push(std::path::Path::new(&home).join(".local").join("share").join(name));

    for root in roots {
        if !root.exists() { continue; }
        let is_user = root.starts_with(&home);
        if is_user {
            if std::fs::remove_dir_all(&root).is_ok() {
                println!("  - Удалено: {}", root.display());
            }
        } else if run_sudo(&["rm", "-rf", root.to_str().unwrap_or("")]).await {
            println!("  - Удалено: {}", root.display());
        }
    }

    remove_path_binary(name).await;
    remove_desktop_files(name, noconfirm).await;
    remove_matching_systemd_units(name).await;
    println!("✅ Приложение '{}' удалено.", name);
}

async fn remove_desktop_files(name: &str, _noconfirm: bool) {
    let q = norm_name(name);
    let home = std::env::var_os("HOME").unwrap_or_default();
    let desktop_dirs: Vec<std::path::PathBuf> = vec![
        std::path::PathBuf::from("/usr/share/applications"),
        std::path::PathBuf::from("/usr/local/share/applications"),
        std::path::Path::new(&home).join(".local").join("share").join("applications"),
    ];

    for dir in desktop_dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if !fname.ends_with(".desktop") { continue; }

            let mut matched = norm_name(fname.trim_end_matches(".desktop")) == q;
            if !matched {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Some(exec) = content.lines().find(|l| l.starts_with("Exec=")) {
                        let cmd = exec.trim_start_matches("Exec=").split_whitespace().next().unwrap_or("");
                        let cmd = cmd.trim_matches('"');
                        let exe = std::path::Path::new(cmd)
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        matched = norm_name(&exe) == q;
                    }
                }
            }

            if matched {
                let is_user = entry.path().starts_with(&home);
                if is_user {
                    let _ = std::fs::remove_file(entry.path());
                } else {
                    run_sudo(&["rm", "-f", entry.path().to_str().unwrap_or("")]).await;
                }
                println!("  - Удален desktop-файл: {}", entry.path().display());
            }
        }
    }
}

async fn remove_matching_systemd_units(name: &str) {
    let q = norm_name(name);
    let Ok(out) = Command::new("systemctl").args(["list-unit-files", "--no-legend"]).output().await else { return };
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let unit = line.split_whitespace().next().unwrap_or("").to_string();
        if unit.is_empty() { continue; }
        let stem = unit.split('.').next().unwrap_or("").to_string();
        if norm_name(&stem) != q { continue; }

        let unit_paths = [
            std::path::PathBuf::from("/etc/systemd/system").join(&unit),
            std::path::PathBuf::from("/usr/local/lib/systemd/system").join(&unit),
            std::path::PathBuf::from("/usr/lib/systemd/system").join(&unit),
        ];
        if !unit_paths.iter().any(|p| p.exists()) { continue; }

        let _ = run_sudo(&["systemctl", "disable", "--now", &unit]).await;
        if unit_paths[0].exists() {
            run_sudo(&["rm", "-f", unit_paths[0].to_str().unwrap_or("")]).await;
            println!("  - Удален systemd-юнит: {}", unit);
        }
    }
}

async fn remove_systemd_unit(unit: &str, _noconfirm: bool) {
    let _ = run_sudo(&["systemctl", "disable", "--now", unit]).await;
    let paths = [
        std::path::PathBuf::from("/etc/systemd/system").join(unit),
        std::path::PathBuf::from("/usr/local/lib/systemd/system").join(unit),
    ];
    for p in paths {
        if p.exists() {
            run_sudo(&["rm", "-f", p.to_str().unwrap_or(unit)]).await;
            println!("  - Удален systemd-юнит: {}", unit);
        }
    }
    println!("✅ systemd-юнит '{}' удален.", unit);
}

async fn remove_all_packages(noconfirm: bool) {
    println!("🧹 Удаление ВСЕХ установленных пакетов (Gpkg, Pacman, Flatpak)...");
    if !noconfirm {
        print!("Вы уверены? [y/N]: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("❌ Отменено.");
            return;
        }
    }

    println!("📦 Удаление Gpkg-пакетов...");
    for gpkg_name in crate::gpkg::list_gpkg_packages() {
        crate::gpkg::remove_gpkg(&gpkg_name).await;
    }

    println!("📦 Удаление пакетов Pacman...");
    if let Ok(out) = Command::new("pacman").arg("-Qq").output().await {
        let pkgs: Vec<String> = String::from_utf8_lossy(&out.stdout).lines().map(|s| s.to_string()).collect();
        if !pkgs.is_empty() {
            let mut args = vec!["pacman", "-Rns", "--noconfirm"];
            if noconfirm { args.push("--noconfirm"); }
            args.extend(pkgs.iter().map(|s| s.as_str()));
            let _ = Command::new("sudo").args(&args).status().await;
        }
    }

    println!("📦 Удаление Flatpak-пакетов...");
    if let Ok(out) = Command::new("flatpak").args(["list", "--app", "--columns=application"]).output().await {
        let apps: Vec<String> = String::from_utf8_lossy(&out.stdout).lines().map(|s| s.to_string()).collect();
        if !apps.is_empty() {
            let mut args = vec!["uninstall", "--all"];
            if noconfirm { args.push("-y"); }
            let _ = Command::new("flatpak").args(&args).status().await;
        }
    }

    println!("✅ Все пакеты удалены.");
}

pub async fn package_info(package: &str) {
    println!("ℹ️ Информация о '{}'...", package);

    if let Some((entry, file_count)) = crate::gpkg::get_gpkg_info(package) {
        println!("\n[Gpkg] Установлен локально");
        println!("  Имя:    {}", package);
        println!("  Версия: {}", entry.version);
        println!("  Файлов: {}", file_count);
        if let Some(exec) = entry.exec_binary { println!("  Бинарь: {}", exec); }
        if let Some(repo) = entry.github_repo { println!("  GitHub: {}", repo); }
        println!("  Файлы:");
        for file in &entry.files {
            println!("    - {}", file);
        }
        return;
    }

    if let Ok(out) = Command::new("pacman").args(["-Si", package]).output().await {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            println!("\n[Pacman] {}", package);
            for line in stdout.lines() {
                if !line.trim().is_empty() {
                    println!("  {}", line);
                }
            }
            return;
        }
    }

    if let Some(gos) = crate::search::get_gos_info(package).await {
        println!("\n[G OS Repository] {}", gos.name);
        println!("  Версия:     {}", gos.version);
        println!("  Описание:   {}", gos.description);
        println!("  Создатель:  {}", gos.creator);
        println!("  URL:        {}", gos.url);
        return;
    }

    let flat = crate::search::search_flatpak(package).await;
    if let Some(matched) = flat.iter().find(|p| p.name.to_lowercase() == package.to_lowercase()) {
        println!("\n[Flatpak] {}", matched.name);
        println!("  Версия:   {}", matched.version);
        println!("  Описание: {}", matched.description);
        return;
    }

    eprintln!("❌ Пакет '{}' не найден ни в одном источнике.", package);
}

pub async fn list_packages(show_flatpak: bool, show_gpkg: bool) {
    println!("📋 Список установленных пакетов:\n");

    if show_gpkg {
        println!("[Gpkg]");
        let gpkg_list = crate::gpkg::list_gpkg_detailed();
        if gpkg_list.is_empty() {
            println!("  (пусто)");
        } else {
            for (name, version, file_count) in gpkg_list {
                println!("  {} v{} ({} файлов)", name, version, file_count);
            }
        }
        println!();
    }

    if show_flatpak {
        println!("[Flatpak]");
        if let Ok(out) = Command::new("flatpak").args(["list", "--app", "--columns=application,version,name"]).output().await {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut any = false;
            for line in stdout.lines() {
                if line.trim().is_empty() || line.contains("Application ID") { continue; }
                any = true;
                let cols: Vec<&str> = line.split('\t').collect();
                if cols.len() >= 2 {
                    println!("  {} v{} {}", cols[0].trim(), cols[1].trim(), 
                        if cols.len() >= 3 { cols[2].trim() } else { "" });
                }
            }
            if !any { println!("  (пусто)"); }
        } else {
            println!("  (недоступно)");
        }
        println!();
    }

    println!("[Pacman]");
    if let Ok(out) = Command::new("pacman").args(["-Q", "--color=never"]).output().await {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut count = 0;
        for line in stdout.lines() {
            if line.trim().is_empty() { continue; }
            count += 1;
            println!("  {}", line);
        }
        println!("\nВсего пакетов (pacman): {}", count);
    } else {
        println!("  (недоступно)");
    }
}

pub fn verify_gpkg() {
    println!("🔍 Проверка целостности установленных .gpkg пакетов...");

    let gpkg_list = crate::gpkg::list_gpkg_detailed();
    if gpkg_list.is_empty() {
        println!("✅ Не установлено ни одного .gpkg пакета — проверять нечего.");
        return;
    }

    let mut all_ok = true;
    for (name, _version, _file_count) in gpkg_list {
        let Some((entry, _)) = crate::gpkg::get_gpkg_info(&name) else { continue };
        let mut missing = Vec::new();
        for file in &entry.files {
            if !std::path::Path::new(file).exists() {
                missing.push(file.clone());
            }
        }
        if missing.is_empty() {
            println!("  ✔ {} — все {} файлов на месте", name, entry.files.len());
        } else {
            all_ok = false;
            println!("  ✘ {} — отсутствует {} файл(ов):", name, missing.len());
            for m in &missing {
                println!("      - {}", m);
            }
        }
    }

    if all_ok {
        println!("✅ Все .gpkg пакеты целы.");
    } else {
        eprintln!("⚠ Обнаружены проблемы с файлами .gpkg пакетов. Рекомендуется переустановить их (gvalli update --gpkg).");
    }
}

pub async fn doctor() {
    println!("🩺 Диагностика окружения...\n");

    let tools: &[(&str, &str)] = &[
        ("pacman", "менеджер пакетов Arch"),
        ("sudo", "эскалация привилегий"),
        ("flatpak", "плоский пакетный менеджер"),
        ("git", "система контроля версий"),
        ("cargo", "Rust-сборка"),
    ];

    for (tool, purpose) in tools {
        let found = Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {}", tool))
            .output()
            .await
            .map_or(false, |o| o.status.success());
        if found {
            println!("  ✔ {} — {} найден", tool, purpose);
        } else {
            println!("  ✘ {} — {} НЕ найден", tool, purpose);
        }
    }

    let is_root = std::env::var("USER").map_or(false, |u| u == "root");
    let sudo_available = Command::new("sudo")
        .args(["-n", "true"])
        .output()
        .await
        .map_or(false, |o| o.status.success());
    println!("\n  {} — запуск от root", if is_root { "✔" } else { "✘" });
    println!("  {} — sudo работает", if sudo_available { "✔" } else { "✘" });

    let db_path = "/var/lib/gvalli/gpkg.json";
    let db_ok = std::path::Path::new(db_path).exists();
    println!("  {} — БД Gpkg найдена ({})", if db_ok { "✔" } else { "✘" }, db_path);

    println!("\nПроверка сети (G OS Repository)...");
    let gos_ok = crate::search::check_gos_reachable().await;
    if gos_ok {
        println!("  ✔ G OS Repository доступен");
    } else {
        println!("  ✘ G OS Repository недоступен (проверьте интернет)");
    }

    println!("\n🩺 Диагностика завершена.");
}

pub async fn autoremove(noconfirm: bool) {
    println!("🧹 Поиск пакетов-сирот (pacman)...");
    let orphan_check = Command::new("pacman").arg("-Qtdq").output().await;
    if let Ok(out) = orphan_check {
        let orphans: Vec<String> = String::from_utf8_lossy(&out.stdout).lines().map(|s| s.to_string()).collect();
        if orphans.is_empty() {
            println!("  ✔ Осиротевших пакетов нет.");
        } else {
            println!("  Найдено {} сирот:", orphans.len());
            for o in &orphans { println!("    - {}", o); }
            let mut args = vec!["pacman", "-Rns"];
            args.extend(orphans.iter().map(|s| s.as_str()));
            if noconfirm { args.push("--noconfirm"); }
            let _ = Command::new("sudo").args(&args).status().await;
        }
    }

    println!("\n🧹 Удаление неиспользуемых Flatpak-рантаймов...");
    let mut flat_args = vec!["uninstall", "--unused"];
    if noconfirm { flat_args.push("-y"); }
    let _ = Command::new("flatpak").args(&flat_args).status().await;

    println!("✅ Автоочистка завершена.");
}

pub async fn update_system(noconfirm: bool) {
    println!("🔄 Обновление системных пакетов (Pacman)...");
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

pub async fn clean_deep(noconfirm: bool) {
    println!("🧹 Глубокая очистка...");

    if !noconfirm {
        print!("Очистить ~/.cache и /tmp? [y/N]: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("❌ Пропуск глубокой очистки.");
            return;
        }
    }

    println!("🧹 Полная очистка кэша pacman (-Scc)...");
    if noconfirm {
        let _ = Command::new("sudo").args(["pacman", "-Scc", "--noconfirm"]).status().await;
    } else {
        let _ = Command::new("sudo").args(["pacman", "-Scc"]).status().await;
    }

    println!("🧹 Удаление всех неиспользуемых Flatpak...");
    let mut flat_args = vec!["uninstall", "--all", "--unused"];
    if noconfirm { flat_args.push("-y"); }
    let _ = Command::new("flatpak").args(&flat_args).status().await;

    if let Some(home) = std::env::var_os("HOME") {
        let cache = std::path::Path::new(&home).join(".cache");
        if cache.exists() {
            println!("🧹 Очистка {:?}...", cache);
            if let Ok(entries) = std::fs::read_dir(&cache) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    if name.starts_with("gvalli") || name == "pip" || name == "npm" || name == "yarn" || name == "go-build" {
                        let _ = std::fs::remove_dir_all(&path);
                    }
                }
            }
        }
    }

    if let Ok(entries) = std::fs::read_dir("/tmp") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("gvalli-") {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }

    println!("✅ Глубокая очистка завершена.");
}

pub async fn run_gpkg(package: &str) {
    let exec = if let Some((entry, _)) = crate::gpkg::get_gpkg_info(package) {
        let guessed = entry.exec_binary.clone().unwrap_or_default();
        if !guessed.is_empty() {
            guessed
        } else {
            entry.files.iter()
                .find(|f| f.starts_with("/usr/bin/"))
                .and_then(|f| f.rsplit('/').next())
                .map(|s| s.to_string())
                .unwrap_or_else(|| package.to_string())
        }
    } else {
        let in_path = Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {}", package))
            .output()
            .await
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();

        if !in_path.is_empty() {
            in_path
        } else if std::path::Path::new("/opt").join(package).is_dir() {
            format!("/opt/{}/{}", package, package)
        } else {
            eprintln!("❌ '{}' не найден: ни .gpkg, ни в PATH, ни в /opt.", package);
            return;
        }
    };

    println!("🚀 Запуск '{}'...", exec);
    let status = Command::new("sh")
        .arg("-c")
        .arg(&exec)
        .status()
        .await;
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => eprintln!("⚠ Процесс завершился с кодом {}", s.code().unwrap_or(-1)),
        Err(e) => eprintln!("❌ Не удалось запустить '{}': {}", exec, e),
    }
}