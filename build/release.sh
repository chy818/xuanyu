#!/usr/bin/env bash
# ==============================================================
# Xuanyu Compiler Release Packaging Script (跨平台: Windows/Linux/macOS)
# --------------------------------------------------------------
# Produces a distributable release:
#   - target/release/xy(.exe)    the compiler
#   - runtime/runtime.c          runtime library
#   - src/compiler_v2/xyc.xy     L2 self-hosting compiler source
#   - examples/                  sample programs
#   - docs/                      docs (README/CHANGELOG/API_REFERENCE)
#   - VERSION                    version file
# Usage: bash build/release.sh [version]
# Example: bash build/release.sh v0.3.0-beta
# ==============================================================

set -euo pipefail

VERSION="${1:-0.3.0-beta}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RELEASE_DIR="$ROOT/dist"
PKG_NAME="xuanyu-$VERSION"
VERSION_DIR="$RELEASE_DIR/$PKG_NAME"

# ---------- Platform detection ----------
case "$(uname -s)" in
    Linux*)     OS=linux;;
    Darwin*)    OS=macos;;
    MINGW*|MSYS*|CYGWIN*) OS=windows;;
    *)          OS=unknown;;
esac

# Binary extension
if [ "$OS" = "windows" ]; then
    EXE_EXT=".exe"
else
    EXE_EXT=""
fi

# ---------- Colors ----------
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

step() { echo -e "\n${CYAN}[$1]${NC}"; }
ok()   { echo -e "${GREEN}$1${NC}"; }
err()  { echo -e "${RED}$1${NC}"; }

# ---------- 1. Build release compiler ----------
step "1/6 Build release compiler (OS=$OS)"
cd "$ROOT"
cargo build --release

# ---------- 2. Run tests ----------
step "2/6 Run tests"
cargo test --all-targets

# ---------- 3. Prepare packaging dir ----------
step "3/6 Prepare packaging dir"
rm -rf "$RELEASE_DIR"
mkdir -p "$VERSION_DIR"

# ---------- 4. Collect artifacts ----------
step "4/6 Collect artifacts"

XY_SRC="$ROOT/target/release/xy$EXE_EXT"
if [ -f "$XY_SRC" ]; then
    cp "$XY_SRC" "$VERSION_DIR/xy$EXE_EXT"
    ok "  xy$EXE_EXT"
else
    err "xy executable not found at $XY_SRC"
    exit 1
fi

cp "$ROOT/runtime/runtime.c" "$VERSION_DIR/runtime.c"
ok "  runtime.c"

if [ -f "$ROOT/src/compiler_v2/xyc.xy" ]; then
    cp "$ROOT/src/compiler_v2/xyc.xy" "$VERSION_DIR/xyc.xy"
    ok "  xyc.xy (L2 compiler)"
fi

mkdir -p "$VERSION_DIR/examples"
if ls "$ROOT/examples/"*.xy >/dev/null 2>&1; then
    cp "$ROOT/examples/"*.xy "$VERSION_DIR/examples/"
    ok "  examples/"
fi

mkdir -p "$VERSION_DIR/docs"
cp "$ROOT/README.md" "$VERSION_DIR/README.md"
for doc in CHANGELOG.md API_REFERENCE.md; do
    if [ -f "$ROOT/docs/$doc" ]; then
        cp "$ROOT/docs/$doc" "$VERSION_DIR/docs/$doc"
    fi
done
ok "  docs/"

echo "$VERSION" > "$VERSION_DIR/VERSION"
ok "  VERSION"

FILE_COUNT=$(find "$VERSION_DIR" -type f | wc -l)
echo "  Collected $FILE_COUNT files"

# ---------- 5. Archive ----------
step "5/6 Archive"
cd "$RELEASE_DIR"
if [ "$OS" = "windows" ]; then
    # Windows: use Python zipfile (most reliable across git bash / CI /本地)
    ARCHIVE="$RELEASE_DIR/$PKG_NAME.zip"
    PYTHON=$(command -v python3 2>/dev/null || command -v python 2>/dev/null || echo "")
    if [ -n "$PYTHON" ]; then
        # 转换 Unix 路径为 Windows 路径 (Python 不认 /c/... 格式)
        WIN_RELEASE_DIR=$(cd "$RELEASE_DIR" && cmd //c cd 2>/dev/null || echo "$RELEASE_DIR")
        # 直接在 release 目录下执行 Python zip
        (cd "$RELEASE_DIR" && "$PYTHON" -c "import shutil; shutil.make_archive('$PKG_NAME', 'zip', '.', '$PKG_NAME')")
        ok "  Created $PKG_NAME.zip"
    else
        echo "  WARNING: python not found, skipping zip. Dist dir is at $VERSION_DIR"
    fi
else
    # Linux/macOS: tar.gz
    ARCHIVE="$RELEASE_DIR/$PKG_NAME.tar.gz"
    tar -czf "$ARCHIVE" "$PKG_NAME"
    ok "  Created $PKG_NAME.tar.gz"
fi

# ---------- 6. Self-check ----------
step "6/6 Self-check: packaged xy compiles hello.xy"
SMOKE_DIR="$RELEASE_DIR/_smoke"
rm -rf "$SMOKE_DIR"
mkdir -p "$SMOKE_DIR"

XY_PACKAGED="$VERSION_DIR/xy$EXE_EXT"
cp "$XY_PACKAGED" "$SMOKE_DIR/xy$EXE_EXT"

if [ "$OS" != "windows" ]; then
    chmod +x "$SMOKE_DIR/xy" 2>/dev/null || true
fi

HELLO_PATH="$VERSION_DIR/examples/hello.xy"
SMOKE_LOG="$RELEASE_DIR/_smoke.log"

if "$SMOKE_DIR/xy$EXE_EXT" "$HELLO_PATH" --run > "$SMOKE_LOG" 2>&1; then
    ok "  Self-check passed: hello.xy compiled and ran"
else
    err "  Self-check failed!"
    cat "$SMOKE_LOG"
    exit 1
fi

rm -rf "$SMOKE_DIR" "$SMOKE_LOG"

# ---------- Done ----------
echo ""
ok "Release complete: $PKG_NAME"
echo "  Platform: $OS"
echo "  Artifacts: $RELEASE_DIR"
echo "    - $PKG_NAME/          extracted dist dir"
if [ "$OS" = "windows" ]; then
    echo "    - $PKG_NAME.zip      dist archive"
else
    echo "    - $PKG_NAME.tar.gz   dist archive"
fi
echo ""
