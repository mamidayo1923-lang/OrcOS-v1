#!/bin/bash
set -e

echo "Building OrcOS Kernel..."
RUSTFLAGS="-C link-arg=-Tlinker.ld" cargo build -Z build-std=core,compiler_builtins -Z build-std-features=compiler-builtins-mem -Z json-target-spec --target x86_64-orcos.json

echo "Fetching Limine Bootloader..."
if [ ! -d "limine" ]; then
    # バイナリブランチが廃止されたため、最新のリリース版(tarファイル)を直接ダウンロードするように変更
    echo "Downloading Limine release..."
    URL=$(curl -s https://api.github.com/repos/limine-bootloader/limine/releases/latest | grep "browser_download_url" | grep -E "\.tar\.(gz|xz)" | head -n 1 | cut -d '"' -f 4)
    if [ -z "$URL" ]; then
        URL="https://github.com/limine-bootloader/limine/releases/download/v8.0.0/limine-8.0.0.tar.xz"
    fi
    wget -qO limine.tar "$URL"
    mkdir -p limine
    tar -xf limine.tar -C limine --strip-components=1
    
    # Limine v8のビルドに必要な lld と llvm (llvm-objcopy) がなければ自動でインストール
    if ! command -v ld.lld &> /dev/null || ! command -v llvm-objcopy &> /dev/null; then
        echo "Installing lld and llvm (required for Limine v8)..."
        sudo apt-get update
        sudo apt-get install -y lld llvm
    fi

    # ホストOS(Linux)用のインストールツールをビルド
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
# ★ 修正箇所1：ファイルの場所を limine/ から limine/bin/ に変更
cp limine/bin/limine-bios.sys limine/bin/limine-bios-cd.bin limine/bin/limine-uefi-cd.bin iso_root/

echo "Generating ISO image..."
xorriso -as mkisofs -b limine-bios-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    --efi-boot limine-uefi-cd.bin \
    -efi-boot-part --efi-boot-image --protective-msdos-label \
    iso_root -o orcos.iso

echo "Installing Limine to ISO for BIOS boot support..."
# ★ 修正箇所2：ツールの場所を limine/limine から limine/bin/limine に変更
./limine/bin/limine bios-install orcos.iso

echo "========================================"
echo "Success! Run with QEMU:"
echo "qemu-system-x86_64 -cdrom orcos.iso -serial stdio"
echo "========================================"
