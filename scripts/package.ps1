param(
    [string]$Version = (Select-String -Path "$PSScriptRoot/../Cargo.toml" -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$dist = Join-Path $root "dist"
$stage = Join-Path $dist "stage"
$dll = Join-Path $root "target/x86_64-pc-windows-msvc/release/aviutl2_ndi_live_output.dll"
$pluginData = Join-Path $stage "Plugin/aviutl2_ndi_live_output"

if (-not (Test-Path $dll)) {
    throw "Release DLL not found: $dll"
}

if (Test-Path $stage) {
    Remove-Item -Recurse -Force $stage
}
New-Item -ItemType Directory -Force -Path $pluginData | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stage "Language") | Out-Null

Copy-Item $dll (Join-Path $stage "Plugin/aviutl2_ndi_live_output.aux2")
Copy-Item (Join-Path $root "i18n/English.aviutl2_ndi_live_output.aul2") (Join-Path $stage "Language/English.aviutl2_ndi_live_output.aul2")
Copy-Item (Join-Path $root "i18n/Japanese.aviutl2_ndi_live_output.aul2") (Join-Path $stage "Language/Japanese.aviutl2_ndi_live_output.aul2")
Copy-Item (Join-Path $root "LICENSE") (Join-Path $pluginData "LICENSE")
Copy-Item (Join-Path $root "THIRD_PARTY_NOTICES.md") (Join-Path $pluginData "THIRD_PARTY_NOTICES.md")
Copy-Item (Join-Path $root "NDI_TERMS.txt") (Join-Path $pluginData "NDI_TERMS.txt")
Copy-Item (Join-Path $root "README.md") (Join-Path $pluginData "README.md")

$ndiRuntime = & (Join-Path $PSScriptRoot "find-ndi-runtime.ps1")
Copy-Item $ndiRuntime.Dll (Join-Path $stage "Plugin/Processing.NDI.Lib.x64.dll")
Copy-Item $ndiRuntime.Dll (Join-Path $pluginData "Processing.NDI.Lib.x64.dll")
Copy-Item $ndiRuntime.Licenses (Join-Path $pluginData "Processing.NDI.Lib.Licenses.txt")

@"
id=aviutl2-ndi-output
name=AviUtl2 Network Video Output
information=AviUtl2 Network Video Output v$Version
"@ | Set-Content -Encoding UTF8 (Join-Path $stage "package.ini")

$zipName = "aviutl2-ndi-output-v$Version.au2pkg.zip"
$zipPath = Join-Path $dist $zipName
if (Test-Path $zipPath) {
    Remove-Item -Force $zipPath
}

Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zipPath

$required = @(
    "Plugin/aviutl2_ndi_live_output.aux2",
    "Plugin/Processing.NDI.Lib.x64.dll",
    "Plugin/aviutl2_ndi_live_output/Processing.NDI.Lib.x64.dll",
    "Plugin/aviutl2_ndi_live_output/Processing.NDI.Lib.Licenses.txt",
    "Plugin/aviutl2_ndi_live_output/NDI_TERMS.txt",
    "Plugin/aviutl2_ndi_live_output/LICENSE",
    "Plugin/aviutl2_ndi_live_output/THIRD_PARTY_NOTICES.md",
    "Language/English.aviutl2_ndi_live_output.aul2",
    "Language/Japanese.aviutl2_ndi_live_output.aul2"
)
foreach ($rel in $required) {
    $path = Join-Path $stage $rel
    if (-not (Test-Path $path)) {
        throw "Package is missing $rel"
    }
}

Write-Host "Wrote $zipPath"
