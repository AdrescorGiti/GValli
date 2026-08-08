#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PackageSource {
    Aur = 1,
    Pacman = 2,
    Flatpak = 3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageResult {
    pub name: String,
    pub version: String,
    pub description: String,
    pub source: PackageSource,
}