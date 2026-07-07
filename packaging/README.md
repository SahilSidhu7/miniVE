# Packaging

Submission templates for package managers. Both need a published GitHub release first
(real download URLs + checksums).

## winget (Windows)

Recommended: let `wingetcreate` fill hashes/ProductCode and open the PR:

```powershell
winget install wingetcreate
wingetcreate new https://github.com/SahilSidhu7/miniVE/releases/download/v0.1.0/miniVE_0.1.0_x64_en-US.msi
# PackageIdentifier: SahilSidhu7.miniVE — copy descriptions from winget/*.yaml here
```

Manual alternative: fill the `REPLACE_WITH_*` fields in `winget/*.yaml`
(`InstallerSha256` = `Get-FileHash file.msi`; ProductCode from the MSI), then PR the three
files to `microsoft/winget-pkgs` under `manifests/s/SahilSidhu7/miniVE/0.1.0/`.

After the PR merges (usually 1–3 days): `winget install SahilSidhu7.miniVE`.

## Homebrew (macOS)

1. Create a public repo named exactly `homebrew-tap` under SahilSidhu7.
2. Copy `homebrew/minive.rb` to `Casks/minive.rb` in that repo.
3. Fill both `sha256` values from the release `.dmg` files (`shasum -a 256 <file>`).
4. Push. Done — no review process, it's your tap.

Users then run: `brew install --cask sahilsidhu7/tap/minive`.

## Each new release

- winget: `wingetcreate update SahilSidhu7.miniVE -u <new .msi url> -v <version> --submit`
- Homebrew: bump `version` + both `sha256` in `Casks/minive.rb`, push.
