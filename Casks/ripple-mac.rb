cask "ripple-mac" do
  version "0.4.0-beta.5"
  sha256 "91fc90720e2e71c6f0955ddbc114cbc60be7b70a1ac6dd6f4c5118d1807252d2"

  url "https://github.com/vgs499p/ripple-mac/releases/download/v0.4.0-beta.5/Ripple-Beta-5-macOS-arm64.zip"
  name "Ripple Native"
  desc "Native Ripple client with local themes"
  homepage "https://github.com/vgs499p/ripple-mac"

  depends_on arch: :arm64
  depends_on macos: ">= :big_sur"
  app "Ripple Beta.app"
end
