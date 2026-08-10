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
  <a href="#русская-версия"><button>Русская версия</button></a>
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
        <p>Package builds and system operations run with reduced privileges where appropriate, improving safety.</p>
      </td>
      <td valign="top" width="33%">
        <h4>🧭 Smart</h4>
        <p>Packages are routed intelligently across Pacman and Flatpak based on context.</p>
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
        <h4>🔗 Flexible Dependency Handling</h4>
        <p>Handles package dependency flows in a consistent way across supported package sources.</p>
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

GValli — это современный и быстрый CLI-агрегатор для систем на базе Arch. Он объединяет Pacman и Flatpak в одном удобном интерфейсе для поиска, установки, обновления и обслуживания программ, делая повседневную работу с пакетами проще и быстрее.

<div align="center">
  <a href="#installation"><button>Установка</button></a>
  <a href="#quick-start"><button>Быстрый старт</button></a>
  <a href="#command-reference"><button>Команды</button></a>
</div>

### Почему GValli?

GValli создан для тех, кто хочет управлять пакетами без лишней сложности: без постоянного переключения между разными утилитами, без лишних шагов и без ощущения устаревшего интерфейса. Проект сочетает скорость, понятный UX и аккуратную работу с основными сценариями пакетного менеджмента.

### Ключевые возможности

- ⚡ Быстрый и отзывчивый интерфейс
- 🧭 Умная маршрутизация между пакетными источниками
- 🧹 Удобные команды для обновления и очистки системы
- 🔒 Безопасное поведение при системных операциях и сборке зависимостей
- 🌐 Поддержка современного рабочего потока для Arch-совместимых систем

### Быстрый старт

```bash
gvalli search <запрос>
gvalli install <пакет>
gvalli remove <пакет>
gvalli update
gvalli clean
```

### Команды

| Команда | Алиас | Описание |
| --- | --- | --- |
| `gvalli search <запрос>` | `gvalli s`, `gvalli S` | Поиск по доступным пакетным источникам |
| `gvalli install <пакет>` | `gvalli i`, `gvalli I` | Установка пакета из подходящего источника |
| `gvalli remove <пакет>` | `gvalli r`, `gvalli R` | Поиск и удаление пакета из системы |
| `gvalli update` | `gvalli u`, `gvalli U`, `Syu` | Обновление пакетов и окружения |
| `gvalli clean` | `gvalli c`, `gvalli C` | Очистка кэша и неиспользуемых данных |

### Лицензия

Проект распространяется под лицензией MIT.

