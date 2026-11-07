# Homebrew cask

The Mac release is a GUI app, so it is distributed as a cask. This repository can also act as its own tap:

```sh
brew tap vgs499p/ripple-desktop https://github.com/vgs499p/ripple-desktop
brew install --cask vgs499p/ripple-desktop/ripple-mac
```

The cask needs the corresponding GitHub release archive to be published. Generate it from the final archive to keep the download checksum exact:

```sh
python3 scripts/make-cask.py --archive /path/to/Ripple-Beta-5-macOS-arm64.zip --version 0.4.0-beta.5 --url https://github.com/vgs499p/ripple-desktop/releases/download/v0.4.0-beta.5/Ripple-Beta-5-macOS-arm64.zip --output Casks/ripple-mac.rb
```

The current build supports Apple silicon and macOS 11+. It is ad-hoc signed, not Developer ID signed or notarized. Downloaded builds can therefore require macOS approval before opening. The cask does not disable Gatekeeper or remove quarantine.

For an Intel and Apple silicon release, provide `--intel-archive` and `--intel-url` alongside the ARM archive and URL. The generated cask selects the correct download and verifies a separate checksum for each architecture.
