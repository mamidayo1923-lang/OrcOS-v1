#!/bash/bin
set -e

# 1. 画面転送プロキシ（websockify）をバックグラウンドで起動
# QEMUのVNCポート（5900）をWebブラウザで読めるポート（6080）に変換する
echo "Starting noVNC proxy on port 6080..."
pkill -f websockify || true
websockify --web /usr/share/novnc 6080 localhost:5900 > /dev/null 2>&1 &

# 2. QEMUをVNCモードで起動
# -vnc 127.0.0.1:0 により、画面をコンテナ内部のポート5900に出力させる
echo "=========================================================="
echo "Starting QEMU with VNC output."
echo "Codespaces will notify you that Port 6080 is available."
echo "Open that port/URL in your iPad Safari to see OrcOS!"
echo "=========================================================="
echo "To exit QEMU, press Ctrl+C in this terminal."
echo "----------------------------------------------------------"

qemu-system-x86_64 -cdrom orcos.iso -serial stdio -vnc 127.0.0.1:0