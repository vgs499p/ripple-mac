#!/bin/bash
set -euo pipefail
yum install -y gtk3-devel libxkbcommon-devel libxkbcommon-x11-devel fontconfig-devel rpm-build xorg-x11-server-Xvfb xorg-x11-xauth
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup.sh
sh /tmp/rustup.sh -y --profile minimal
. "$HOME/.cargo/env"
cargo build --release --locked -p ripple-mac --no-default-features --features software
objdump -T target/release/ripple | grep -oE 'GLIBC_[0-9.]+' | sort -Vu > glibc-symbols.txt
cat glibc-symbols.txt
test "$(tail -1 glibc-symbols.txt)" = GLIBC_2.17
/opt/python/cp311-cp311/bin/python3 scripts/generate-notices.py
/opt/python/cp311-cp311/bin/python3 scripts/package-unix.py --format rpm
/opt/python/cp311-cp311/bin/python3 scripts/package-unix.py --format tar
mv dist/*linux-x86_64.tar.gz dist/ripple-native-linux-glibc217-x86_64.tar.gz
set +e
xvfb-run -a timeout 15s target/release/ripple > startup.log 2>&1
code=$?
set -e
cat startup.log
test "$code" = 124
! grep -E 'panicked|error while loading shared libraries' startup.log
