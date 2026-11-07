# Desktop builds and packages

The Desktop packages workflow builds Intel and Apple silicon macOS apps, Linux x86_64 DEB/RPM/tar archives, and a portable Windows x86_64 ZIP. FreeBSD, OpenBSD, NetBSD and DragonFly BSD each build natively in a separate VM with the software renderer; only successful builds produce downloadable archives. Windows and BSD GUI behavior is untested. No platform is described as tested solely because it compiled.

Linux packages install the binary, desktop entry, icon and license notices. GTK 3 provides file pickers. Linux GPU builds need Vulkan or a supported graphics driver. BSD archives install under /usr/local; extract to a staging directory and use your package manager or install the staged files. Do not mix binaries between BSD variants or OS releases.

Build locally with `cargo build --release --locked -p ripple-mac`. For CPU rendering use `--no-default-features --features software`. On Linux, install GTK 3, xkbcommon, Wayland and fontconfig development packages. On BSD use their native Rust and GTK packages. Run `python3 scripts/package-unix.py --format deb` or `--format rpm` after a native Linux build; RPM needs rpmbuild, DEB needs dpkg-deb. `--format tar` works on Linux and BSD.

macOS saves sessions in Keychain. Other platforms currently keep sign-in in memory for the process and report that secure persistence is unavailable. No plaintext cookie fallback is used. File pickers and theme import/export are available on all desktop builds. Preferences follow APPDATA on Windows and XDG_CONFIG_HOME on Linux/BSD.

Manual builds upload workflow artifacts. Supply a release_tag matching the Cargo version to publish a prerelease after the main desktop jobs pass. A failed BSD job remains visible and never produces a fake binary.
