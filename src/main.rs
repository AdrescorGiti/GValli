mod cli;
mod models;
mod search;
mod install;
mod commands;
pub mod gpkg;

use clap::Parser;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use std::collections::HashMap;
use std::io::{stdout, Write};
use std::time::Duration;
use tokio::sync::mpsc;

struct RawModeGuard;
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), cursor::Show, ResetColor);
    }
}

enum AppState {
    Menu(usize),
    SearchInput(String, Vec<models::PackageResult>),
    SearchResults(Vec<models::PackageResult>, usize),
    InstallInput(String),
    RemoveInput(String),
    InfoInput(String),
    Exit,
}

#[tokio::main]
async fn main() {
    let cli_args = cli::Cli::parse();

    if let Some(command) = cli_args.command {
        execute_cli(command).await;
        return;
    }

    run_tui().await;
}

async fn execute_cli(command: cli::Commands) {
    match command {
        cli::Commands::Search { query, pacman, flatpak, json } => {
            let search_all = !pacman && !flatpak;
            let (mut gos_res, mut pacman_res, mut flatpak_res) = (vec![], vec![], vec![]);

            if search_all { gos_res = search::search_gos(&query).await; }
            if search_all || pacman { pacman_res = search::search_pacman(&query).await; }
            if search_all || flatpak { flatpak_res = search::search_flatpak(&query).await; }

            let mut deduplicated: HashMap<String, models::PackageResult> = HashMap::new();
            for pkg in gos_res.into_iter().chain(pacman_res).chain(flatpak_res) {
                deduplicated.entry(pkg.name.clone())
                    .and_modify(|e| { if pkg.source < e.source { *e = pkg.clone(); } })
                    .or_insert(pkg);
            }

            let mut final_list: Vec<_> = deduplicated.values().cloned().collect();
            final_list.sort_by(|a, b| a.name.cmp(&b.name));

            if json {
                let payload = serde_json::json!({
                    "query": query,
                    "count": final_list.len(),
                    "results": final_list,
                });
                println!("{}", serde_json::to_string_pretty(&payload).unwrap());
            } else {
                for p in final_list {
                    println!("[{:?}] {} v{} - {}", p.source, p.name, p.version, p.description);
                }
            }
        }
        cli::Commands::Create { path, sync: _, install } => {
            let target = path.as_deref().unwrap_or(".");
            let _ = gpkg::create_package(target, true, install).await;
        }
        cli::Commands::Install { package, noconfirm } => {
            install::smart_install(&package, noconfirm, None).await;
        }
        cli::Commands::Remove { package, noconfirm, all } => {
            match package {
                Some(pkg_name) => commands::remove_package(&pkg_name, noconfirm, all).await,
                None if all => commands::remove_package("", noconfirm, true).await,
                None => {},
            }
        }
        cli::Commands::Update { noconfirm, gpkg } => {
            commands::update_system(noconfirm).await;
            if gpkg {
                let _ = gpkg::update_all_gpkg().await;
            }
        }
        cli::Commands::Clean { noconfirm, all } => {
            commands::clean_system(noconfirm).await;
            if all {
                commands::clean_deep(noconfirm).await;
            }
        }
        cli::Commands::Info { package } => {
            commands::package_info(&package).await;
        }
        cli::Commands::List { flatpak, gpkg } => {
            commands::list_packages(flatpak, gpkg).await;
        }
        cli::Commands::Autoremove { noconfirm } => {
            commands::autoremove(noconfirm).await;
        }
        cli::Commands::Verify => {
            commands::verify_gpkg();
        }
        cli::Commands::Doctor => {
            commands::doctor().await;
        }
        cli::Commands::Run { package } => {
            commands::run_gpkg(&package).await;
        }
        cli::Commands::Gpkg { action } => match action {
            cli::GpkgCommands::Create { path, sync: _, install } => {
                let target = path.as_deref().unwrap_or(".");
                let _ = gpkg::create_package(target, true, install).await;
            },
            cli::GpkgCommands::Install { target } => gpkg::install_package(&target).await,
            cli::GpkgCommands::Get { url } => gpkg::get_package(&url).await,
        },
    }
}

