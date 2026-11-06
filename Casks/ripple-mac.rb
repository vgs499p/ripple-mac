cask "ripple-mac" do
  version "0.4.0-beta.3"
  sha256 "5ebf9d508bea60d54d542fc502f7c5ebf16f99e314540dbc2472aa3b6551fd70"

  url "https://github.com/vgs499p/ripple-mac/releases/download/v0.4.0-beta.3/Ripple-Beta-3-macOS-arm64.zip"
  name "Ripple Native"
  desc "Native Ripple client with local themes"
  homepage "https://github.com/vgs499p/ripple-mac"

  depends_on arch: :arm64
  depends_on macos: ">= :big_sur"
  app "Ripple Beta.app"
end
