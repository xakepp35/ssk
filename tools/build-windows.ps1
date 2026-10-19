$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    # Run with Rust MSVC + Visual Studio C++ Build Tools and Windows SDK installed.
    rustup target add x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'rustup failed' }
    cargo build --locked --release -p ssk --target x86_64-pc-windows-msvc --jobs 2
    if ($LASTEXITCODE -ne 0) { throw 'Cargo build failed' }
    $bundle = 'dist/ssk-windows-msvc'
    New-Item -ItemType Directory -Force "$bundle/examples" | Out-Null
    Copy-Item 'target/x86_64-pc-windows-msvc/release/ssk.exe' "$bundle/ssk.exe"
    Copy-Item 'examples/lan.yaml' "$bundle/examples/lan.yaml"
    Copy-Item 'docs/LAN.md' "$bundle/PLAY.md"
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archivePath = Join-Path $PWD 'dist/ssk-windows-msvc.zip'
    if (Test-Path $archivePath) { Remove-Item $archivePath }
    $archive = [System.IO.Compression.ZipFile]::Open($archivePath, 'Create')
    try {
        foreach ($name in @('ssk.exe', 'examples/lan.yaml', 'PLAY.md')) {
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
                $archive, (Join-Path $PWD "$bundle/$name"), $name
            ) | Out-Null
        }
    } finally { $archive.Dispose() }
    Write-Host 'Windows package: dist/ssk-windows-msvc.zip'
} finally { Pop-Location }
