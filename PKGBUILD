# Maintainer: Your Name <your.email@example.com>
pkgname=BrainRot_Battery
pkgver=1.0.0
pkgrel=1
pkgdesc="BrainRot Battery Monitor. A package which helps you to monitor your battery details in real time"
arch=('x86_64')
url="https://github.com/username/batfi"
license=('MIT')
depends=()
makedepends=('rust' 'cargo')
source=("$pkgname-$pkgver.tar.gz::https://github.com/username/$pkgname/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')  # Replace with actual checksum

build() {
    cd "$pkgname-$pkgver"
    cargo build --release --locked
}

check() {
    cd "$pkgname-$pkgver"
    cargo test --release --locked
}

package() {
    cd "$pkgname-$pkgver"
    
    # Install binary
    install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
    
    # Install license
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
    
    # Install documentation
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
    
    # Install man page if it exists
    if [ -f "$pkgname.1" ]; then
        install -Dm644 "$pkgname.1" "$pkgdir/usr/share/man/man1/$pkgname.1"
    fi
}
