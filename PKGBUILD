# Maintainer: Giti Adrescor <your-email@domain.com>
pkgname=gvalli
pkgver=0.6.0
pkgrel=1
pkgdesc="Ультрабыстрый CLI-агрегатор пакетных менеджеров (AUR, Pacman, Flatpak)"
arch=('x86_64' 'aarch64')
license=('MIT')
depends=('gcc-libs' 'glibc' 'pacman' 'flatpak' 'openssl')
makedepends=('cargo' 'pkgconf')
optdepends=(
    'git: необходим для клонирования и сборки пакетов из AUR'
)

# Пустой массив, так как собираем напрямую из текущей рабочей директории
source=()
sha256sums=()

build() {
    cd "$startdir"

    # Изолируем CARGO_HOME в рабочей директории сборки
    export CARGO_HOME="$srcdir/cargo-home"

    # =========================================================================
    # ФИКС ЛИНКОВЩИКА (LLD & --as-needed):
    # Переопределяем системные LDFLAGS/RUSTFLAGS, которые агрессивно режут 
    # динамические символы OpenSSL (SSL_free) и ассемблерные вставки ring.
    # =========================================================================
    export LDFLAGS=""
    export RUSTFLAGS="-C link-arg=-Wl,--no-as-needed"

    cargo build --release
}

check() {
    cd "$startdir"

    export CARGO_HOME="$srcdir/cargo-home"
    export LDFLAGS=""
    export RUSTFLAGS="-C link-arg=-Wl,--no-as-needed"

    cargo test
}

package() {
    cd "$startdir"

    # Установка скомпилированного бинарника в /usr/bin/gvalli
    install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
}