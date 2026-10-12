<!--
[INPUT]: Linux 原生实现、构建配方与实际验证边界。
[OUTPUT]: 社区开发依赖、源码构建和平台限制；不提供官方发布流程。
[POS]: README 的 Linux 开发入口，与原生平台边界对应。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
-->
# Linux

CCBaga-A builds on Linux x86_64. The pinned Fingerprint Chromium 148.0.7778.215 Linux engine is bundled with A; CCBaga-B is a desktop launcher into the same isolated browser profile. Both launch paths start the host and its authenticated local measurement session. There is no upstream Linux ARM64 engine for this pinned version.

## Installation

Download the Linux x86_64 package for your distribution from [GitHub Releases](https://github.com/liseami/CCBaga/releases/latest): DEB for Ubuntu/Debian, RPM for compatible Fedora systems, AppImage for compatible desktops, or `.pkg.tar.zst` for Arch/pacman. A and its companion B engine are included in each package.

Ubuntu packages and AppImages are built on Ubuntu 24.04; older glibc distributions are not validated. Arch packages are built separately on Arch. Linux updates use the package manager or a new downloaded package; the in-app updater currently supports macOS only.

## Dependencies

Use stable Rust, Bun, Python 3.12+ (safe tar extraction), a C compiler, pkg-config, and the [Tauri Linux prerequisites](https://v2.tauri.app/start/prerequisites/#linux). Runtime operations use zsh, curl and xdg-utils. OpenSSH is needed for the separately installed Network Skill and its offline tests; the app has no SSH file chooser or deployment command. Clipboard copying uses wl-clipboard on Wayland and xclip on X11. A tray indicator requires desktop AppIndicator support; closing the window hides it, and launching A again restores it.

Ubuntu 24.04 / Debian 13:

```sh
sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev patchelf python3 curl zsh \
  openssh-client xdg-utils wl-clipboard xclip libnss3 libnspr4 \
  libgbm1 libasound2t64 libxss1
```

Arch Linux:

```sh
sudo pacman -S --needed base-devel webkit2gtk-4.1 gtk3 libayatana-appindicator \
  librsvg patchelf python curl zsh openssh xdg-utils wl-clipboard xclip \
  nss nspr mesa alsa-lib libxss
```

Chromium uses its normal Linux sandbox. Run as a regular user with working user namespaces; do not add `--no-sandbox`. AppImage packaging may need additional Tauri bundler tools and FUSE support. Build packages on the oldest distribution you intend to support: a build on a recent Arch installation does not establish compatibility with older glibc systems.

## Local Arch Linux build

Arch Linux and Arch-based desktops such as Omarchy are supported on x86_64. A native pacman package recipe is included; it is not an AUR publication. Install `base-devel`, `bun`, `nodejs`, `rust` (or `rustup` with a stable toolchain), `git`, `pkgconf`, `python`, `cmake` and `openssh` (Skill tests), plus the runtime dependencies above, then:

```sh
cd packaging/arch
makepkg -f
sudo pacman -U ./ccbaga-git-*.pkg.tar.zst
```

The recipe builds upstream `main`, downloads the pinned browser with SHA-256 verification, and installs the A/B desktop entries and icons. It preserves the bundled engine instead of stripping vendor ELF files. To build this checkout before its changes are merged upstream, run from `packaging/arch`:

```sh
CCBAGA_GIT_URL="file://$(git rev-parse --show-toplevel)" \
  CCBAGA_GIT_REF="commit=$(git rev-parse HEAD)" makepkg -f
```

Only committed changes are included. The package does not enable services, change desktop settings, or change your default browser. Omarchy/Hyprland uses the Wayland clipboard helper; desktop language and location settings remain subject to the limits below.

## Build and install

```sh
bun install --frozen-lockfile
python3 scripts/fetch-browser.py
bun run build
bun test
cargo test --locked --manifest-path src-tauri/Cargo.toml
python3 scripts/test-linux.py
python3 skills/ccbaga-network/scripts/test_network.py
bun run tauri build --features custom-protocol
```

The fetcher checks the pinned publisher SHA-256, ELF architecture and actual browser version before preparing `browser-runtime/LinuxBrowser`. It refuses to overwrite an existing prepared engine. Linux builds use the existing PNG icon and platform-native tools. DEB, RPM and AppImage keep the vendor browser in `/usr/libexec/CCBaga-B`, separate from host libraries so AppImage tooling cannot rewrite its binaries. Source and Arch installations retain their existing resource layout.

Local Tauri builds produce DEB and AppImage under `src-tauri/target/release/bundle`. RPM downloads use native RPM tooling with the same application file tree. Install the package matching your distribution, or make the AppImage executable and run it. The pinned browser and its notices travel with the package. Source builds are for development. Official versions are provided only by liseami; Linux updates use a new installation package, not the macOS in-app update feed.

For a local source installation without a package manager:

```sh
bun run tauri build --no-bundle --features custom-protocol
bun run install:apps
~/.local/bin/ccbaga-a
```

This installs application files under `~/.local/share/ccbaga-install`, creates A/B entries under `~/.local/share/applications`, and refuses to replace an unrelated launcher or a running installation. It leaves user profiles and settings intact. A custom prefix is supported with `sh scripts/install-apps.sh --prefix /absolute/prefix`; include that prefix's `share` directory in `XDG_DATA_DIRS` if needed. Installation does not change the default browser. The browser page's explicit default-browser action registers the persistent installed executable (or the AppImage path) and verifies both HTTP and HTTPS associations.

## Features and limits

| Feature | Linux behavior |
| --- | --- |
| Claude identities | SQLite metadata and encrypted local archives; snapshot, owner, symlink and process checks. Users choose which personal data stays in the workspace. Project source is not scanned. |
| Paths | Shared `.claude` targets plus Claude CLI cache in `XDG_CACHE_HOME` (default `~/.cache`); Claude Desktop cache/logs in `XDG_CONFIG_HOME` (default `~/.config`). Absolute custom XDG paths are supported. |
| Third-party Claude Desktop | Only explicit cache/log paths are scanned. Full desktop account reset is intentionally unavailable because Linux desktop distributions have differing layouts. |
| Credentials | Default file-based CLI credentials move with the identity. Secret Service/KWallet cleanup and unlock are unavailable and produce an explicit warning/error. No empty-success keyring result is fabricated. |
| CLI install management | Existing native and npm flows, including nvm. Requires zsh; Homebrew cask detection runs only on macOS. |
| Browser | Pinned Linux fingerprint engine, isolated Default profile, cache/site-data separation, rotation and real observations. Linux platform is checked; Chromium does not expose the kernel release through UA client hints, so that field is not compared with `uname`. |
| Network route | Use TUN or the explicit browser proxy. Native exit probes can use proxy environment variables; GNOME/KDE desktop proxy settings and PAC are not independently discovered. Proxy credentials stay in native code. |
| Network | Fixed provider/Skill links open through xdg-open; the selected coding agent handles the separate Skill, files and authorized deployment. |
| Time zone | Reads zoneinfo; `timedatectl set-timezone` uses systemd/polkit authorization and verifies the result. Desktop automatic time-zone policies remain separate. Non-systemd systems return an actionable error. |
| Language/region | Reads login-session language (`LC_MESSAGES`) and regional time formats (`LC_TIME`), respecting `LC_ALL`. Open GNOME/KDE settings, change language/formats, then sign out and back in. Existing matching session settings are verified. A differing language/region returns a manual-action error before the region action changes the time zone. |
| Location | GNOME's location preference can be read, disabled and reread. Other desktops return unknown, never disabled. Full machine alignment stops before modifying settings on desktops with unknown location state. Portal/other location services are not independently verified; do not treat this preference as a global privacy guarantee. |
| Settings launcher | GNOME and KDE panels; other desktops return a manual-action message. |

Linux keyring and third-party desktop reset, session-language changes, and independent portal/location verification remain platform limitations. A Linux build passing local tests does not validate every distribution, display server, desktop, privileged operation or remote VPS.
