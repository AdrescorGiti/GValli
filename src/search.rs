use crate::models::{PackageResult, PackageSource};
use anyhow::Result;
use serde::Deserialize;
use std::sync::Mutex;

pub const GOS_REPO_PACKAGES_URL: &str =
    "https://raw.githubusercontent.com/AdrescorGiti/gvalli-repo/main/packages.json";

#[derive(Deserialize, Debug, Clone)]
pub struct GosPackage {
    pub name: String,
    pub version: String,
    pub description: String,
    pub creator: String,
    #[allow(dead_code)]
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub url: String,
    #[allow(dead_code)]
    #[serde(default)]
    pub sha256: String,
}

#[derive(Deserialize, Debug)]
struct GosRepoRoot {
    #[serde(default)]
    packages: Vec<GosPackage>,
}

static REPO_CACHE: Mutex<Option<Vec<GosPackage>>> = Mutex::new(None);

async fn fetch_gos_repo() -> Result<Vec<GosPackage>> {
    let client = reqwest::Client::builder().user_agent("GValli").build()?;
    let resp = client.get(GOS_REPO_PACKAGES_URL).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("Repository unreachable");
    }
    let text = resp.text().await?;
    let root: GosRepoRoot = serde_json::from_str(&text)?;
    Ok(root.packages)
}

pub async fn load_gos_repo() -> Vec<GosPackage> {
    {
        let guard = REPO_CACHE.lock().unwrap();
        if let Some(cached) = guard.as_ref() {
            return cached.clone();
        }
    }
    let repo = fetch_gos_repo().await.unwrap_or_default();
    *REPO_CACHE.lock().unwrap() = Some(repo.clone());
    repo
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
            url: Some(p.url),
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
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}