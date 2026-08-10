use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum PackageSource {
    Gos = 1,
    Pacman = 2,
    Flatpak = 3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageResult {
    pub name: String,
    pub version: String,
    pub description: String,
    pub source: PackageSource,
}