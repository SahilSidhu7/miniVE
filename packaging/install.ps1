# miniVE installer — Windows PowerShell.
#
#   irm https://sahilsidhu7.github.io/minive-landing/install.ps1 | iex
#
# Downloads the latest MSI from GitHub Releases and runs the installer.
$ErrorActionPreference = "Stop"

$repo = "SahilSidhu7/miniVE"
$api = "https://api.github.com/repos/$repo/releases/latest"

Write-Host "Looking up the latest miniVE release..."
$release = Invoke-RestMethod -Uri $api -UseBasicParsing
$asset = $release.assets | Where-Object { $_.name -like "*_x64_en-US.msi" } | Select-Object -First 1
if (-not $asset) {
    Write-Error "No MSI asset found in the latest release ($($release.tag_name))."
    exit 1
}

$msi = Join-Path $env:TEMP $asset.name
Write-Host "Downloading $($asset.name)..."
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $msi -UseBasicParsing

Write-Host "Installing miniVE $($release.tag_name)..."
$proc = Start-Process msiexec.exe -ArgumentList "/i", "`"$msi`"", "/qb" -Wait -PassThru
Remove-Item $msi -ErrorAction SilentlyContinue

if ($proc.ExitCode -ne 0) {
    Write-Error "msiexec exited with code $($proc.ExitCode)."
    exit $proc.ExitCode
}
Write-Host "Done. Launch miniVE from the Start menu."
