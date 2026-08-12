use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PackageSource {
    Gos = 1,
    LocalGpkg = 2,
}

impl PackageSource {
    pub fn label(&self) -> &'static str {
        match self {
            PackageSource::Gos => "G OS Repo",
            PackageSource::LocalGpkg => "Local .gpkg",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageResult {
    pub name: String,
    pub version: String,
    pub description: String,
    pub source: PackageSource,
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub maintainer: String,
    #[serde(default)]
    pub maintainer_email: String,
    #[serde(default)]
    pub github_repo: String,
    pub exec_binary: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GpkgEntry {
    pub version: String,
    pub files: Vec<String>,
    #[serde(default)]
    pub checksums: HashMap<String, String>,
    #[serde(default)]
    pub github_repo: Option<String>,
    #[serde(default)]
    pub exec_binary: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct GpkgDatabase {
    pub packages: HashMap<String, GpkgEntry>,
}