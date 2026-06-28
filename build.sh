#!/bin/bash
set -e

echo "Building OrcOS Kernel..."
RUSTFLAGS="-C link-arg=-Tlinker.ld" cargo build -Z build-std=core,compiler_builtins -Z build-std-features=compiler-builtins-mem -Z json-target-spec --target x86_64-orcos.json

echo "Fetching Limine Bootloader..."
if [ ! -d "limine" ]; then
    echo "Querying the latest Limine 7.x tag using git ls-remote to bypass GitHub API limits..."
    # API制限を回避して最新のv7タグを自動取得
    LIMINE_TAG=$(git ls-remote --tags https://github.com/limine-bootloader/limine.git | awk '{print $2}' | grep -E '^refs/tags/v7\.[0-9]+\.[0-9]+$' | sed 's|^refs/tags/||' | sort -V | tail -n 1)
    LIMINE_VERSION=${LIMINE_TAG#v}
    echo "Latest Limine 7.x tag is: $LIMINE_TAG (Version: $LIMINE_VERSION)"

    URL_XZ="https://github.com/limine-bootloader/limine/releases/download/${LIMINE_TAG}/limine-${LIMINE_VERSION}.tar.xz"
    mkdir -p limine
    
    echo "Downloading $URL_XZ ..."
    wget -qO limine.tar.xz "$URL_XZ"
    tar -xf limine.tar.xz -C limine --strip-components=1

    # ホストOS用の操作ツール（limineコマンド）だけをシンプルにビルド
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

echo "Finding and copying Limine bootloader files..."
# フォルダ構造が変わっても、同梱されている完成品ファイルを確実に見つけてコピーするコマンド
find limine -type f -name "limine-bios.sys" -exec cp -v {} iso_root/ \;
find limine -type f -name "limine-bios-cd.bin" -exec cp -v {} iso_root/ \;
find limine -type f -name "limine-uefi-cd.bin" -exec cp -v {} iso_root/ \;

# ファイルが正しくコピーされたか念のためチェック
if [ ! -f "iso_root/limine-bios.sys" ]; then
    echo "ERROR: Required bootloader files not found!"
    exit 1
fi

echo "Generating ISO image..."
xorriso -as mkisofs -b limine-bios-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    --efi-boot limine-uefi-cd.bin \
    -efi-boot-part --efi-boot-image --protective-msdos-label \
    iso_root -o orcos.iso

echo "Installing Limine to ISO for BIOS boot support..."
# 実行ファイル(limine)も自動で見つけて実行
LIMINE_TOOL=$(find limine -type f -name "limine" -executable | head -n 1)
$LIMINE_TOOL bios-install orcos.iso

echo "========================================"
echo "Success! Run with QEMU:"
echo "qemu-system-x86_64 -cdrom orcos.iso -serial stdio"
echo "========================================"
