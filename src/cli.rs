use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "GValli", version = "0.5.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(visible_alias = "s", visible_alias = "S")]
    Search { 
        query: String,
        #[arg(short, long)] pacman: bool,
        #[arg(short, long)] flatpak: bool,
        #[arg(long)] json: bool,
    },

    Create {
        #[arg(value_name = "PATH")]
        path: Option<String>,
        #[arg(short = 's', long = "sync")]
        sync: bool,
        #[arg(short = 'i', long = "install")]
        install: bool,
    },
    
    #[command(visible_alias = "i", visible_alias = "I")]
    Install { 
        package: String,
        #[arg(long)] noconfirm: bool,
    },
    
    #[command(visible_alias = "r", visible_alias = "R")]
    Remove { 
        package: Option<String>,
        #[arg(long)] all: bool,
        #[arg(long)] noconfirm: bool,
    },
    
    #[command(visible_alias = "u", visible_alias = "U", visible_alias = "Syu")]
    Update {
        #[arg(long)] noconfirm: bool,
        #[arg(long)] gpkg: bool,
    },

    #[command(visible_alias = "c", visible_alias = "C")]
    Clean {
        #[arg(long)] noconfirm: bool,
        #[arg(long)] all: bool,
    },

    #[command(visible_alias = "in")]
    Info { package: String },

    #[command(visible_alias = "l", visible_alias = "L")]
    List {
        #[arg(long)] flatpak: bool,
        #[arg(long)] gpkg: bool,
    },

    #[command(visible_alias = "ar")]
    Autoremove {
        #[arg(long)] noconfirm: bool,
    },

    #[command(visible_alias = "v")]
    Verify,

    #[command(visible_alias = "d")]
    Doctor,

    Run { package: String },

    #[command(visible_alias = "g")]
    Gpkg {
        #[command(subcommand)]
        action: GpkgCommands,
    },
}

#[derive(Subcommand)]
pub enum GpkgCommands {
    Create {
        #[arg(value_name = "PATH")]
        path: Option<String>,
        #[arg(short = 's', long = "sync")]
        sync: bool,
        #[arg(short = 'i', long = "install")]
        install: bool,
    },
    Install { 
        target: String 
    },
    Get { 
        url: String 
    },
}