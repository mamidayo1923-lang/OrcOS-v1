#!/bin/bash
set -e

# 万が一に備えてBIOSブートローダーのビルドに必要な nasm をインストール
echo "Ensuring nasm is installed for BIOS bootloader building..."
sudo apt-get update -y > /dev/null 2>&1 || true
sudo apt-get install -y nasm > /dev/null 2>&1 || true

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

    # ホスト用のインストールツール(limineコマンド)のみをコンパイル
    # これにより、同梱されているブートローダーバイナリが削除されるのを防ぎます
    cd limine
    ./configure
    make limine
    cd ..
fi

echo "Preparing ISO directory structure..."
rm -rf iso_root
mkdir -p iso_root/boot
cp target/x86_64-orcos/debug/orcos iso_root/boot/orcos
cp limine.conf iso_root/

echo "Finding and copying Limine bootloader files..."
# フォルダ構造が変わっても、確実に見つけてコピーする最強のコマンド
find limine -type f -name "limine-bios.sys" -exec cp -v {} iso_root/ \;
find limine -type f -name "limine-bios-cd.bin" -exec cp -v {} iso_root/ \;
find limine -type f -name "limine-uefi-cd.bin" -exec cp -v {} iso_root/ \;

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
