# GValli

# 🚀 GValli

**GValli** is an ultra-fast, concurrent CLI package manager aggregator written in Rust. It unifies searching, installing, and managing packages across **AUR**, **Pacman**, and **Flatpak** into a single, intuitive interface.

---

## 📥 Installation

Download the pre-compiled `.pkg.tar.zst` package from the [Releases](https://github.com/AdrescorGiti/GValli/releases) page and install it with `pacman`:

```bash
sudo pacman -U gvalli-0.1.0-1-x86_64.pkg.tar.zst

```

---

## ✨ Features

* **⚡ Blazing-Fast Concurrency:** Parallel async search via `tokio::join!` (<0.1s response times).
* **🎨 Interactive TUI:** Smooth terminal UI with viewport scrolling for easy package selection.
* **🛡️ Privilege Sandboxing:** Safe AUR builds. Automatically drops root privileges to `$SUDO_USER` during `makepkg` execution.
* **🔗 Recursive AUR Resolver:** Automatically parses `.SRCINFO` and recursively resolves/builds nested AUR dependencies.
* **🎯 Deduplication & Priorities:** Merges search results automatically prioritized by `[1] AUR > [2] Pacman > [3] Flatpak`.
* **🔍 Smart Routing & Fuzzy Removal:** Automatically detects where packages are installed and provides interactive deletion options.
* **⚡ Short Command Aliases:** Command shortcuts like `s` (search), `i` (install), `r` (remove), `u` (update), and `c` (clean).

---

## ⌨️ Command Usage & Aliases

| Command | Alias | Description |
| --- | --- | --- |
| `gvalli search <query>` | `gvalli s` / `gvalli S` | Interactive parallel search across all repositories with TUI selection |
| `gvalli install <pkg>` | `gvalli i` / `gvalli I` | Smart package installation (auto-detects source) |
| `gvalli remove <pkg>` | `gvalli r` / `gvalli R` | Fuzzy package search and removal from system |
| `gvalli update` | `gvalli u` / `gvalli U` / `Syu` | Full system update (Pacman repos + Flatpak runtimes) |
| `gvalli clean` | `gvalli c` / `gvalli C` | Cleans Pacman package cache and removes unused Flatpak runtimes |

---

## 📄 License

Distributed under the MIT License.

---

---

# 🚀 GValli (Русская версия)

**GValli** — это ультрабыстрый асинхронный CLI-агрегатор пакетных менеджеров на Rust. Он объединяет поиск, установку и управление пакетами из **AUR**, **Pacman** и **Flatpak** в едином удобном интерфейсе.

---

## 📥 Установка

Скачайте готовый собранный пакет `.pkg.tar.zst` со страницы [Releases](https://www.google.com/url?sa=E&source=gmail&q=https://github.com/yourusername/gvalli/releases) и установите его одной командой через `pacman`:

```bash
sudo pacman -U gvalli-0.1.0-1-x86_64.pkg.tar.zst

```

---

## ✨ Особенности

* **⚡ Параллельный асинхронный движок:** Параллельный поиск через `tokio::join!` (время отклика <0.1s).
* **🎨 Интерактивный TUI:** Плавный терминальный интерфейс со скроллингом и удобным выбором пакетов.
* **🛡️ Безопасность сборки:** Безопасная сборка AUR-пакетов с автоматическим сбросом root-привилегий до `$SUDO_USER`.
* **🔗 Рекурсивный резолвер AUR:** Автоматический анализ `.SRCINFO` и рекурсивная сборка вложенных AUR-зависимостей.
* **🎯 Дедупликация и приоритеты:** Объединение результатов поиска со строгим приоритетом: `[1] AUR > [2] Pacman > [3] Flatpak`.
* **🔍 Умный роутинг и нечеткое удаление:** Автоматическое определение источника пакета и удобный поиск при удалении.
* **⚡ Короткие алиасы:** Быстрые сокращения команд (`s`, `i`, `r`, `u`, `c`).

---

## ⌨️ Использование и Алиасы

| Команда | Алиас | Описание |
| --- | --- | --- |
| `gvalli search <запрос>` | `gvalli s` / `gvalli S` | Параллельный поиск по всем репозиториям с вызовом TUI-меню |
| `gvalli install <пакет>` | `gvalli i` / `gvalli I` | Умная установка пакета с автоопределением источника |
| `gvalli remove <пакет>` | `gvalli r` / `gvalli R` | Поиск и нечеткое удаление пакета из системы |
| `gvalli update` | `gvalli u` / `gvalli U` / `Syu` | Полное обновление системы (Pacman + Flatpak) |
| `gvalli clean` | `gvalli c` / `gvalli C` | Очистка кэша Pacman и неиспользуемых пакетов Flatpak |

---

## 📄 Лицензия

Распространяется под лицензией MIT.

```

```

```

```
