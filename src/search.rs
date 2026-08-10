use crate::models::{PackageResult, PackageSource};
use serde::Deserialize;
use tokio::process::Command;

pub const GOS_REPO_PACKAGES_URL: &str =
    "https://raw.githubusercontent.com/AdrescorGiti/gvalli-repo/main/packages.json";

#[derive(Deserialize, Debug, Clone)]
pub struct GosPackage {
    pub name: String,
    pub version: String,
    pub description: String,
    pub creator: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub url: String,
    #[serde(default)]
    pub sha256: String,
}

#[derive(Deserialize, Debug)]
struct GosRepoRoot {
    #[serde(default)]
    packages: Vec<GosPackage>,
}

static REPO_CACHE: std::sync::Mutex<Option<Vec<GosPackage>>> = std::sync::Mutex::new(None);

async fn fetch_gos_repo() -> Vec<GosPackage> {
    let Ok(resp) = reqwest::get(GOS_REPO_PACKAGES_URL).await else { return vec![] };
    let Ok(text) = resp.text().await else { return vec![] };
    let root: GosRepoRoot = match serde_json::from_str(&text) {
        Ok(root) => root,
        Err(_) => return vec![],
    };
    root.packages
}

async fn load_gos_repo() -> Vec<GosPackage> {
    {
        let guard = REPO_CACHE.lock().unwrap();
        if let Some(cached) = guard.as_ref() {
            return cached.clone();
        }
    }

    let repo = fetch_gos_repo().await;
    *REPO_CACHE.lock().unwrap() = Some(repo.clone());
    repo
}

pub async fn gos_packages() -> Vec<GosPackage> {
    load_gos_repo().await
}

pub async fn search_gos(query: &str) -> Vec<PackageResult> {
    let q_low = query.trim().to_lowercase();
    load_gos_repo()
        .await
        .into_iter()
        .filter(|p| {
            p.name.to_lowercase().contains(&q_low)
                || p.description.to_lowercase().contains(&q_low)
        })
        .map(|p| PackageResult {
            name: p.name,
            version: p.version,
            description: p.description,
            source: PackageSource::Gos,
        })
        .collect()
}

pub async fn get_gos_package(name: &str) -> Option<GosPackage> {
    load_gos_repo()
        .await
        .into_iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
}

pub async fn check_gos_reachable() -> bool {
    reqwest::get(GOS_REPO_PACKAGES_URL)
        .await
        .map(|r| r.status().is_success() || r.status().is_client_error())
        .unwrap_or(false)
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

pub async fn get_gos_info(package: &str) -> Option<GosPackage> {
    get_gos_package(package).await
}