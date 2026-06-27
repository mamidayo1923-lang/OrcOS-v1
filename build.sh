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

    # --- 32ビット開発ツールのインストール（追加） ---
    echo "32-bit development tools をインストールしています..."
    if command -v apt-get >/dev/null 2>&1; then
      sudo apt-get update
      sudo apt-get install -y --no-install-recommends gcc-multilib g++-multilib
    elif command -v yum >/dev/null 2>&1; then
      sudo yum install -y glibc-devel.i686 gcc
    elif command -v brew >/dev/null 2>&1; then
      echo "macOS では 32-bit tools は不要です"
    else
      echo "警告: 32ビット開発ツールの自動インストールがサポートされていません" >&2
    fi
    # --- ここまで追加 ---

    # --- ここから nasm のチェックとインストール ---
    # Limine の configure が nasm を必要とするため、ホストに nasm がなければ自動で入れる
    if ! command -v nasm >/dev/null 2>&1; then
      echo "nasm が見つかりません。インストールを試みます..."
      if command -v apt-get >/dev/null 2>&1; then
        sudo apt-get update
        sudo apt-get install -y --no-install-recommends nasm
      elif command -v yum >/dev/null 2>&1; then
        sudo yum install -y nasm
      elif command -v brew >/dev/null 2>&1; then
        brew install nasm
      else
        echo "自動インストールがサポートされていない環境です。手動で nasm をインストールしてください。" >&2
        exit 1
      fi
    fi
    # --- ここまで ---

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
