mod cli;
mod models;
mod search;
mod install;
mod commands;

use clap::Parser;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use std::collections::HashMap;
use std::io::{stdout, Write};

// Защита терминала: гарантирует отключение raw_mode при панике или Ctrl+C
struct RawModeGuard;
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), cursor::Show, ResetColor);
    }
}

#[tokio::main]
async fn main() {
    let cli_args = cli::Cli::parse();

    match cli_args.command {
        cli::Commands::Search { query, aur, pacman, flatpak } => {
            let search_all = !aur && !pacman && !flatpak;
            println!("🔍 Поиск '{}'...", query);

            // ПАРАЛЛЕЛИЗМ: скачиваем данные из всех источников
            let (mut aur_res, mut pacman_res, mut flatpak_res) = (vec![], vec![], vec![]);
            
            if search_all || aur { aur_res = search::search_aur(&query).await; }
            if search_all || pacman { pacman_res = search::search_pacman(&query).await; }
            if search_all || flatpak { flatpak_res = search::search_flatpak(&query).await; }

            // ДЕДУПЛИКАЦИЯ: Приоритет Aur > Pacman > Flatpak
            let mut deduplicated: HashMap<String, models::PackageResult> = HashMap::new();
            for pkg in aur_res.into_iter().chain(pacman_res).chain(flatpak_res) {
                deduplicated.entry(pkg.name.clone())
                    .and_modify(|e| { if pkg.source < e.source { *e = pkg.clone(); } })
                    .or_insert(pkg);
            }

            let mut final_list: Vec<_> = deduplicated.values().cloned().collect();
            final_list.sort_by(|a, b| a.name.cmp(&b.name));

            if final_list.is_empty() {
                println!("❌ Ничего не найдено.");
                return;
            }

            // Вызов интерактивного TUI-меню
            if let Some(selected_pkg) = run_tui_selection(&final_list) {
                install::smart_install(&selected_pkg.name, false, Some(&selected_pkg.source)).await;
            }
        }
        cli::Commands::Install { package, noconfirm } => {
            install::smart_install(&package, noconfirm, None).await;
        }
        cli::Commands::Remove { package, noconfirm } => {
            commands::remove_package(&package, noconfirm).await;
        }
        cli::Commands::Update { noconfirm } => {
            commands::update_system(noconfirm).await;
        }
        cli::Commands::Clean { noconfirm } => {
            commands::clean_system(noconfirm).await;
        }
    }
}

// Плавный скроллинг (Viewport) без лагов и морганий
fn run_tui_selection(packages: &[models::PackageResult]) -> Option<models::PackageResult> {
    enable_raw_mode().unwrap();
    let _guard = RawModeGuard;

    let mut out = stdout();
    execute!(out, cursor::Hide, Clear(ClearType::All)).unwrap();

    let mut selected = 0;
    let mut scroll_offset = 0;
    let mut result = None;

    loop {
        let (term_width, term_height) = crossterm::terminal::size().unwrap_or((80, 24));
        
        // Вычисляем, сколько элементов влезает на экран (каждый занимает 2 строки + заголовок)
        let display_limit = (term_height as usize).saturating_sub(4) / 2;
        let display_limit = std::cmp::max(1, display_limit);

        // Логика скролла окна
        if selected < scroll_offset { scroll_offset = selected; }
        if selected >= scroll_offset + display_limit { scroll_offset = selected - display_limit + 1; }

        queue!(out, cursor::MoveTo(0, 0)).unwrap();
        queue!(out, Print(format!(" === Найдено пакетов: {} ===\r\n", packages.len()))).unwrap();
        queue!(out, Print(" [Tab/Стрелки] Навигация | [Enter] Установить | [Esc] Отмена\r\n\n")).unwrap();

        let end_idx = std::cmp::min(scroll_offset + display_limit, packages.len());

        for i in scroll_offset..end_idx {
            let pkg = &packages[i];
            let is_selected = i == selected;
            
            let tag = match pkg.source {
                models::PackageSource::Aur => "[AUR]    ",
                models::PackageSource::Pacman => "[PACMAN] ",
                models::PackageSource::Flatpak => "[FLATPAK]",
            };

            if is_selected {
                queue!(out, SetForegroundColor(Color::Black), SetBackgroundColor(Color::White)).unwrap();
            }

            let title = format!("  {} {} v{} ", tag, pkg.name, pkg.version);
            let desc = format!("         {} ", pkg.description);
            
            // Защита от разъезжания строк по ширине терминала
            let safe_title = if title.chars().count() > term_width as usize { 
                title.chars().take(term_width as usize - 3).collect::<String>() + "..." 
            } else { 
                title 
            };
            
            let safe_desc = if desc.chars().count() > term_width as usize { 
                desc.chars().take(term_width as usize - 3).collect::<String>() + "..." 
            } else { 
                desc 
            };

            queue!(out, Print(format!("{}\r\n{}\r\n", safe_title, safe_desc))).unwrap();
            
            if is_selected {
                queue!(out, ResetColor).unwrap();
            }
        }

        queue!(out, Clear(ClearType::FromCursorDown)).unwrap();
        out.flush().unwrap(); // Отрисовка буфера на экран за 1 кадр

        // Обработка клавиш
        if let Event::Key(key) = event::read().unwrap() {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Tab | KeyCode::Down => {
                        if selected < packages.len() - 1 { selected += 1; }
                    }
                    KeyCode::BackTab | KeyCode::Up => {
                        if selected > 0 { selected -= 1; }
                    }
                    KeyCode::Enter => { 
                        result = Some(packages[selected].clone()); 
                        break; 
                    }
                    KeyCode::Esc => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    _ => {}
                }
            }
        }
    }

    // Возвращаем терминал в исходное состояние
    execute!(out, ResetColor, Clear(ClearType::All), cursor::MoveTo(0,0)).unwrap();
    result
}
