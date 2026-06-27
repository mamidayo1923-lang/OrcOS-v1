#!/bin/bash
set -e

echo "Building OrcOS Kernel..."
RUSTFLAGS="-C link-arg=-Tlinker.ld" cargo build -Z build-std=core,compiler_builtins -Z build-std-features=compiler-builtins-mem -Z json-target-spec --target x86_64-orcos.json

echo "Fetching Limine Bootloader..."
if [ ! -d "limine" ]; then
    echo "Querying the latest Limine 7.x tag using git ls-remote to bypass GitHub API limits..."
    # GitHubのAPI制限を回避するため、Gitプロトコルで確実に存在する最新のv7タグを自動取得します
    LIMINE_TAG=$(git ls-remote --tags https://github.com/limine-bootloader/limine.git | awk '{print $2}' | grep -E '^refs/tags/v7\.[0-9]+\.[0-9]+$' | sed 's|^refs/tags/||' | sort -V | tail -n 1)
    LIMINE_VERSION=${LIMINE_TAG#v}
    echo "Latest Limine 7.x tag is: $LIMINE_TAG (Version: $LIMINE_VERSION)"

    URL_XZ="https://github.com/limine-bootloader/limine/releases/download/${LIMINE_TAG}/limine-${LIMINE_VERSION}.tar.xz"
    URL_GZ="https://github.com/limine-bootloader/limine/releases/download/${LIMINE_TAG}/limine-${LIMINE_VERSION}.tar.gz"

    mkdir -p limine
    
    # wgetのエラーでスクリプトが止まらないようにする (set -e の一時解除)
    set +e
    
    echo "Downloading $URL_XZ ..."
    wget -qO limine.tar.xz "$URL_XZ"
    
    # ダウンロード成功 ＆ ファイルサイズが0ではないかチェック
    if [ $? -eq 0 ] && [ -s limine.tar.xz ]; then
        tar -xf limine.tar.xz -C limine --strip-components=1
    else
        echo "Fallback: Downloading $URL_GZ ..."
        wget -qO limine.tar.gz "$URL_GZ"
        tar -xf limine.tar.gz -C limine --strip-components=1
    fi
    
    # エラー時の自動停止を元に戻す
    set -e

    # ホストOS用のインストールツール(limineコマンド)をビルド
    cd limine
    ./configure
    make
    cd ..
fi

echo "Preparing ISO directory structure..."
rm -rf iso_root
mkdir -p iso_root/boot
cp target/x86_64-orcos/debug/orcos iso_root/boot/orcos
cp limine.conf iso_root/

# 必要なファイルをISOフォルダにコピー
cd limine
./configure --enable-bios --enable-uefi
make
cd ..

# ビルド成功確認
if [ ! -f "limine/limine-bios.sys" ] || [ ! -f "limine/limine-bios-cd.bin" ] || [ ! -f "limine/limine-uefi-cd.bin" ]; then
    echo "ERROR: Limine build failed - required bootloader files not generated"
    exit 1
fi

cp limine/limine-bios.sys limine/limine-bios-cd.bin limine/limine-uefi-cd.bin iso_root/

echo "Generating ISO image..."
xorriso -as mkisofs -b limine-bios-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    --efi-boot limine-uefi-cd.bin \
    -efi-boot-part --efi-boot-image --protective-msdos-label \
    iso_root -o orcos.iso

echo "Installing Limine to ISO for BIOS boot support..."
./limine/bin/limine bios-install orcos.iso

echo "========================================"
echo "Success! Run with QEMU:"
echo "qemu-system-x86_64 -cdrom orcos.iso -serial stdio"
echo "========================================"
