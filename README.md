# GValli
# 🚀 GValli

**GValli** is an ultra-fast, concurrent CLI package manager aggregator written in Rust. It unifies searching, installing, and managing packages across **AUR**, **Pacman**, and **Flatpak** into a single, intuitive interface.

---

## ✨ Features

- **⚡ Blazing-Fast Concurrency:** Parallel async search via `tokio::join!` (<0.1s response times).
- **🎨 Interactive TUI:** Smooth terminal UI with viewport scrolling and pagination for easy package selection.
- **🛡️ Privilege Sandboxing:** Safe AUR builds. Automatically drops root privileges to `$SUDO_USER` during `makepkg` execution.
- **🔗 Recursive AUR Resolver:** Automatically parses `.SRCINFO` and recursively resolves/builds nested AUR dependencies.
- **🎯 Deduplication & Priorities:** Merges search results automatically prioritized by `[1] AUR > [2] Pacman > [3] Flatpak`.
- **🔍 Smart Routing & Fuzzy Removal:** Automatically detects where packages are installed and provides interactive deletion options.
- **⚡ Short Command Aliases:** Command shortcuts like `s` (search), `i` (install), `r` (remove), `u` (update), and `c` (clean).

---

## ⌨️ Command Usage & Aliases

| Command | Alias | Description |
| :--- | :--- | :--- |
| `gvalli search <query>` | `gvalli s` / `gvalli S` | Interactive parallel search across all repositories with TUI selection |
| `gvalli install <pkg>` | `gvalli i` / `gvalli I` | Smart package installation (auto-detects source or source routing) |
| `gvalli remove <pkg>` | `gvalli r` / `gvalli R` | Fuzzy package search and removal from system |
| `gvalli update` | `gvalli u` / `gvalli U` / `Syu` | Full system update (Pacman repos + Flatpak runtimes) |
| `gvalli clean` | `gvalli c` / `gvalli C` | Cleans Pacman package cache and removes unused Flatpak runtimes |

---

## 🛠️ Installation

### Building from Source (Arch Linux)

1. Clone the repository:
   ```bash
   git clone [https://github.com/yourusername/gvalli.git](https://github.com/yourusername/gvalli.git)
   cd gvalli
