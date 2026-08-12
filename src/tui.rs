use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::io;
use std::path::PathBuf;
use tokio::sync::mpsc;

use crate::commands;
use crate::gpkg;
use crate::models::{GpkgEntry, PackageResult};
use crate::scanner::Scanner;
use crate::search;

pub enum TuiState {
    MainMenu,
    GpkgMenu,
    SearchRemote(String, Vec<PackageResult>, ListState),
    LocalScan(Vec<PathBuf>, ListState, mpsc::UnboundedReceiver<PathBuf>),
    InputPrompt {
        title: String,
        input: String,
        action: PromptAction,
    },
    RemoveMenu {
        query: String,
        packages: Vec<(String, GpkgEntry)>,
        list_state: ListState,
    },
}

pub enum PromptAction {
    CreatePackage,
    ExtractArchive,
    GetRemote,
    InspectPackage,
}

fn filter_packages(query: &str) -> Vec<(String, GpkgEntry)> {
    let db = gpkg::load_db();
    let q = query.to_lowercase();
    let mut pkgs: Vec<_> = db.packages
        .into_iter()
        .filter(|(name, _)| name.to_lowercase().contains(&q))
        .collect();
    pkgs.sort_by(|a, b| a.0.cmp(&b.0));
    pkgs
}

pub async fn run_tui() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = TuiState::MainMenu;
    let mut main_menu_state = ListState::default();
    main_menu_state.select(Some(0));

    let mut gpkg_menu_state = ListState::default();
    gpkg_menu_state.select(Some(0));

    let main_items = [
        "Search & Install (G OS Repo)",
        "Update System Packages",
        "Uninstall / Remove Packages", // Moved to Main Menu
        "GPKG Native Manager",
        "List Installed Packages",
        "System Health Doctor",
        "Exit",
    ];

    let gpkg_items = [
        "Install .gpkg (System Scan)",
        "Create .gpkg Package",
        "Inspect Package Archive/Info",
        "Extract .gpkg Package",
        "Verify Checksums / Integrity",
        "Get Source from Git",
        "Back to Main Menu",
    ];

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Length(3), Constraint::Min(5), Constraint::Length(3)].as_ref())
                .split(f.size());

            let header = Paragraph::new(" GValli - Native G OS Package Manager & GPKG Toolkit ")
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, chunks[0]);

            match &mut state {
                TuiState::MainMenu => {
                    let items: Vec<ListItem> = main_items.iter().map(|i| ListItem::new(*i)).collect();
                    let list = List::new(items)
                        .block(Block::default().title(" Main Menu ").borders(Borders::ALL))
                        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD))
                        .highlight_symbol(">> ");
                    f.render_stateful_widget(list, chunks[1], &mut main_menu_state);
                }
                TuiState::GpkgMenu => {
                    let items: Vec<ListItem> = gpkg_items.iter().map(|i| ListItem::new(*i)).collect();
                    let list = List::new(items)
                        .block(Block::default().title(" Dedicated GPKG Toolkit Sub-Menu ").borders(Borders::ALL))
                        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD))
                        .highlight_symbol(">> ");
                    f.render_stateful_widget(list, chunks[1], &mut gpkg_menu_state);
                }
                TuiState::SearchRemote(query, results, list_state) => {
                    let mut items = vec![ListItem::new(format!("Search Query: {}", query)).style(Style::default().fg(Color::Yellow))];
                    for p in results {
                        items.push(ListItem::new(format!("{} v{} - {}", p.name, p.version, p.description)));
                    }
                    let list = List::new(items)
                        .block(Block::default().title(" G OS Repo Search & Install ").borders(Borders::ALL))
                        .highlight_style(Style::default().fg(Color::Black).bg(Color::LightMagenta).add_modifier(Modifier::BOLD))
                        .highlight_symbol("> ");
                    f.render_stateful_widget(list, chunks[1], list_state);
                }
                TuiState::RemoveMenu { query, packages, list_state } => {
                    let layout = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(chunks[1]);

                    let mut items = vec![ListItem::new(format!("🔍 Search Installed: {}", query)).style(Style::default().fg(Color::Yellow))];
                    for (name, entry) in packages.iter() {
                        items.push(ListItem::new(format!("{} v{}", name, entry.version)));
                    }
                    
                    let list = List::new(items)
                        .block(Block::default().title(" Remove / Uninstall Package ").borders(Borders::ALL))
                        .highlight_style(Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD))
                        .highlight_symbol("X ");
                    f.render_stateful_widget(list, layout[0], list_state);

                    let details_text = if let Some(idx) = list_state.selected() {
                        if idx > 0 && idx - 1 < packages.len() {
                            let (name, entry) = &packages[idx - 1];
                            format!(
                                "Package Name : {}\nVersion      : {}\nExecutable   : {}\nFiles Deployed: {}\nRepository   : {}\n\n[WARNING] Press [Enter] to permanently uninstall.",
                                name,
                                entry.version,
                                entry.exec_binary.as_deref().unwrap_or("None"),
                                entry.files.len(),
                                entry.github_repo.as_deref().unwrap_or("None")
                            )
                        } else {
                            "Select a package to view metadata...".into()
                        }
                    } else {
                        "Select a package to view metadata...".into()
                    };

                    let details = Paragraph::new(details_text)
                        .block(Block::default().title(" Package Details ").borders(Borders::ALL));
                    f.render_widget(details, layout[1]);
                }
                TuiState::LocalScan(files, list_state, rx) => {
                    while let Ok(path) = rx.try_recv() {
                        files.push(path);
                    }
                    let items: Vec<ListItem> = files.iter().map(|p| ListItem::new(p.to_string_lossy().to_string())).collect();
                    let list = List::new(items)
                        .block(Block::default().title(format!(" System-wide .gpkg Live Scanner (Discovered: {}) ", files.len())).borders(Borders::ALL))
                        .highlight_style(Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD))
                        .highlight_symbol(">> ");
                    f.render_stateful_widget(list, chunks[1], list_state);
                }
                TuiState::InputPrompt { title, input, action: _ } => {
                    let prompt = Paragraph::new(format!("{}: {}\n\nPress [Enter] to submit, [Esc] to cancel", title, input))
                        .block(Block::default().title(" Action Prompt ").borders(Borders::ALL))
                        .style(Style::default().fg(Color::Yellow));
                    f.render_widget(prompt, chunks[1]);
                }
            }

            let footer = Paragraph::new(" Navigation: [Up/Down] | Select: [Enter] | Back: [Esc/Backspace] ")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match &mut state {
                    TuiState::MainMenu => match key.code {
                        KeyCode::Down => {
                            let i = match main_menu_state.selected() {
                                Some(i) => if i >= main_items.len() - 1 { 0 } else { i + 1 },
                                None => 0,
                            };
                            main_menu_state.select(Some(i));
                        }
                        KeyCode::Up => {
                            let i = match main_menu_state.selected() {
                                Some(i) => if i == 0 { main_items.len() - 1 } else { i - 1 },
                                None => 0,
                            };
                            main_menu_state.select(Some(i));
                        }
                        KeyCode::Enter => match main_menu_state.selected().unwrap_or(0) {
                            0 => state = TuiState::SearchRemote(String::new(), vec![], ListState::default()),
                            1 => {
                                drop_terminal(&mut terminal)?;
                                if !gpkg::is_root() {
                                    let current = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("gvalli"));
                                    let _ = std::process::Command::new("sudo").arg(current).arg("update").status();
                                } else {
                                    let _ = commands::update_system().await;
                                }
                                restore_terminal(&mut terminal)?;
                            }
                            2 => {
                                let mut ls = ListState::default();
                                ls.select(Some(0));
                                state = TuiState::RemoveMenu { query: String::new(), packages: filter_packages(""), list_state: ls };
                            }
                            3 => state = TuiState::GpkgMenu,
                            4 => {
                                drop_terminal(&mut terminal)?;
                                let _ = commands::list_packages().await;
                                restore_terminal(&mut terminal)?;
                            }
                            5 => {
                                drop_terminal(&mut terminal)?;
                                let _ = commands::doctor().await;
                                restore_terminal(&mut terminal)?;
                            }
                            6 => break,
                            _ => {}
                        },
                        KeyCode::Esc => break,
                        _ => {}
                    },
                    TuiState::GpkgMenu => match key.code {
                        KeyCode::Down => {
                            let i = match gpkg_menu_state.selected() {
                                Some(i) => if i >= gpkg_items.len() - 1 { 0 } else { i + 1 },
                                None => 0,
                            };
                            gpkg_menu_state.select(Some(i));
                        }
                        KeyCode::Up => {
                            let i = match gpkg_menu_state.selected() {
                                Some(i) => if i == 0 { gpkg_items.len() - 1 } else { i - 1 },
                                None => 0,
                            };
                            gpkg_menu_state.select(Some(i));
                        }
                        KeyCode::Enter => match gpkg_menu_state.selected().unwrap_or(0) {
                            0 => {
                                let (tx, rx) = mpsc::unbounded_channel();
                                Scanner::start_scan(tx);
                                let mut ls = ListState::default();
                                ls.select(Some(0));
                                state = TuiState::LocalScan(vec![], ls, rx);
                            }
                            1 => state = TuiState::InputPrompt { title: "Enter directory path to build .gpkg".into(), input: ".".into(), action: PromptAction::CreatePackage },
                            2 => state = TuiState::InputPrompt { title: "Enter .gpkg file or package name".into(), input: "".into(), action: PromptAction::InspectPackage },
                            3 => state = TuiState::InputPrompt { title: "Enter .gpkg file to extract".into(), input: "".into(), action: PromptAction::ExtractArchive },
                            4 => {
                                drop_terminal(&mut terminal)?;
                                let _ = gpkg::verify_packages();
                                restore_terminal(&mut terminal)?;
                            }
                            5 => state = TuiState::InputPrompt { title: "Enter Git repository URL".into(), input: "".into(), action: PromptAction::GetRemote },
                            6 => state = TuiState::MainMenu,
                            _ => {}
                        },
                        KeyCode::Esc | KeyCode::Backspace => state = TuiState::MainMenu,
                        _ => {}
                    },
                    TuiState::RemoveMenu { query, packages, list_state } => match key.code {
                        KeyCode::Char(c) => {
                            query.push(c);
                            *packages = filter_packages(query);
                        }
                        KeyCode::Backspace => {
                            query.pop();
                            *packages = filter_packages(query);
                        }
                        KeyCode::Down => {
                            let i = match list_state.selected() {
                                Some(i) => if i >= packages.len() { 0 } else { i + 1 },
                                None => 0,
                            };
                            list_state.select(Some(i));
                        }
                        KeyCode::Up => {
                            let i = match list_state.selected() {
                                Some(i) => if i == 0 { packages.len() } else { i - 1 },
                                None => 0,
                            };
                            list_state.select(Some(i));
                        }
                        KeyCode::Enter => {
                            if let Some(idx) = list_state.selected() {
                                if idx > 0 && idx - 1 < packages.len() {
                                    let (target, _) = packages[idx - 1].clone();
                                    drop_terminal(&mut terminal)?;
                                    
                                    if !gpkg::is_root() {
                                        let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("gvalli"));
                                        let _ = std::process::Command::new("sudo")
                                            .arg(current_exe)
                                            .args(["remove", &target])
                                            .status();
                                    } else {
                                        let _ = gpkg::remove_package(&target).await;
                                    }
                                    
                                    restore_terminal(&mut terminal)?;
                                    *packages = filter_packages(query);
                                    list_state.select(Some(0));
                                }
                            }
                        }
                        KeyCode::Esc => state = TuiState::MainMenu,
                        _ => {}
                    },
                    TuiState::SearchRemote(query, results, list_state) => match key.code {
                        KeyCode::Char(c) => {
                            query.push(c);
                            *results = search::search_gos(query).await;
                        }
                        KeyCode::Backspace => {
                            query.pop();
                            if query.is_empty() {
                                *results = vec![];
                            } else {
                                *results = search::search_gos(query).await;
                            }
                        }
                        KeyCode::Down => {
                            let i = match list_state.selected() {
                                Some(i) => if i >= results.len() { 0 } else { i + 1 },
                                None => 0,
                            };
                            list_state.select(Some(i));
                        }
                        KeyCode::Up => {
                            let i = match list_state.selected() {
                                Some(i) => if i == 0 { results.len() } else { i - 1 },
                                None => 0,
                            };
                            list_state.select(Some(i));
                        }
                        KeyCode::Enter => {
                            if let Some(idx) = list_state.selected() {
                                if idx > 0 && idx <= results.len() {
                                    if let Some(url) = &results[idx - 1].url {
                                        drop_terminal(&mut terminal)?;
                                        if !gpkg::is_root() {
                                            let current = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("gvalli"));
                                            let _ = std::process::Command::new("sudo").arg(current).args(["install", url]).status();
                                        } else {
                                            let _ = gpkg::install_package(url).await;
                                        }
                                        restore_terminal(&mut terminal)?;
                                    }
                                }
                            }
                        }
                        KeyCode::Esc => state = TuiState::MainMenu,
                        _ => {}
                    },
                    TuiState::LocalScan(files, list_state, _) => match key.code {
                        KeyCode::Down => {
                            let i = match list_state.selected() {
                                Some(i) => if files.is_empty() { 0 } else if i >= files.len() - 1 { 0 } else { i + 1 },
                                None => 0,
                            };
                            list_state.select(Some(i));
                        }
                        KeyCode::Up => {
                            let i = match list_state.selected() {
                                Some(i) => if files.is_empty() { 0 } else if i == 0 { files.len() - 1 } else { i - 1 },
                                None => 0,
                            };
                            list_state.select(Some(i));
                        }
                        KeyCode::Enter => {
                            if let Some(idx) = list_state.selected() {
                                if idx < files.len() {
                                    let path = files[idx].to_string_lossy().to_string();
                                    drop_terminal(&mut terminal)?;
                                    if !gpkg::is_root() {
                                        let current = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("gvalli"));
                                        let _ = std::process::Command::new("sudo").arg(current).args(["gpkg", "install", &path]).status();
                                    } else {
                                        let _ = gpkg::install_package(&path).await;
                                    }
                                    restore_terminal(&mut terminal)?;
                                }
                            }
                        }
                        KeyCode::Esc | KeyCode::Backspace => state = TuiState::GpkgMenu,
                        _ => {}
                    },
                    TuiState::InputPrompt { title: _, input, action } => match key.code {
                        KeyCode::Char(c) => input.push(c),
                        KeyCode::Backspace => { input.pop(); },
                        KeyCode::Enter => {
                            let target = input.clone();
                            drop_terminal(&mut terminal)?;
                            match action {
                                PromptAction::CreatePackage => {
                                    let _ = gpkg::create_package(&target, true).await;
                                }
                                PromptAction::ExtractArchive => {
                                    let _ = gpkg::extract_package(&target, std::path::Path::new("./extracted"));
                                }
                                PromptAction::GetRemote => {
                                    let _ = gpkg::get_package(&target).await;
                                }
                                PromptAction::InspectPackage => {
                                    let _ = gpkg::inspect_package(&target).await;
                                }
                            }
                            restore_terminal(&mut terminal)?;
                            state = TuiState::GpkgMenu;
                        }
                        KeyCode::Esc => state = TuiState::GpkgMenu,
                        _ => {}
                    }
                }
            }
        }
    }

    drop_terminal(&mut terminal)?;
    Ok(())
}

fn drop_terminal(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> anyhow::Result<()> {
    println!("\nPress Enter to return to TUI...");
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf)?;
    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
    terminal.clear()?;
    Ok(())
}