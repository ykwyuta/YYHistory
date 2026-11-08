#!/usr/bin/env bash
# WebAssembly 版をビルドして dist/ に静的サイトを出力する。
# Cloudflare Pages ではビルドコマンドに `bash build.sh`、出力ディレクトリに `dist` を指定する。
set -euo pipefail
cd "$(dirname "$0")"

if ! command -v cargo >/dev/null 2>&1; then
  if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
  else
    echo "Rust が無いので rustup で導入する"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    . "$HOME/.cargo/env"
  fi
fi
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true

# ブラウザ版はゲーム本体（lib）だけを、デバッグ情報なしで作る（Windows 版の設定とは別）
cargo build --release --lib --target wasm32-unknown-unknown --config profile.release.debug=false

rm -rf dist
mkdir -p dist
cp web/index.html web/main.js web/speech.mjs web/manifest.webmanifest web/icon-192.png web/icon-512.png web/apple-touch-icon.png dist/
cp assets/app_icon.ico dist/favicon.ico
cp target/wasm32-unknown-unknown/release/yyhistory.wasm dist/
# wasm-opt があればさらに小さくする（任意）
if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -O3 dist/yyhistory.wasm -o dist/yyhistory.wasm || true
fi
node tools/package_wasm.mjs dist/yyhistory.wasm
cat > dist/_headers <<'H'
/yyhistory.wasm
  Content-Type: application/wasm
  Cache-Control: no-cache
/wasm/*
  Content-Type: application/octet-stream
  Cache-Control: public, max-age=31536000, immutable
/wasm-manifest.json
  Cache-Control: no-cache
/main.js
  Cache-Control: no-cache
/speech.mjs
  Content-Type: text/javascript
  Cache-Control: no-cache
/index.html
  Cache-Control: no-cache
/
  Cache-Control: no-cache
H
ls -la dist
