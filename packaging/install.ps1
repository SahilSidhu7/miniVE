# miniVE installer — Windows PowerShell.
#
#   irm https://sahilsidhu7.github.io/minive-landing/install.ps1 | iex
#
# Installs the GUI app (MSI from GitHub Releases), the minive CLI, or both.
# Non-interactive use: set MINIVE_COMPONENTS to both|gui|cli before running.
$ErrorActionPreference = "Stop"

$repo = "SahilSidhu7/miniVE"
$api = "https://api.github.com/repos/$repo/releases/latest"

# --- Component selection ---------------------------------------------------
$components = "$env:MINIVE_COMPONENTS".Trim().ToLower()
if ($components -notin @("both", "gui", "cli")) {
    $components = "both"
    if (-not [Console]::IsInputRedirected) {
        $answer = Read-Host "Install [B]oth, [G]UI app only, or [C]LI only? (default: both)"
        switch -Regex ($answer.Trim()) {
            '^g' { $components = "gui" }
            '^c' { $components = "cli" }
        }
    }
}
$installGui = $components -ne "cli"
$installCli = $components -ne "gui"

Write-Host "Looking up the latest miniVE release..."
$release = Invoke-RestMethod -Uri $api -UseBasicParsing

# --- GUI app (MSI) ----------------------------------------------------------
if ($installGui) {
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
    Write-Host "App installed. Launch miniVE from the Start menu."
}

# --- minive CLI (separate release asset; the MSI only contains the app) -----
if ($installCli) {
    $cliAsset = $release.assets | Where-Object { $_.name -eq "minive-cli-windows-x86_64.zip" } | Select-Object -First 1
    if ($cliAsset) {
        $zip = Join-Path $env:TEMP $cliAsset.name
        Write-Host "Downloading the minive CLI..."
        Invoke-WebRequest -Uri $cliAsset.browser_download_url -OutFile $zip -UseBasicParsing
        $cliDir = Join-Path $env:LOCALAPPDATA "Programs\minive"
        New-Item -ItemType Directory -Force -Path $cliDir | Out-Null
        Expand-Archive -Path $zip -DestinationPath $cliDir -Force
        Remove-Item $zip -ErrorAction SilentlyContinue

        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if (($userPath -split ";") -notcontains $cliDir) {
            if ([string]::IsNullOrEmpty($userPath)) { $newPath = $cliDir } else { $newPath = "$userPath;$cliDir" }
            [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
            Write-Host "Added $cliDir to your user PATH."
        }
        Write-Host "CLI installed. Open a NEW terminal and run: minive list"
    } elseif ($components -eq "cli") {
        Write-Error "This release ($($release.tag_name)) has no prebuilt CLI; see the README for building it from source."
        exit 1
    } else {
        Write-Host "Note: this release has no prebuilt CLI yet; see the README for building it from source."
    }
}

Write-Host "Done."
