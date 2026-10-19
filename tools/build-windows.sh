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
# Keep compiler support libraries in the executable; OS DLL imports remain normal.
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-C target-feature=+crt-static -C link-arg=-static -C link-arg=-static-libgcc"
cargo build --locked --release -p ssk --target x86_64-pc-windows-gnu --jobs "${SSK_BUILD_JOBS:-2}"
ssk_bundle="$PWD/dist/ssk-windows-gnu"
mkdir -p "$ssk_bundle"
cp target/x86_64-pc-windows-gnu/release/ssk.exe "$ssk_bundle/"
# Remove only obsolete files from earlier generated distributions.
rm -f "$ssk_bundle/libwinpthread-1.dll" "$ssk_bundle/examples/lan.yaml"
rmdir "$ssk_bundle/examples" 2>/dev/null || true
cp docs/LAN.md "$ssk_bundle/PLAY.md"
ssk_objdump="${SSK_WINDOWS_OBJDUMP:-$(dirname "$ssk_cc")/x86_64-w64-mingw32-objdump}"
if [[ ! -x "$ssk_objdump" ]]; then ssk_objdump=x86_64-w64-mingw32-objdump; fi
"$ssk_objdump" -p "$ssk_bundle/ssk.exe" > target/ssk-windows-imports.txt
python3 - <<'PY'
from pathlib import Path
from zipfile import ZipFile,ZIP_DEFLATED
import re
root=Path('dist/ssk-windows-gnu')
imports = re.findall(r'DLL Name:\s*(\S+)', Path('target/ssk-windows-imports.txt').read_text())
system = {'advapi32.dll', 'gdi32.dll', 'kernel32.dll', 'shell32.dll', 'user32.dll',
          'imm32.dll', 'msvcrt.dll', 'ucrtbase.dll', 'bcryptprimitives.dll', 'combase.dll',
          'iphlpapi.dll', 'ntdll.dll', 'ole32.dll', 'oleaut32.dll', 'ws2_32.dll',
          'bcrypt.dll', 'crypt32.dll', 'opengl32.dll', 'winmm.dll', 'dwmapi.dll',
          'secur32.dll', 'userenv.dll', 'setupapi.dll', 'psapi.dll', 'version.dll'}
unexpected = [name for name in imports if name.lower() not in system
              and not name.lower().startswith(('api-ms-win-', 'ext-ms-win-'))]
if not imports or unexpected:
    raise SystemExit(f'Non-system Windows DLL imports or missing import table: {unexpected}')
config = Path('ssk.yml').read_text().replace('mode: host\n', 'mode: join\n', 1)
(root/'ssk.yml').write_text(config)
with ZipFile('dist/ssk-windows-gnu.zip','w',ZIP_DEFLATED) as z:
    for name in ['ssk.exe', 'ssk.yml', 'PLAY.md']:
        z.write(root / name, name)
print('Windows imports: system DLLs only; no external runtime DLL to distribute.')
PY
printf 'Windows package: %s\n' "$PWD/dist/ssk-windows-gnu.zip"
