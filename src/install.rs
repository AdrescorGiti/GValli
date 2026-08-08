use std::env;
use std::pin::Pin;
use tokio::process::Command;
use crate::models::PackageSource;

pub async fn smart_install(package: &str, noconfirm: bool, force_source: Option<&PackageSource>) {
    let mut source = force_source.cloned();
    let mut actual_package_name = package.to_string();

    // РОУТИНГ (если вызвано без TUI)
    if source.is_none() {
        if Command::new("pacman").args(["-Si", package]).output().await.map_or(false, |o| o.status.success()) {
            source = Some(PackageSource::Pacman);
        } else {
            let flat_res = crate::search::search_flatpak(package).await;
            if let Some(matched) = flat_res.iter().find(|p| p.name.to_lowercase() == package.to_lowercase()) {
                source = Some(PackageSource::Flatpak);
                actual_package_name = matched.name.clone();
            } else {
                let aur_check = Command::new("git").args(["ls-remote", &format!("https://aur.archlinux.org/{}.git", package)]).output().await;
                if aur_check.map_or(false, |o| o.status.success()) {
                    source = Some(PackageSource::Aur);
                } else {
                    eprintln!("❌ Точный пакет '{}' не найден.", package);
                    return;
                }
            }
        }
    }

    // ==========================================
    // ФИКС UX: АВТОМАТИЧЕСКИЙ ЗАПРОС ПРАВ ROOT
    // ==========================================
    if source.as_ref().unwrap() == &PackageSource::Aur || source.as_ref().unwrap() == &PackageSource::Pacman {
        if env::var("USER").unwrap_or_default() != "root" {
            println!("🔑 Для установки '{}' требуются права root. Вызов sudo...", actual_package_name);
            let mut args = vec!["gvalli", "install", &actual_package_name];
            if noconfirm { args.push("--noconfirm"); }
            
            let status = Command::new("sudo").args(&args).status().await;
            if status.is_err() || !status.unwrap().success() {
                eprintln!("❌ Ошибка эскалации привилегий.");
            }
            return;
        }
    }

    match source.unwrap() {
        PackageSource::Pacman => {
            println!("🚀 Установка из Pacman: {}", actual_package_name);
            let mut args = vec!["-S", &actual_package_name];
            if noconfirm { args.push("--noconfirm"); }
            let _ = Command::new("pacman").args(&args).status().await;
        }
        PackageSource::Flatpak => {
            println!("🚀 Установка из Flatpak: {}", actual_package_name);
            let mut args = vec!["install", &actual_package_name];
            if noconfirm { args.push("-y"); }
            let _ = Command::new("flatpak").args(&args).status().await;
        }
        PackageSource::Aur => {
            install_aur(actual_package_name, noconfirm).await;
        }
    }
}

// Рекурсивный AUR остается таким же, но теперь гарантированно выполняется под sudo (рутом с $SUDO_USER)
fn install_aur(package: String, noconfirm: bool) -> Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
    Box::pin(async move {
        println!("🚀 Установка из AUR: {}", package);

        let sudo_user = env::var("SUDO_USER").unwrap_or_default();
        if sudo_user.is_empty() {
            eprintln!("❌ Критическая ошибка: Не найден $SUDO_USER. Невозможно собрать пакет в песочнице.");
            return;
        }

        let build_dir = format!("/tmp/gvalli-build-{}", package);
        let _ = Command::new("rm").args(["-rf", &build_dir]).status().await;

        println!("🔽 Клонирование AUR ({})", sudo_user);
        let clone_cmd = format!("git clone https://aur.archlinux.org/{}.git {}", package, build_dir);
        if !Command::new("su").args(["-", &sudo_user, "-c", &clone_cmd]).status().await.unwrap().success() {
            eprintln!("❌ Ошибка клонирования. Пакет не существует.");
            return;
        }

        println!("🔍 Анализ зависимостей...");
        let srcinfo_cmd = format!("cd {} && makepkg --printsrcinfo", build_dir);
        if let Ok(out) = Command::new("su").args(["-", &sudo_user, "-c", &srcinfo_cmd]).output().await {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let re = regex::Regex::new(r"^[a-z0-9_]*depends\s*=\s*([a-zA-Z0-9_\-\.\+]+)").unwrap();
            let mut all_deps = Vec::new();

            for line in stdout.lines() {
                if let Some(cap) = re.captures(line.trim()) {
                    all_deps.push(cap[1].to_string());
                }
            }

            if !all_deps.is_empty() {
                let missing_check = Command::new("pacman").arg("-T").args(&all_deps).output().await;
                if let Ok(missing_out) = missing_check {
                    let missing_str = String::from_utf8_lossy(&missing_out.stdout);
                    let missing_deps: Vec<&str> = missing_str.split_whitespace().collect();

                    if !missing_deps.is_empty() {
                        let mut repo_deps = Vec::new();
                        let mut aur_deps = Vec::new();

                        for dep in missing_deps {
                            let check = Command::new("pacman").args(["-Sp", dep]).output().await;
                            if check.map_or(false, |o| o.status.success()) {
                                repo_deps.push(dep);
                            } else {
                                aur_deps.push(dep);
                            }
                        }

                        // Рекурсия AUR
                        for aur_dep in aur_deps {
                            println!("🔗 AUR-зависимость: {}. Устанавливаем...", aur_dep);
                            install_aur(aur_dep.to_string(), noconfirm).await;
                        }

                        // Pacman зависимости
                        if !repo_deps.is_empty() {
                            println!("📦 Установка системных зависимостей (root): {:?}", repo_deps);
                            let mut pac_args = vec!["-S", "--needed"];
                            if noconfirm { pac_args.push("--noconfirm"); }
                            pac_args.extend(repo_deps);
                            let _ = Command::new("pacman").args(&pac_args).status().await;
                        }
                    }
                }
            }
        }

        println!("🛠 Сборка makepkg ({}) ...", package);
        let build_cmd = format!("cd {} && makepkg -cf{}", build_dir, if noconfirm { " --noconfirm" } else { "" });
        if Command::new("su").args(["-", &sudo_user, "-c", &build_cmd]).status().await.unwrap().success() {
            let install_cmd = format!("pacman -U{} {}/*.pkg.tar.zst", if noconfirm { " --noconfirm" } else { "" }, build_dir);
            let _ = Command::new("sh").args(["-c", &install_cmd]).status().await;
            println!("✅ Пакет {} успешно установлен!", package);
        } else {
            eprintln!("❌ Сбой сборки {}.", package);
        }
    })
}
