cask "ripple-mac" do
  version "0.4.0-beta.4"
  sha256 "aa7b262431437a9fbfddcdf76973e1a553dfc1d20c650435aab9105eb7355e33"

  url "https://github.com/vgs499p/ripple-mac/releases/download/v0.4.0-beta.4/Ripple-Beta-4-macOS-arm64.zip"
  name "Ripple Native"
  desc "Native Ripple client with local themes"
  homepage "https://github.com/vgs499p/ripple-mac"

  depends_on arch: :arm64
  depends_on macos: ">= :big_sur"
  app "Ripple Beta.app"
end
