# GValli

<p align="center">
  <a href="https://github.com/AdrescorGiti/GValli"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.70%2B-orange.svg" alt="Rust" /></a>
  <a href="https://archlinux.org/"><img src="https://img.shields.io/badge/Platform-Arch%20Linux-1793d1.svg" alt="Arch Linux" /></a>
  <a href="https://github.com/AdrescorGiti/gvalli-repo"><img src="https://img.shields.io/badge/Repo-gvalli-repo-8a2be2.svg" alt="Gvalli repository" /></a>
  <a href="#русская-версия"><img src="https://img.shields.io/badge/README-Русская%20версия-ff69b4.svg" alt="Русская версия" /></a>
</p>

<div align="center">
  <h3>Fast, intelligent package management for Arch-based systems</h3>
  <p>GValli brings together AUR, Pacman, and Flatpak into one elegant CLI experience for searching, installing, updating, and maintaining software.</p>
</div>

<div align="center">
  <a href="#installation"><button>Install</button></a>
  <a href="#quick-start"><button>Quick Start</button></a>
  <a href="#features"><button>Features</button></a>
  <a href="https://github.com/AdrescorGiti/gvalli-repo"><button>Gvalli Repo</button></a>
  <a href="#русская-версия"><button>Русская версия</button></a>
</div>

---

## Why GValli?

GValli is designed for users who want a single tool for everyday package management without sacrificing speed, safety, or clarity. It combines intelligent routing, async operations, and a polished terminal interface into a workflow that feels modern and dependable. The project is built around the custom G OS ecosystem, where packages are distributed through the dedicated Gvalli repository in .gpkg format.

At its core, GValli is more than a package manager wrapper. It is a practical infrastructure layer for a self-contained operating system experience: package discovery, dependency awareness, installation flow, and maintenance tasks are unified under one consistent interface.

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

GValli is built to simplify the most common package-management workflows while staying flexible enough for a custom Linux environment. The command-line interface is intentionally compact, but the underlying behavior is designed to be reliable and consistent for both daily use and system maintenance.

<div align="center">
  <table>
    <tr>
      <td valign="top" width="50%">
        <h4>🔍 Unified Search</h4>
        <p>Search across supported package sources from a single command with a streamlined interactive flow.</p>
      </td>
      <td valign="top" width="50%">
        <h4>🧰 Rich Maintenance Commands</h4>
        <p>Install, remove, update, and clean packages without switching tools or contexts, including packages from the custom Gvalli repository.</p>
      </td>
    </tr>
    <tr>
      <td valign="top" width="50%">
        <h4>🔗 Flexible Dependency Handling</h4>
        <p>Handles package dependency flows in a consistent way across supported package sources.</p>
      </td>
      <td valign="top" width="50%">
        <h4>🎯 Priority-Based Results</h4>
        <p>Results are deduplicated and prioritized with a clear source order: Pacman and Flatpak, with support for the custom Gvalli repository workflow.</p>
      </td>
    </tr>
    <tr>
      <td valign="top" width="50%">
        <h4>🧠 Workflow-Oriented Design</h4>
        <p>The tool is structured around real user tasks such as discovery, installation, updates, and cleanup rather than exposing unnecessary low-level details.</p>
      </td>
      <td valign="top" width="50%">
        <h4>🛠️ Extensible Architecture</h4>
        <p>The project is designed to grow with the ecosystem around it, including future support for more custom package formats and repository logic.</p>
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

For the full G OS experience, you can also explore the dedicated package repository:

- GitHub repository: [gvalli-repo](https://github.com/AdrescorGiti/gvalli-repo)
- Package format: `.gpkg` for the custom G OS ecosystem

---

## Quick Start

The fastest way to get started is to use the core commands below:

```bash
gvalli search <query>
gvalli install <package>
gvalli remove <package>
gvalli update
gvalli clean
```

These commands are intended to cover the most common day-to-day operations while keeping the interface simple and predictable.

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

If you want to contribute, the project is open to improvements in CLI behavior, repository support, package handling, and overall user experience.

---

## License

This project is distributed under the MIT License.

---

## Русская версия

GValli — это современный и быстрый CLI-агрегатор для собственной экосистемы G OS. Проект рассчитан на работу с собственным репозиторием Gvalli-repo, где хранятся пакеты в формате .gpkg, предназначенном для этой системы. В будущем планируется отказаться от совместимости с Pacman, но пока инструмент поддерживает его как часть текущего этапа разработки.

Это не просто интерфейс поверх внешних утилит, а часть более широкой инфраструктуры для управления пакетами в среде G OS: поиск, установка, обновление, очистка и поддержка собственного репозитория объединены в одном инструменте.

<div align="center">
  <a href="#installation"><button>Установка</button></a>
  <a href="#quick-start"><button>Быстрый старт</button></a>
  <a href="#command-reference"><button>Команды</button></a>
  <a href="https://github.com/AdrescorGiti/gvalli-repo"><button>Репозиторий Gvalli</button></a>
</div>

### Почему GValli?

GValli создан для тех, кто хочет управлять пакетами без лишней сложности: без постоянного переключения между разными утилитами, без лишних шагов и без ощущения устаревшего интерфейса. Проект сочетает скорость, понятный UX и аккуратную работу с основными сценариями пакетного менеджмента внутри собственной среды G OS и её репозитория Gvalli-repo.

### Ключевые возможности

- ⚡ Быстрый и отзывчивый интерфейс
- 🧭 Умная маршрутизация между пакетными источниками, включая собственный репозиторий Gvalli-repo
- 🧹 Удобные команды для обновления и очистки системы
- 🔒 Безопасное поведение при системных операциях и сборке зависимостей
- 🌐 Поддержка современного рабочего потока для Arch-совместимых систем и кастомной среды G OS
- 🧱 Гибкая основа для будущего развития собственного формата пакетов и репозитория

### Быстрый старт

```bash
gvalli search <запрос>
gvalli install <пакет>
gvalli remove <пакет>
gvalli update
gvalli clean
```

Эти команды охватывают основные сценарии работы с пакетами и позволяют быстро перейти от поиска к установке и обслуживанию системы.

### Команды

| Команда | Алиас | Описание |
| --- | --- | --- |
| `gvalli search <запрос>` | `gvalli s`, `gvalli S` | Поиск по доступным пакетным источникам |
| `gvalli install <пакет>` | `gvalli i`, `gvalli I` | Установка пакета из подходящего источника |
| `gvalli remove <пакет>` | `gvalli r`, `gvalli R` | Поиск и удаление пакета из системы |
| `gvalli update` | `gvalli u`, `gvalli U`, `Syu` | Обновление пакетов и окружения |
| `gvalli clean` | `gvalli c`, `gvalli C` | Очистка кэша и неиспользуемых данных |

### Репозиторий

Для полноценной работы с экосистемой G OS используется собственный репозиторий:

- GitHub: [gvalli-repo](https://github.com/AdrescorGiti/gvalli-repo)
- Формат пакетов: `.gpkg`

### Лицензия

Проект распространяется под лицензией MIT.
