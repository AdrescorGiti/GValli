use crate::models::{PackageResult, PackageSource};
use serde::Deserialize;
use tokio::process::Command;

#[derive(Deserialize)]
struct AurResponse { results: Vec<AurResult> }

#[derive(Deserialize)]
struct AurResult {
    #[serde(rename = "Name")] name: String,
    #[serde(rename = "Version")] version: String,
    #[serde(rename = "Description")] description: Option<String>,
}

pub async fn search_aur(query: &str) -> Vec<PackageResult> {
    let url = format!("https://aur.archlinux.org/rpc/v5/search/{}", query);
    let Ok(resp) = reqwest::get(&url).await else { return vec![]; };
    let Ok(parsed) = resp.json::<AurResponse>().await else { return vec![]; };
    
    parsed.results.into_iter().map(|r| PackageResult {
        name: r.name, 
        version: r.version,
        description: r.description.unwrap_or_default(),
        source: PackageSource::Aur,
    }).collect()
}

pub async fn search_pacman(query: &str) -> Vec<PackageResult> {
    let Ok(out) = Command::new("pacman").args(["-Ss", query]).output().await else { return vec![]; };
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut results = vec![];
    let mut lines = stdout.lines().peekable();

    while let Some(line) = lines.next() {
        if !line.starts_with(' ') {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].split('/').last().unwrap_or("").to_string();
                let desc = if let Some(next_line) = lines.peek() {
                    if next_line.starts_with("    ") { lines.next().unwrap().trim().to_string() } else { String::new() }
                } else { String::new() };
                results.push(PackageResult { name, version: parts[1].to_string(), description: desc, source: PackageSource::Pacman });
            }
        }
    }
    results
}

pub async fn search_flatpak(query: &str) -> Vec<PackageResult> {
    let Ok(out) = Command::new("flatpak")
        .args(["search", "--columns=app,name,version,description", query])
        .output().await else { return vec![]; };
        
    let mut results = vec![];
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() >= 4 {
            if cols[0] == "Application ID" { continue; }
            results.push(PackageResult {
                name: cols[0].trim().to_string(),
                version: cols[2].trim().to_string(),
                description: cols[1].trim().to_string(),
                source: PackageSource::Flatpak,
            });
        }
    }
    results
}