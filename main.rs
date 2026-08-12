mod cli;
mod commands;
mod gpkg;
mod models;
mod scanner;
mod search;
mod tui;

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let cli_args = cli::Cli::parse();

    if let Some(command) = cli_args.command {
        match command {
            cli::Commands::Search { query, json } => {
                let res = search::search_gos(&query).await;
                if json {
                    println!("{}", serde_json::to_string_pretty(&res)?);
                } else {
                    for p in res {
                        println!("[{}] {} v{} - {}", p.source.label(), p.name, p.version, p.description);
                    }
                }
            }
            cli::Commands::Install { package } => {
                if let Some(gos) = search::get_gos_package(&package).await {
                    gpkg::install_package(&gos.url).await?;
                } else {
                    eprintln!("Package '{}' not found in G OS repository.", package);
                }
            }
            cli::Commands::Remove { package, all } => {
                if let Some(pkg) = package {
                    gpkg::remove_package(&pkg).await?;
                } else if all {
                    println!("Removing all packages...");
                }
            }
            cli::Commands::Update => {
                commands::update_system().await?;
            }
            cli::Commands::Clean { all } => {
                commands::clean_system(all).await?;
            }
            cli::Commands::Info { package } => {
                commands::package_info(&package).await?;
            }
            cli::Commands::List => {
                commands::list_packages().await?;
            }
            cli::Commands::Verify => {
                gpkg::verify_packages()?;
            }
            cli::Commands::Doctor => {
                commands::doctor().await?;
            }
            cli::Commands::Run { package } => {
                commands::run_gpkg(&package).await?;
            }
            cli::Commands::Gpkg { action } => match action {
                cli::GpkgCommands::Create { path, install } => {
                    let target = path.as_deref().unwrap_or(".");
                    let gpkg_path = gpkg::create_package(target, true).await?;
                    if install {
                        gpkg::install_package(&gpkg_path).await?;
                    }
                }
                cli::GpkgCommands::Install { target } => {
                    gpkg::install_package(&target).await?;
                }
                cli::GpkgCommands::Get { url } => {
                    gpkg::get_package(&url).await?;
                }
                cli::GpkgCommands::Extract { target, dest } => {
                    let dest_path = dest.unwrap_or_else(|| "./extracted".into());
                    gpkg::extract_package(&target, std::path::Path::new(&dest_path))?;
                }
                cli::GpkgCommands::Inspect { package } => {
                    gpkg::inspect_package(&package).await?;
                }
                cli::GpkgCommands::Verify => {
                    gpkg::verify_packages()?;
                }
                cli::GpkgCommands::Uninstall { package } => {
                    gpkg::remove_package(&package).await?;
                }
            },
        }
    } else {
        tui::run_tui().await?;
    }

    Ok(())
}