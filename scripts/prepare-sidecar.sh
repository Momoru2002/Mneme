#!/usr/bin/env bash
# Stage the `mneme-mcp` binary where Tauri's `bundle.externalBin` expects it:
#   apps/desktop/src-tauri/binaries/mneme-mcp-<target-triple>[.exe]
#
# Usage:
#   scripts/prepare-sidecar.sh --placeholder   empty stand-in file (dev / CI tests / clippy)
#   scripts/prepare-sidecar.sh --host          real release build for this machine (default)
#   scripts/prepare-sidecar.sh --universal     macOS only: arm64 + x64 + lipo'd universal binary
#
# Why placeholders exist: tauri-build checks that every externalBin file is present
# while compiling the package, and `mneme-mcp` lives in that same package — so the
# files must exist (even empty) before the real binary can be built.
set -euo pipefail

mode="${1:---host}"
cd "$(dirname "$0")/../apps/desktop/src-tauri"
mkdir -p binaries

host="$(rustc -vV | sed -n 's/^host: //p')"
ext=""
case "$host" in *windows*) ext=".exe" ;; esac

case "$mode" in
  --placeholder | --host) triples=("$host") ;;
  --universal) triples=(aarch64-apple-darwin x86_64-apple-darwin universal-apple-darwin) ;;
  *)
    echo "usage: $0 [--placeholder|--host|--universal]" >&2
    exit 2
    ;;
esac

for t in "${triples[@]}"; do
  if [ ! -e "binaries/mneme-mcp-$t$ext" ]; then
    : > "binaries/mneme-mcp-$t$ext"
  fi
done

if [ "$mode" = "--placeholder" ]; then
  echo "staged placeholder(s) in binaries/"
  exit 0
fi

if [ "$mode" = "--universal" ]; then
  for t in aarch64-apple-darwin x86_64-apple-darwin; do
    cargo build --release --bin mneme-mcp --target "$t"
    cp "target/$t/release/mneme-mcp" "binaries/mneme-mcp-$t"
  done
  lipo -create -output binaries/mneme-mcp-universal-apple-darwin \
    binaries/mneme-mcp-aarch64-apple-darwin binaries/mneme-mcp-x86_64-apple-darwin
else
  cargo build --release --bin mneme-mcp
  cp "target/release/mneme-mcp$ext" "binaries/mneme-mcp-$host$ext"
fi

ls -l binaries/
