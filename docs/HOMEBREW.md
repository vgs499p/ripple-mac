# Homebrew cask

The Mac release is a GUI app, so it is distributed as a cask. This repository can also act as its own tap:

```sh
brew tap vgs499p/ripple-mac https://github.com/vgs499p/ripple-mac
brew install --cask vgs499p/ripple-mac/ripple-mac
```

The cask needs the corresponding GitHub release archive to be published. Generate it from the final archive to keep the download checksum exact:

```sh
python3 scripts/make-cask.py --archive /path/to/Ripple-Beta-2-macOS-arm64.zip --version 0.4.0-beta.2 --url https://github.com/vgs499p/ripple-mac/releases/download/v0.4.0-beta.2/Ripple-Beta-2-macOS-arm64.zip --output Casks/ripple-mac.rb
```

The current build supports Apple silicon and macOS 11+. It is ad-hoc signed, not Developer ID signed or notarized. Downloaded builds can therefore require macOS approval before opening. The cask does not disable Gatekeeper or remove quarantine.
