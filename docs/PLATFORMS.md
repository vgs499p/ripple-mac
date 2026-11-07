# Desktop builds and packages

The Desktop packages workflow builds Intel and Apple silicon macOS apps, Linux x86_64 DEB/RPM/tar archives, and a portable Windows x86_64 ZIP. FreeBSD, OpenBSD, NetBSD and DragonFly BSD each build natively in a separate VM with the software renderer; only successful builds produce downloadable archives. Windows and BSD GUI behavior is untested. No platform is described as tested solely because it compiled.

Linux packages install the binary, desktop entry, icon and license notices. GTK 3 provides file pickers. Beta 7 Linux packages use CPU rendering and do not require Vulkan. Beta 6 used the GPU backend and can panic during startup on older graphics hardware. BSD archives install under /usr/local; extract to a staging directory and use your package manager or install the staged files. Do not mix binaries between BSD variants or OS releases.

Build locally with `cargo build --release --locked -p ripple-mac`. For CPU rendering use `--no-default-features --features software`. On Linux, install GTK 3, xkbcommon, Wayland and fontconfig development packages. On BSD use their native Rust and GTK packages. Run `python3 scripts/package-unix.py --format deb` or `--format rpm` after a native Linux build; RPM needs rpmbuild, DEB needs dpkg-deb. `--format tar` works on Linux and BSD.

macOS saves sessions in Keychain. Other platforms currently keep sign-in in memory for the process and report that secure persistence is unavailable. No plaintext cookie fallback is used. File pickers and theme import/export are available on all desktop builds. Preferences follow APPDATA on Windows and XDG_CONFIG_HOME on Linux/BSD.

Manual builds upload workflow artifacts. Supply a release_tag matching the Cargo version to publish a prerelease after the main desktop jobs pass. A failed BSD job remains visible and never produces a fake binary.

## Beta 6 build results

Intel and Apple silicon macOS, Linux x86_64, Windows x86_64, FreeBSD and OpenBSD compiled successfully on native runners/VMs. Windows and BSD GUI interactions have not been tested. Linux package startup checks caught and corrected an X11 runtime dependency. NetBSD compiled successfully after installing the official X distribution sets in the VM image. DragonFly's available Rust 1.85.1 is below the dependency minimum of Rust 1.88, so no DragonFly binary is published.

## Runtime baselines

- Both Mac architectures declare macOS 11.0. Older-version GUI compatibility has not been verified.
- Linux x86_64 builds use Ubuntu 22.04 and require glibc 2.35+, GTK 3, libxkbcommon and libxkbcommon-x11.
- Windows x86_64 targets Windows 10+ and requires the supported Visual C++ v14 runtime; GUI behavior is untested.
- Current BSD archives are built natively on FreeBSD 15.1, OpenBSD 8.0 and NetBSD 11.1, respectively. These are exact build baselines, not established backwards-compatible minimums.
