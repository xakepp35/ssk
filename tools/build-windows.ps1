$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
$previousRustflags = $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS
try {
    # Run with Rust MSVC + Visual Studio C++ Build Tools and Windows SDK installed.
    rustup target add x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'rustup failed' }
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = '-C target-feature=+crt-static'
    cargo build --locked --release -p ssk --target x86_64-pc-windows-msvc --jobs 2
    if ($LASTEXITCODE -ne 0) { throw 'Cargo build failed' }
    $bundle = 'dist/ssk-windows-msvc'
    New-Item -ItemType Directory -Force $bundle | Out-Null
    Copy-Item 'target/x86_64-pc-windows-msvc/release/ssk.exe' "$bundle/ssk.exe"
    $config = (Get-Content 'ssk.yml' -Raw) -replace '(?m)^mode: host\s*$', 'mode: join'
    [System.IO.File]::WriteAllText((Join-Path $PWD "$bundle/ssk.yml"), $config)
    Copy-Item 'docs/LAN.md' "$bundle/PLAY.md"
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archivePath = Join-Path $PWD 'dist/ssk-windows-msvc.zip'
    if (Test-Path $archivePath) { Remove-Item $archivePath }
    $archive = [System.IO.Compression.ZipFile]::Open($archivePath, 'Create')
    try {
        foreach ($name in @('ssk.exe', 'ssk.yml', 'PLAY.md')) {
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
                $archive, (Join-Path $PWD "$bundle/$name"), $name
            ) | Out-Null
        }
    } finally { $archive.Dispose() }
    Write-Host 'Windows package: dist/ssk-windows-msvc.zip'
} finally {
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = $previousRustflags
    Pop-Location
}
