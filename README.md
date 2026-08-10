# GValli

GValli is a modern, fast, and convenient CLI package manager aggregator for Arch-based systems. It brings together package discovery and package management for AUR, Pacman, and Flatpak in one streamlined experience.

## Why GValli?

- Fast and responsive: parallel asynchronous search and package operations
- Unified workflow: search, install, remove, update, and clean from a single tool
- Safe AUR handling: builds are executed with reduced privileges for better security
- Smart package routing: automatically detects the most appropriate source for each action
- Friendly UX: short aliases and an interactive terminal interface

## Key Features

- ⚡ Parallel package search across multiple backends
- 🧭 Interactive TUI-based package selection
- 🔒 Safer AUR builds with privilege drop during package compilation
- 🔗 Recursive dependency resolution for AUR packages
- 🎯 Result deduplication with clear source priority: AUR > Pacman > Flatpak
- 🧹 Useful maintenance commands for updates and cache cleanup

## Installation

Download the pre-built package from the releases page and install it with pacman:

```bash
sudo pacman -U gvalli-0.1.0-1-x86_64.pkg.tar.zst
```

## Quick Start

```bash
gvalli search <query>
gvalli install <package>
gvalli remove <package>
gvalli update
gvalli clean
```

## Command Reference

| Command | Alias | Description |
| --- | --- | --- |
| `gvalli search <query>` | `gvalli s`, `gvalli S` | Interactive search across supported sources |
| `gvalli install <pkg>` | `gvalli i`, `gvalli I` | Install a package from the best available source |
| `gvalli remove <pkg>` | `gvalli r`, `gvalli R` | Find and remove packages from the system |
| `gvalli update` | `gvalli u`, `gvalli U`, `Syu` | Update Pacman packages and Flatpak runtimes |
| `gvalli clean` | `gvalli c`, `gvalli C` | Clean package caches and remove unused Flatpak data |

## Development

Build the project locally:

```bash
cargo build
```

Run the CLI:

```bash
cargo run -- search firefox
```

## License

This project is distributed under the MIT License.

---

## Русская версия

GValli — это современный и быстрый CLI-агрегатор пакетных менеджеров для систем на базе Arch. Он объединяет поиск, установку, удаление, обновление и очистку пакетов из AUR, Pacman и Flatpak в одном удобном интерфейсе.

### Почему GValli?

- Быстрая работа благодаря параллельному асинхронному поиску
- Единый рабочий процесс для всех основных операций
- Безопасная сборка AUR-пакетов с понижением привилегий
- Умная маршрутизация пакетов между источниками
- Удобные сокращения команд и интерактивный интерфейс

### Быстрый старт

```bash
gvalli search <запрос>
gvalli install <пакет>
gvalli remove <пакет>
gvalli update
gvalli clean
```

### Лицензия

Проект распространяется под лицензией MIT.


```

```

```
