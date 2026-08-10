# GValli

<p align="center">
  <a href="https://github.com/AdrescorGiti/GValli"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.70%2B-orange.svg" alt="Rust" /></a>
  <a href="https://archlinux.org/"><img src="https://img.shields.io/badge/Platform-Arch%20Linux-1793d1.svg" alt="Arch Linux" /></a>
</p>

<div align="center">
  <h3>Fast, intelligent package management for Arch-based systems</h3>
  <p>GValli brings together AUR, Pacman, and Flatpak into one elegant CLI experience for searching, installing, updating, and maintaining software.</p>
</div>

<div align="center">
  <a href="#installation"><button>Install</button></a>
  <a href="#quick-start"><button>Quick Start</button></a>
  <a href="#features"><button>Features</button></a>
</div>

---

## Why GValli?

GValli is designed for users who want a single tool for everyday package management without sacrificing speed, safety, or clarity. It combines intelligent routing, async operations, and a polished terminal interface into a workflow that feels modern and dependable.

<div align="center">
  <table>
    <tr>
      <td valign="top" width="33%">
        <h4>⚡ Fast</h4>
        <p>Parallel asynchronous search and package operations keep the experience quick and responsive.</p>
      </td>
      <td valign="top" width="33%">
        <h4>🛡️ Safe</h4>
        <p>AUR builds run with reduced privileges, improving security during compilation.</p>
      </td>
      <td valign="top" width="33%">
        <h4>🧭 Smart</h4>
        <p>Packages are routed intelligently across AUR, Pacman, and Flatpak based on context.</p>
      </td>
    </tr>
  </table>
</div>

---

## Features

<div align="center">
  <table>
    <tr>
      <td valign="top" width="50%">
        <h4>🔍 Unified Search</h4>
        <p>Search across supported sources from a single command with a streamlined interactive flow.</p>
      </td>
      <td valign="top" width="50%">
        <h4>🧰 Rich Maintenance Commands</h4>
        <p>Install, remove, update, and clean packages without switching tools or contexts.</p>
      </td>
    </tr>
    <tr>
      <td valign="top" width="50%">
        <h4>🔗 Recursive AUR Resolution</h4>
        <p>Automatically resolves nested AUR dependencies and handles the build chain more effectively.</p>
      </td>
      <td valign="top" width="50%">
        <h4>🎯 Priority-Based Results</h4>
        <p>Results are deduplicated and prioritized with a clear source order: AUR &gt; Pacman &gt; Flatpak.</p>
      </td>
    </tr>
  </table>
</div>

---

## Installation

Download the pre-built package from the releases page and install it with pacman:

```bash
sudo pacman -U gvalli-0.1.0-1-x86_64.pkg.tar.zst
```

---

## Quick Start

```bash
gvalli search <query>
gvalli install <package>
gvalli remove <package>
gvalli update
gvalli clean
```

---

## Command Reference

| Command | Alias | Description |
| --- | --- | --- |
| `gvalli search <query>` | `gvalli s`, `gvalli S` | Interactive search across supported sources |
| `gvalli install <pkg>` | `gvalli i`, `gvalli I` | Install a package from the best available source |
| `gvalli remove <pkg>` | `gvalli r`, `gvalli R` | Find and remove packages from the system |
| `gvalli update` | `gvalli u`, `gvalli U`, `Syu` | Update Pacman packages and Flatpak runtimes |
| `gvalli clean` | `gvalli c`, `gvalli C` | Clean package caches and remove unused Flatpak data |

---

## Development

Build the project locally:

```bash
cargo build
```

Run the CLI:

```bash
cargo run -- search firefox
```

---

## License

This project is distributed under the MIT License.

---

## Русская версия

GValli — это современный и быстрый CLI-агрегатор для систем на базе Arch. Он объединяет AUR, Pacman и Flatpak в одном удобном интерфейсе для поиска, установки, обновления и обслуживания программ.

<div align="center">
  <a href="#installation"><button>Установка</button></a>
  <a href="#quick-start"><button>Быстрый старт</button></a>
</div>

### Ключевые возможности

- ⚡ Быстрый и отзывчивый интерфейс
- 🛡️ Более безопасная сборка AUR-пакетов
- 🧭 Умная маршрутизация между источниками пакетов
- 🧹 Удобные команды для обновления и очистки системы

### Лицензия

Проект распространяется под лицензией MIT.

