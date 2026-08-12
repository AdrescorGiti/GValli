use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "GValli", version = "0.6.0", about = "Native Package Manager for G OS")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(visible_alias = "s")]
    Search { 
        query: String,
        #[arg(long)] json: bool,
    },

    #[command(visible_alias = "i")]
    Install { 
        package: String,
    },

    #[command(visible_alias = "r")]
    Remove { 
        package: Option<String>,
        #[arg(long)] all: bool,
    },

    #[command(visible_alias = "u")]
    Update,

    #[command(visible_alias = "c")]
    Clean {
        #[arg(long)] all: bool,
    },

    #[command(visible_alias = "in")]
    Info { package: String },

    #[command(visible_alias = "l")]
    List,

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

#[derive(Subcommand, Clone)]
pub enum GpkgCommands {
    Create {
        #[arg(value_name = "PATH")]
        path: Option<String>,
        #[arg(short = 'i', long = "install")]
        install: bool,
    },
    Install { 
        target: String 
    },
    Get { 
        url: String 
    },
    Extract {
        target: String,
        #[arg(short, long)]
        dest: Option<String>,
    },
    Inspect {
        package: String,
    },
    Verify,
    Uninstall {
        package: String,
    },
}