#!/bin/bash
set -e

echo "Building OrcOS Kernel..."
# .cargoフォルダの代わりとして、コンパイルオプションと環境変数を直接指定する
RUSTFLAGS="-C link-arg=-Tlinker.ld" cargo build -Z build-std=core,compiler_builtins -Z build-std-features=compiler-builtins-mem --target x86_64-orcos.json

echo "Fetching Limine Bootloader..."
if [ ! -d "limine" ]; then
    git clone https://github.com/limine-bootloader/limine.git --branch v7.x-branch-binary --depth=1
fi

echo "Preparing ISO directory structure..."
rm -rf iso_root
mkdir -p iso_root/boot
cp target/x86_64-orcos/debug/orcos iso_root/boot/orcos
cp limine.conf iso_root/
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