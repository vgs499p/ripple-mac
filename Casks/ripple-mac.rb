cask "ripple-mac" do
  version "0.4.0-beta.2"
  sha256 "dc72843786e6020074af5c222a8c0ef5ab4b832907850a31349c7d739846345a"

  url "https://github.com/vgs499p/ripple-mac/releases/download/v0.4.0-beta.2/Ripple-Beta-2-macOS-arm64.zip"
  name "Ripple Native"
  desc "Native Ripple client with local themes"
  homepage "https://github.com/vgs499p/ripple-mac"

  depends_on arch: :arm64
  depends_on macos: ">= :big_sur"
  app "Ripple Beta.app"
end
