#!/bin/bash
set -e

echo "Building OrcOS Kernel..."
RUSTFLAGS="-C link-arg=-Tlinker.ld" cargo build -Z build-std=core,compiler_builtins -Z build-std-features=compiler-builtins-mem -Z json-target-spec --target x86_64-orcos.json

echo "Fetching Limine Bootloader..."
if [ ! -d "limine" ]; then
    echo "Cloning Limine binary branch..."
    # 公式が用意している「コンパイル済みバイナリ入り」の専用ブランチを直接取得！
    # 最新の v8.x-binary を試し、見つからなければ v7.x-binary にフォールバックします
    if ! git clone https://github.com/limine-bootloader/limine.git --branch v8.x-binary --depth=1; then
        echo "v8.x-binary branch not found, falling back to v7.x-binary..."
        git clone https://github.com/limine-bootloader/limine.git --branch v7.x-binary --depth=1
    fi
    
    # ISOに書き込むためのホスト用ツール（limineコマンド）のみをビルド
    make -C limine
fi

echo "Preparing ISO directory structure..."
rm -rf iso_root
mkdir -p iso_root/boot
cp target/x86_64-orcos/debug/orcos iso_root/boot/orcos
cp limine.conf iso_root/

echo "Copying Limine bootloader files..."
# git cloneしたディレクトリには必ずファイルが存在するので、そのままシンプルにコピー！
cp limine/limine-bios.sys iso_root/
cp limine/limine-bios-cd.bin iso_root/
cp limine/limine-uefi-cd.bin iso_root/

# コピーが正しく行われたか確認する安全装置
if [ ! -f "iso_root/limine-bios.sys" ] || [ ! -f "iso_root/limine-bios-cd.bin" ] || [ ! -f "iso_root/limine-uefi-cd.bin" ]; then
    echo "ERROR: Required bootloader files not found in iso_root!"
    exit 1
fi

echo "Generating ISO image..."
xorriso -as mkisofs -b limine-bios-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    --efi-boot limine-uefi-cd.bin \
    -efi-boot-part --efi-boot-image --protective-msdos-label \
    iso_root -o orcos.iso

echo "Installing Limine to ISO for BIOS boot support..."
# 実行ファイル(limine)も自動で見つけて実行する
LIMINE_TOOL=$(find limine -type f -name "limine" -executable | head -n 1)
if [ -n "$LIMINE_TOOL" ]; then
    $LIMINE_TOOL bios-install orcos.iso
else
    echo "Error: limine host tool not found!"
    exit 1
fi

echo "========================================"
echo "Success! Run with QEMU:"
echo "qemu-system-x86_64 -cdrom orcos.iso -serial stdio"
echo "========================================"