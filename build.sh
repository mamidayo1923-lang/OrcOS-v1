#!/bin/bash
set -e

echo "Building OrcOS Kernel..."
RUSTFLAGS="-C link-arg=-Tlinker.ld" cargo build -Z build-std=core,compiler_builtins -Z build-std-features=compiler-builtins-mem -Z json-target-spec --target x86_64-orcos.json

echo "Fetching Limine Bootloader..."
if [ ! -d "limine" ]; then
    # GitHubのAPI制限やバージョン依存を回避するため、
    # Limine公式が推奨している「バイナリブランチ(v8.x-branch-binary)」を直接取得します。
    # これにより事前にビルドされたファイルが確実に取得でき、LLVM等の追加ツールも不要になります。
    echo "Cloning Limine v8 binary branch..."
    git clone https://github.com/limine-bootloader/limine.git --branch v8.x-branch-binary --depth=1

    # ホストOS用のインストールツール(limineコマンド)をビルド
    cd limine
    make
    cd ..
fi

echo "Preparing ISO directory structure..."
rm -rf iso_root
mkdir -p iso_root/boot
cp target/x86_64-orcos/debug/orcos iso_root/boot/orcos
cp limine.conf iso_root/

# バイナリブランチの場合、必要なファイルは limine/ 直下に用意されています
cp limine/limine-bios.sys limine/limine-bios-cd.bin limine/limine-uefi-cd.bin iso_root/

echo "Generating ISO image..."
xorriso -as mkisofs -b limine-bios-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    --efi-boot limine-uefi-cd.bin \
    -efi-boot-part --efi-boot-image --protective-msdos-label \
    iso_root -o orcos.iso

echo "Installing Limine to ISO for BIOS boot support..."
./limine/limine bios-install orcos.iso

echo "========================================"
echo "Success! Run with QEMU:"
echo "qemu-system-x86_64 -cdrom orcos.iso -serial stdio"
echo "========================================"
