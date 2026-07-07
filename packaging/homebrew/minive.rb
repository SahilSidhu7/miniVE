# Homebrew cask for miniVE.
# Lives in the tap repo: github.com/SahilSidhu7/homebrew-tap as Casks/minive.rb
# After each release: bump `version`, replace both sha256 values
# (shasum -a 256 <file>, or copy from the release's *.sig-adjacent checksums).
cask "minive" do
  version "0.1.0"

  arch arm: "aarch64", intel: "x64"
  sha256 arm:   "REPLACE_WITH_AARCH64_DMG_SHA256",
         intel: "REPLACE_WITH_X64_DMG_SHA256"

  url "https://github.com/SahilSidhu7/miniVE/releases/download/v#{version}/miniVE_#{version}_#{arch}.dmg"
  name "miniVE"
  desc "Disposable Docker-backed dev environments"
  homepage "https://github.com/SahilSidhu7/miniVE"

  depends_on cask: "docker-desktop"

  app "miniVE.app"

  zap trash: [
    "~/Library/Application Support/com.sahil.minive",
    "~/Library/Caches/com.sahil.minive",
    "~/Library/WebKit/com.sahil.minive",
  ]
end
