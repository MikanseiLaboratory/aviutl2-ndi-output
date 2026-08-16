param(
    [switch]$Install
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$dist = Join-Path $root "dist"
$dll = Join-Path $root "target/x86_64-pc-windows-msvc/release/aviutl2_ndi_live_output.dll"
if (-not (Test-Path $dll)) {
    throw "Release DLL not found: $dll"
}

New-Item -ItemType Directory -Force -Path $dist | Out-Null
$aux2 = Join-Path $dist "aviutl2_ndi_live_output.aux2"
Copy-Item -Force $dll $aux2
Copy-Item -Force (Join-Path $root "i18n/English.aviutl2_ndi_live_output.aul2") (Join-Path $dist "English.aviutl2_ndi_live_output.aul2")
Copy-Item -Force (Join-Path $root "i18n/Japanese.aviutl2_ndi_live_output.aul2") (Join-Path $dist "Japanese.aviutl2_ndi_live_output.aul2")
Copy-Item -Force (Join-Path $root "LICENSE") (Join-Path $dist "LICENSE")

Write-Host "Wrote $aux2"

if ($Install) {
    $pluginDir = "C:\ProgramData\aviutl2\Plugin"
    $langDir = "C:\ProgramData\aviutl2\Language"
    if (-not (Test-Path $pluginDir)) {
        throw "AviUtl2 plugin dir not found: $pluginDir"
    }
    New-Item -ItemType Directory -Force -Path (Join-Path $pluginDir "aviutl2_ndi_live_output") | Out-Null
    New-Item -ItemType Directory -Force -Path $langDir | Out-Null
    Copy-Item -Force $aux2 (Join-Path $pluginDir "aviutl2_ndi_live_output.aux2")
    Copy-Item -Force (Join-Path $dist "English.aviutl2_ndi_live_output.aul2") (Join-Path $langDir "English.aviutl2_ndi_live_output.aul2")
    Copy-Item -Force (Join-Path $dist "Japanese.aviutl2_ndi_live_output.aul2") (Join-Path $langDir "Japanese.aviutl2_ndi_live_output.aul2")
    Copy-Item -Force (Join-Path $dist "LICENSE") (Join-Path $pluginDir "aviutl2_ndi_live_output\LICENSE")
    Write-Host "Installed to $pluginDir"
}
