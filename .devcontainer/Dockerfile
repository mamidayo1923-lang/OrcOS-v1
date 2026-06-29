FROM mcr.microsoft.com/devcontainers/base:ubuntu-22.04

# OS開発に必要な依存パッケージ、および画面転送に必要なnoVNC関連のインストール
RUN apt-get update && apt-get install -y \
    qemu-system-x86 \
    xorriso \
    mtools \
    git \
    curl \
    build-essential \
    novnc \
    websockify \
    && rm -rf /var/lib/apt/lists/*

# Rust Nightly環境の構築
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly \
    && rustup component add rust-src llvm-tools-preview

# 作業ディレクトリの権限調整
RUN chmod -R a+w /usr/local/cargo /usr/local/rustup