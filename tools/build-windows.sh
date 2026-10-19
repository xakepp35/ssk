#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# A system MinGW installation or an unpacked toolchain under ignored target/.
ssk_mingw="$PWD/target/mingw/usr"
ssk_cc="${SSK_WINDOWS_CC:-x86_64-w64-mingw32-gcc}"
if ! command -v "$ssk_cc" >/dev/null && [[ -x "$ssk_mingw/bin/x86_64-w64-mingw32-gcc-posix" ]]; then
    export PATH="$ssk_mingw/bin:$PATH"
    ssk_cc="$ssk_mingw/bin/x86_64-w64-mingw32-gcc-posix"
fi
if ! command -v "$ssk_cc" >/dev/null; then
    echo 'Install MinGW-w64 (gcc-mingw-w64-x86-64), or set SSK_WINDOWS_CC.' >&2
    exit 1
fi
rustup target add x86_64-pc-windows-gnu
export CC_x86_64_pc_windows_gnu="$ssk_cc"
export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="$ssk_cc"
cargo build --locked --release -p ssk --target x86_64-pc-windows-gnu --jobs "${SSK_BUILD_JOBS:-2}"
ssk_bundle="$PWD/dist/ssk-windows-gnu"
mkdir -p "$ssk_bundle/examples"
cp target/x86_64-pc-windows-gnu/release/ssk.exe "$ssk_bundle/"
# The GCC runtime may import this DLL; bundling it makes the archive portable.
ssk_pthread="$("$ssk_cc" -print-file-name=libwinpthread-1.dll)"
if [[ -f "$ssk_pthread" ]]; then cp "$ssk_pthread" "$ssk_bundle/"; fi
cp examples/lan.yaml "$ssk_bundle/examples/"
cp docs/LAN.md "$ssk_bundle/PLAY.md"
python3 - <<'PY'
from pathlib import Path
from zipfile import ZipFile,ZIP_DEFLATED
root=Path('dist/ssk-windows-gnu')
with ZipFile('dist/ssk-windows-gnu.zip','w',ZIP_DEFLATED) as z:
    for name in ['ssk.exe', 'examples/lan.yaml', 'PLAY.md', 'libwinpthread-1.dll']:
        p = root / name
        if p.is_file(): z.write(p, name)
PY
printf 'Windows package: %s\n' "$PWD/dist/ssk-windows-gnu.zip"
