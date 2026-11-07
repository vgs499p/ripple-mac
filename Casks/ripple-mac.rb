cask "ripple-mac" do
  version "0.4.0-beta.6"
  on_arm do
    sha256 "abe498fe940bd62eb25c67e3e71977440c8c7d85929dff89ffd7899d10c1df02"
    url "https://github.com/vgs499p/ripple-mac/releases/download/v0.4.0-beta.6/ripple-native-macos-arm64.zip"
  end
  on_intel do
    sha256 "600a0f99934b6adbac898dcbed83a8c6580ca23cdcff656ee271328cc73a24a0"
    url "https://github.com/vgs499p/ripple-mac/releases/download/v0.4.0-beta.6/ripple-native-macos-x86_64.zip"
  end
  name "Ripple Native"
  desc "Native Ripple client with local themes"
  homepage "https://github.com/vgs499p/ripple-mac"
  depends_on macos: ">= :big_sur"
  app "Ripple Beta.app"
end
