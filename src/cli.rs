use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "GValli", version = "0.4.1")]
#[command(about = "Ультрабыстрый интерактивный CLI-агрегатор: AUR, Pacman, Flatpak, Gpkg")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Поиск пакетов [Алиасы: s, S]
    #[command(visible_alias = "s", visible_alias = "S")]
    Search { 
        query: String,
        #[arg(short, long)] aur: bool,
        #[arg(short, long)] pacman: bool,
        #[arg(short, long)] flatpak: bool,
    },
    
    /// Установка пакета [Алиасы: i, I]
    #[command(visible_alias = "i", visible_alias = "I")]
    Install { 
        package: String,
        #[arg(long)] noconfirm: bool,
    },
    
    /// Умное удаление пакета [Алиасы: r, R]
    #[command(visible_alias = "r", visible_alias = "R")]
    Remove { 
        package: String,
        #[arg(long)] noconfirm: bool, 
    },
    
    /// Обновление всей системы [Алиасы: u, U, Syu]
    #[command(visible_alias = "u", visible_alias = "U", visible_alias = "Syu")]
    Update {
        #[arg(long)] noconfirm: bool, 
    },

    /// Очистка кэша и мусора [Алиасы: c, C]
    #[command(visible_alias = "c", visible_alias = "C")]
    Clean {
        #[arg(long)] noconfirm: bool, 
    },

    /// Управление пакетами Glavo OS (.gpkg) [Алиасы: g]
    #[command(visible_alias = "g")]
    Gpkg {
        #[command(subcommand)]
        action: GpkgCommands,
    },
}

#[derive(Subcommand)]
pub enum GpkgCommands {
    /// Сборка .gpkg пакета из Rust-проекта
    Create { 
        /// Путь к директории с Cargo.toml
        path: String 
    },
    /// Установка .gpkg пакета (локальный путь или URL)
    Install { 
        /// Файл, путь или URL ссылка
        target: String 
    },
    /// Клонирование с GitHub, авто-сборка и установка
    Get { 
        /// URL репозитория GitHub
        url: String 
    },
}