async fn search_all(q: &str) -> Vec<models::PackageResult> {
    let mut all = Vec::new();
    all.extend(search::search_gos(q).await);
    all.extend(search::search_pacman(q).await);
    all.extend(search::search_flatpak(q).await);

    let mut deduplicated: HashMap<String, models::PackageResult> = HashMap::new();
    for pkg in all {
        deduplicated.entry(pkg.name.clone())
            .and_modify(|e| { if pkg.source < e.source { *e = pkg.clone(); } })
            .or_insert(pkg);
    }
    let mut final_list: Vec<_> = deduplicated.values().cloned().collect();
    final_list.sort_by(|a, b| a.name.cmp(&b.name));
    final_list
}

async fn run_tui() {
    enable_raw_mode().unwrap();
    let _guard = RawModeGuard;
    let mut out = stdout();
    execute!(out, cursor::Hide, Clear(ClearType::All)).unwrap();

    let (tx_query, mut rx_query) = mpsc::channel::<String>(10);
    let (tx_results, mut rx_results) = mpsc::channel::<Vec<models::PackageResult>>(10);

    tokio::spawn(async move {
        while let Some(q) = rx_query.recv().await {
            tokio::time::sleep(Duration::from_millis(400)).await;
            let mut final_q = q;
            while let Ok(new_q) = rx_query.try_recv() {
                final_q = new_q;
            }
            if !final_q.is_empty() {
                let res = search_all(&final_q).await;
                let _ = tx_results.send(res).await;
            } else {
                let _ = tx_results.send(vec![]).await;
            }
        }
    });

    let mut state = AppState::Menu(0);
    let menu_items = [
        "Search Packages",
        "Install Package",
        "Remove Package",
        "Update System",
        "Clean System",
        "Package Info",
        "List Installed",
        "Autoremove Orphans",
        "Verify Gpkg",
        "System Doctor",
        "Exit"
    ];

    loop {
        let (term_width, term_height) = crossterm::terminal::size().unwrap_or((80, 24));
        queue!(out, cursor::MoveTo(0, 0), Clear(ClearType::All)).unwrap();

        queue!(out, SetForegroundColor(Color::Magenta), Print("  _____  __      __      _ _ _ \r\n"),
                    Print(" / ____| \\ \\    / /     | | | |\r\n"),
                    Print("| |  __   \\ \\  / /_ _   | | | |\r\n"),
                    Print("| | |_ |   \\ \\/ / _` |  | | | |\r\n"),
                    Print("| |__| |    \\  / (_| |  | | | |\r\n"),
                    Print(" \\_____|     \\/ \\__,_|  |_|_|_|\r\n\r\n"), ResetColor).unwrap();

        if let Ok(res) = rx_results.try_recv() {
            if let AppState::SearchInput(_, ref mut current_res) = state {
                *current_res = res;
            }
        }

        match &state {
            AppState::Menu(selected) => {
                queue!(out, SetForegroundColor(Color::DarkCyan), Print(" ┌────────────────────────────────────────┐\r\n"), ResetColor).unwrap();
                for (i, item) in menu_items.iter().enumerate() {
                    if i == *selected {
                        queue!(out, SetForegroundColor(Color::DarkCyan), Print(" │ "), SetForegroundColor(Color::Black), SetBackgroundColor(Color::Green), Print(format!(" {:<36} ", item)), ResetColor, SetForegroundColor(Color::DarkCyan), Print("│\r\n"), ResetColor).unwrap();
                    } else {
                        queue!(out, SetForegroundColor(Color::DarkCyan), Print(" │ "), ResetColor, Print(format!("   {:<34} ", item)), SetForegroundColor(Color::DarkCyan), Print("│\r\n"), ResetColor).unwrap();
                    }
                }
                queue!(out, SetForegroundColor(Color::DarkCyan), Print(" └────────────────────────────────────────┘\r\n"), ResetColor).unwrap();
            }
            AppState::SearchInput(query, results) => {
                queue!(out, Print(" 🔍 Поиск: "), SetForegroundColor(Color::Yellow), Print(format!("{}\r\n\n", query)), ResetColor).unwrap();
                queue!(out, SetForegroundColor(Color::DarkGrey), Print(" ────────────────── ПРЕВЬЮ ──────────────────\r\n"), ResetColor).unwrap();
                for p in results.iter().take(5) {
                    queue!(out, SetForegroundColor(Color::Cyan), Print(format!(" [{:?}] ", p.source)), ResetColor, Print(format!("{} - {}\r\n", p.name, p.version))).unwrap();
                }
                queue!(out, SetForegroundColor(Color::DarkGrey), Print("\r\n [Enter] Полный список | [Backspace] Стереть/Назад\r\n"), ResetColor).unwrap();
            }
            AppState::SearchResults(results, selected) => {
                queue!(out, SetForegroundColor(Color::Green), Print(format!(" Найдено ({}):\r\n\n", results.len())), ResetColor).unwrap();
                let display_limit = (term_height as usize).saturating_sub(10);
                let start = if *selected >= display_limit { *selected - display_limit + 1 } else { 0 };
                let end = std::cmp::min(start + display_limit, results.len());

                for i in start..end {
                    let p = &results[i];
                    let desc = if p.description.chars().count() > term_width as usize - 30 {
                        p.description.chars().take(term_width as usize - 35).collect::<String>() + "..."
                    } else { p.description.clone() };

                    if i == *selected {
                        queue!(out, SetForegroundColor(Color::Black), SetBackgroundColor(Color::Cyan), Print(format!(" > [{:?}] {} v{} - {}\r\n", p.source, p.name, p.version, desc)), ResetColor).unwrap();
                    } else {
                        queue!(out, SetForegroundColor(Color::DarkCyan), Print(format!("   [{:?}] ", p.source)), ResetColor, Print(format!("{} v{} - {}\r\n", p.name, p.version, desc))).unwrap();
                    }
                }
                queue!(out, SetForegroundColor(Color::DarkGrey), Print("\r\n [Enter] Установить | [Esc] Назад\r\n"), ResetColor).unwrap();
            }
            AppState::InstallInput(query) => {
                queue!(out, Print(" 🚀 Установить пакет: "), SetForegroundColor(Color::Green), Print(format!("{}\r\n\n", query)), ResetColor).unwrap();
                queue!(out, SetForegroundColor(Color::DarkGrey), Print(" [Enter] Подтвердить | [Backspace] Отмена\r\n"), ResetColor).unwrap();
            }
            AppState::RemoveInput(query) => {
                queue!(out, Print(" 🗑 Удалить пакет: "), SetForegroundColor(Color::Red), Print(format!("{}\r\n\n", query)), ResetColor).unwrap();
                queue!(out, SetForegroundColor(Color::DarkGrey), Print(" [Enter] Подтвердить | [Backspace] Отмена\r\n"), ResetColor).unwrap();
            }
            AppState::InfoInput(query) => {
                queue!(out, Print(" ℹ️ Информация о пакете: "), SetForegroundColor(Color::Blue), Print(format!("{}\r\n\n", query)), ResetColor).unwrap();
                queue!(out, SetForegroundColor(Color::DarkGrey), Print(" [Enter] Подтвердить | [Backspace] Отмена\r\n"), ResetColor).unwrap();
            }
            AppState::Exit => break,
        }

        out.flush().unwrap();

        if event::poll(Duration::from_millis(50)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                if key.kind == KeyEventKind::Press {
                    match &mut state {
                        AppState::Menu(selected) => {
                            match key.code {
                                KeyCode::Up => if *selected > 0 { *selected -= 1; },
                                KeyCode::Down => if *selected < menu_items.len() - 1 { *selected += 1; },
                                KeyCode::Enter => {
                                    match *selected {
                                        0 => state = AppState::SearchInput(String::new(), vec![]),
                                        1 => state = AppState::InstallInput(String::new()),
                                        2 => state = AppState::RemoveInput(String::new()),
                                        3 => {
                                            execute!(out, ResetColor, Clear(ClearType::All), cursor::MoveTo(0,0), cursor::Show).unwrap();
                                            disable_raw_mode().unwrap();
                                            execute_cli(cli::Commands::Update { noconfirm: false, gpkg: true }).await;
                                            println!("\nНажмите Enter чтобы продолжить...");
                                            let mut buf = String::new();
                                            std::io::stdin().read_line(&mut buf).unwrap();
                                            enable_raw_mode().unwrap();
                                            execute!(out, cursor::Hide).unwrap();
                                        }
                                        4 => {
                                            execute!(out, ResetColor, Clear(ClearType::All), cursor::MoveTo(0,0), cursor::Show).unwrap();
                                            disable_raw_mode().unwrap();
                                            execute_cli(cli::Commands::Clean { noconfirm: false, all: false }).await;
                                            println!("\nНажмите Enter чтобы продолжить...");
                                            let mut buf = String::new();
                                            std::io::stdin().read_line(&mut buf).unwrap();
                                            enable_raw_mode().unwrap();
                                            execute!(out, cursor::Hide).unwrap();
                                        }
                                        5 => state = AppState::InfoInput(String::new()),
                                        6 => {
                                            execute!(out, ResetColor, Clear(ClearType::All), cursor::MoveTo(0,0), cursor::Show).unwrap();
                                            disable_raw_mode().unwrap();
                                            execute_cli(cli::Commands::List { flatpak: true, gpkg: true }).await;
                                            println!("\nНажмите Enter чтобы продолжить...");
                                            let mut buf = String::new();
                                            std::io::stdin().read_line(&mut buf).unwrap();
                                            enable_raw_mode().unwrap();
                                            execute!(out, cursor::Hide).unwrap();
                                        }
                                        7 => {
                                            execute!(out, ResetColor, Clear(ClearType::All), cursor::MoveTo(0,0), cursor::Show).unwrap();
                                            disable_raw_mode().unwrap();
                                            execute_cli(cli::Commands::Autoremove { noconfirm: false }).await;
                                            println!("\nНажмите Enter чтобы продолжить...");
                                            let mut buf = String::new();
                                            std::io::stdin().read_line(&mut buf).unwrap();
                                            enable_raw_mode().unwrap();
                                            execute!(out, cursor::Hide).unwrap();
                                        }
                                        8 => {
                                            execute!(out, ResetColor, Clear(ClearType::All), cursor::MoveTo(0,0), cursor::Show).unwrap();
                                            disable_raw_mode().unwrap();
                                            execute_cli(cli::Commands::Verify).await;
                                            println!("\nНажмите Enter чтобы продолжить...");
                                            let mut buf = String::new();
                                            std::io::stdin().read_line(&mut buf).unwrap();
                                            enable_raw_mode().unwrap();
                                            execute!(out, cursor::Hide).unwrap();
                                        }
                                        9 => {
                                            execute!(out, ResetColor, Clear(ClearType::All), cursor::MoveTo(0,0), cursor::Show).unwrap();
                                            disable_raw_mode().unwrap();
                                            execute_cli(cli::Commands::Doctor).await;
                                            println!("\nНажмите Enter чтобы продолжить...");
                                            let mut buf = String::new();
                                            std::io::stdin().read_line(&mut buf).unwrap();
                                            enable_raw_mode().unwrap();
                                            execute!(out, cursor::Hide).unwrap();
                                        }
                                        10 => state = AppState::Exit,
                                        _ => {}
                                    }
                                }
                                KeyCode::Esc => state = AppState::Exit,
                                _ => {}
                            }
                        }
                        AppState::SearchInput(q, results) => {
                            match key.code {
                                KeyCode::Char(c) => {
                                    q.push(c);
                                    let _ = tx_query.try_send(q.clone());
                                }
                                KeyCode::Backspace => {
                                    if q.is_empty() {
                                        state = AppState::Menu(0);
                                    } else {
                                        q.pop();
                                        let _ = tx_query.try_send(q.clone());
                                    }
                                }
                                KeyCode::Enter => {
                                    let r = results.clone();
                                    state = AppState::SearchResults(r, 0);
                                }
                                KeyCode::Esc => state = AppState::Menu(0),
                                _ => {}
                            }
                        }
                        AppState::SearchResults(results, selected) => {
                            match key.code {
                                KeyCode::Up => if *selected > 0 { *selected -= 1; },
                                KeyCode::Down => if *selected < results.len() - 1 { *selected += 1; },
                                KeyCode::Enter => {
                                    if !results.is_empty() {
                                        let chosen = results[*selected].clone();
                                        execute!(out, ResetColor, Clear(ClearType::All), cursor::MoveTo(0,0), cursor::Show).unwrap();
                                        disable_raw_mode().unwrap();
                                        install::smart_install(&chosen.name, false, Some(chosen.source)).await;
                                        println!("\nНажмите Enter чтобы продолжить...");
                                        let mut buf = String::new();
                                        std::io::stdin().read_line(&mut buf).unwrap();
                                        enable_raw_mode().unwrap();
                                        execute!(out, cursor::Hide).unwrap();
                                        state = AppState::Menu(0);
                                    }
                                }
                                KeyCode::Backspace | KeyCode::Esc => state = AppState::SearchInput(String::new(), vec![]),
                                _ => {}
                            }
                        }
                        AppState::InstallInput(q) => {
                            match key.code {
                                KeyCode::Char(c) => q.push(c),
                                KeyCode::Backspace => {
                                    if q.is_empty() { state = AppState::Menu(1); } else { q.pop(); }
                                }
                                KeyCode::Enter => {
                                    execute!(out, ResetColor, Clear(ClearType::All), cursor::MoveTo(0,0), cursor::Show).unwrap();
                                    disable_raw_mode().unwrap();
                                    execute_cli(cli::Commands::Install { package: q.clone(), noconfirm: false }).await;
                                    println!("\nНажмите Enter чтобы продолжить...");
                                    let mut buf = String::new();
                                    std::io::stdin().read_line(&mut buf).unwrap();
                                    enable_raw_mode().unwrap();
                                    execute!(out, cursor::Hide).unwrap();
                                    state = AppState::Menu(1);
                                }
                                KeyCode::Esc => state = AppState::Menu(1),
                                _ => {}
                            }
                        }
                        AppState::RemoveInput(q) => {
                            match key.code {
                                KeyCode::Char(c) => q.push(c),
                                KeyCode::Backspace => {
                                    if q.is_empty() { state = AppState::Menu(2); } else { q.pop(); }
                                }
                                KeyCode::Enter => {
                                    execute!(out, ResetColor, Clear(ClearType::All), cursor::MoveTo(0,0), cursor::Show).unwrap();
                                    disable_raw_mode().unwrap();
                                    execute_cli(cli::Commands::Remove { package: Some(q.clone()), noconfirm: false, all: false }).await;
                                    println!("\nНажмите Enter чтобы продолжить...");
                                    let mut buf = String::new();
                                    std::io::stdin().read_line(&mut buf).unwrap();
                                    enable_raw_mode().unwrap();
                                    execute!(out, cursor::Hide).unwrap();
                                    state = AppState::Menu(2);
                                }
                                KeyCode::Esc => state = AppState::Menu(2),
                                _ => {}
                            }
                        }
                        AppState::InfoInput(q) => {
                            match key.code {
                                KeyCode::Char(c) => q.push(c),
                                KeyCode::Backspace => {
                                    if q.is_empty() { state = AppState::Menu(5); } else { q.pop(); }
                                }
                                KeyCode::Enter => {
                                    execute!(out, ResetColor, Clear(ClearType::All), cursor::MoveTo(0,0), cursor::Show).unwrap();
                                    disable_raw_mode().unwrap();
                                    execute_cli(cli::Commands::Info { package: q.clone() }).await;
                                    println!("\nНажмите Enter чтобы продолжить...");
                                    let mut buf = String::new();
                                    std::io::stdin().read_line(&mut buf).unwrap();
                                    enable_raw_mode().unwrap();
                                    execute!(out, cursor::Hide).unwrap();
                                    state = AppState::Menu(5);
                                }
                                KeyCode::Esc => state = AppState::Menu(5),
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}