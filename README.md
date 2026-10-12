<!--
[INPUT]: 已实现功能、开发配置、跨平台 PR 回归、双应用使用方式与 MIT/第三方许可边界。
[OUTPUT]: 面向公开仓库的英文产品首页、安装与开发入口、架构和贡献导航。
[POS]: 项目公开入口，连接产品功能、使用说明与模块导航。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
-->
<div align="center">
  <img src="docs/brand-logos.svg" width="320" height="149" alt="CCBaga-A and CCBaga-B logos, side by side" />
  <h1>CCBaga</h1>
  <p><strong>A native macOS, Linux and Windows toolkit for a cleaner workspace and an environment you can verify.</strong></p>
  <p>Claude identities · Browser fingerprints · System settings · Network Skill</p>
  <p>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-e8a181?style=flat-square" alt="MIT license" /></a>
    <img src="https://img.shields.io/badge/macOS-12%2B-111827?style=flat-square&amp;logo=apple&amp;logoColor=white" alt="macOS 12 and later" />
    <img src="https://img.shields.io/badge/Linux-x86__64-111827?style=flat-square&amp;logo=linux&amp;logoColor=white" alt="Linux x86_64" />
    <img src="https://img.shields.io/badge/Windows-10%2F11-111827?style=flat-square&amp;logo=windows&amp;logoColor=white" alt="Windows 10 and 11" />
    <img src="https://img.shields.io/badge/core-Rust%20%2B%20Tauri-111827?style=flat-square&amp;logo=rust&amp;logoColor=white" alt="Rust and Tauri" />
  </p>
  <p>
    <a href="#features">Features</a> ·
    <a href="#installation">Installation</a> ·
    <a href="#updates">Updates</a> ·
    <a href="#quick-start">Quick start</a> ·
    <a href="#architecture">Architecture</a> ·
    <a href="#development">Development</a> ·
    <a href="CONTRIBUTING.md">Contributing</a>
  </p>
</div>

![CCBaga: native control, observable environments](docs/readme-hero.png)

CCBaga brings local maintenance and environment setup into one focused desktop workflow. Inspect Claude data before removing it, rotate your companion browser's fingerprint and restart it, align system settings with a selected region, and compare real browser readings with the machine and network exit.

Two applications work together: **CCBaga-A** owns native operations; **CCBaga-B** runs an isolated Fingerprint Chromium profile. Browser measurements return to the host through a local session. Changing a setting and verifying its effect are separate steps.

## Features

<table>
  <tr>
    <td width="33%"><strong>Local Claude identities</strong><br /><br />Archive the current local identity, switch to a saved identity or start with an empty workspace. Choose what stays shared and what moves with each identity.</td>
    <td width="33%"><strong>Fingerprint &amp; restart</strong><br /><br />Generate a fresh seed, apply the selected region's language and timezone, then restart the companion browser.</td>
    <td width="33%"><strong>Real environment checks</strong><br /><br />Compare browser and machine readings with the actual network exit. Passing measured checks produces a green page and a large checkmark.</td>
  </tr>
  <tr>
    <td><strong>Native system settings</strong><br /><br />Apply timezone, preferred language and region settings. Open desktop controls for location and permissions; see platform-specific limits.</td>
    <td><strong>AI-guided network setup</strong><br /><br />Three simple steps, a coding-agent selector and a copyable Skill installation prompt. Give the AI your files and connection details; it handles the commands.</td>
    <td><strong>Claude Code CLI tools</strong><br /><br />Detect the installation method and version. Uninstall or reinstall through native, npm/bun, Homebrew or WinGet workflows.</td>
  </tr>
</table>

**One region selector. Four everyday actions.** Easy setup puts browser cache cleanup, fingerprint rotation, timezone/language reset and identity archiving on the same page. The sidebar keeps advanced controls in Network, Claude identities, Browser and Computer.

The interface supports **English and Chinese**, with shared YAML catalogs. Switching the interface language does not change the computer or browser language.

## Installation

**Platforms:** macOS 12 or later on Apple Silicon; Linux x86_64 with GTK 3 and WebKitGTK 4.1; Windows 10/11 x86_64. Intel macOS and Linux ARM64 are not supported.

Linux x86_64 packages include DEB, RPM, AppImage and Arch/pacman formats, with the companion browser included. See [Linux setup](docs/linux.md) for installation and platform limits. Official versions are published exclusively by the project owner, **liseami**.

Windows x86_64 uses one NSIS setup installer containing both A and B. The installer is not Authenticode signed and may show SmartScreen; managed security policy can block it. See [Windows setup](docs/windows.md) for installation, toolchains and platform behaviors.

### macOS

Download the official installer from [GitHub Releases](https://github.com/liseami/CCBaga/releases/latest). The distribution package is named `CCBaga-<version>-macos-arm64.dmg`. It contains both applications:

| Application | Installed bundle | Role |
| --- | --- | --- |
| CCBaga-A | `CCBaga-A.app` | Identity management, system settings and environment verification |
| CCBaga-B | `CCBaga-B.app` | Companion browser with a dedicated profile |

Download **one DMG**; it contains both applications. There is no separate browser download. The installer uses the same pixel-art style as this README.

1. Open the disk image and drag **both applications** into Applications.
2. Launch CCBaga-A. It starts the companion browser automatically.
3. Choose English at the bottom of the sidebar if needed.

Source builds are available through the [development guide](#development).

## Updates

Use **Check for updates** at the bottom of CCBaga-A’s sidebar. Startup checks quietly; installation only starts when you choose **Update and restart**. Downloads show actual progress and are verified before installation. Failed downloads can be retried; a failed restart does not download the update again.

On **macOS**, in-app updates replace A; use the latest DMG to update both apps. On **Windows**, a verified NSIS update installs both A and B after closing this companion browser. On **Linux**, use the package manager or full package.

## Quick start

### Everyday setup

Open **Easy setup**, select a country or region, then choose an action. Browser fingerprint rotation closes and restarts CCBaga-B, so save unfinished browser work first. Selecting a region changes language, timezone and regional settings; it does **not** change your public IP.

### Claude identities

Open **Claude identities** to manage the local state of both Claude Code and Claude Desktop. Choose **Empty identity** to archive the current state before leaving a fresh workspace, or choose a saved identity to restore it. Every switch saves the outgoing identity first.

Expand **Keep in workspace** to choose retention for skills and commands, plugins, MCP configuration, personal settings, project records, conversation history, and caches/logs. Login data always moves with an archived identity. Project source outside Claude's own data directories is not scanned. A confirmation names the outgoing identity and summarizes what stays in the workspace before closing recognized Claude processes.

Portrait identity cards scroll horizontally with spring motion and subtle hover tilt. The white empty identity stays first, followed by the current local account with available name, email and organization information. **Archive** saves the current state and leaves an empty identity. Select a saved card to restore that identity. **Direct delete** removes selected current data without creating an archive; deleting a saved card removes only that archive. Both deletion actions require an explicit confirmation.

A local SQLite database tracks names, policies and recovery state. Data is compressed into an authenticated, encrypted `.baga` container outside Claude's working directories. Archive verification precedes workspace changes; failed switches roll back, and an interrupted switch exposes recovery. Recovery also saves the pre-recovery state. Ordinary archive tools cannot open this container directly; filesystem ownership or administrator access is not an absolute secrecy boundary.

Expired or revoked server-side logins may need authentication again. The app manages the default local directories, not every custom Claude profile. The program-maintenance controls uninstall or reinstall Claude Code without removing the identity vault.

### Browser verification

Open **Browser** or the companion browser's local test page to measure the actual environment. Checks cover the network exit, country, timezone, language, platform, CPU count, engine version and location settings.

The success page appears only when the measured checks match, the report reaches the cleaner and no measurement error remains. **DNS and WebRTC leak checks are not independently verified.** A green result describes the measured checks, not a guarantee of anonymity or account acceptance.

### Network setup

Open **Network** for three steps: **Get VPS → Get residential proxy → Install Skill**. Select **General**, **Codex**, **Cursor**, **OpenCode** or **Copilot**, then copy the installation prompt into your local coding agent. The prompt follows the app's English/Chinese setting.

The [Network Skill](skills/ccbaga-network/README.md#installation) follows the Agent Skills specification and installs separately from the apps. It supports the npm `skills` CLI: `npx skills add liseami/CCBaga --skill ccbaga-network --global`; the AI selects the target tool, uses copy mode on Windows, and disables installer tracking. Give the AI your VPS address, the three matching PEM/PPK/PUB files or downloaded key bundle, and your residential SOCKS5 connection details. It handles file paths, private configuration, commands and checks; you do not need to open a terminal or edit JSON. Missing resources are purchased by you with the AI's guidance. Remote setup runs only after approval of the actual server changes.

The Skill supports authorized Debian 12/13 servers using root key login on port 22. It checks the SSH identity and fixed SOCKS5 exit, configures Xray/Reality and delivers a private local import file after server self-checks pass. Client setup uses **TUN ON**, **System Proxy OFF** and an empty explicit browser proxy; actual TUN/browser verification remains a separate step. The subscription URL uses HTTP and contains credentials; prefer the file retrieved over SSH.

Provider buttons link to [DMIT](https://www.dmit.io/aff.php?aff=28800) and [Webshare](https://www.webshare.io/?referral_code=qlm1mt9xibc9). Network setup is optional; local cleanup does not require a VPS or a proxy subscription. **The app does not collect SSH keys or proxy credentials and does not deploy to servers.**

## Architecture

![Pixel architecture map: native core, companion browser and a separate AI Network Skill](docs/architecture.svg)

The UI owns inputs and presentation. Rust owns validation, task locks, process identity checks and execution. Small Objective-C bridges call macOS APIs; Rust Linux and Windows boundaries use native process identity checks and platform controls. The browser test page has no Tauri command access; it exchanges context and measurements through a random-token loopback session.

| Layer | Technology | Responsibility |
| --- | --- | --- |
| Desktop host | Rust · Tauri 2 | Native commands, transactions, progress and lifecycle |
| Interface | React 19 · TypeScript · Cromma UI | Forms, previews and execution results |
| Platform bridges | AppKit · Security · CoreLocation · Linux / Windows Rust | Platform permissions and system integration |
| Companion browser | Fingerprint Chromium | Dedicated profile and real browser measurements |
| Separate Network Skill | Python standard library · OpenSSH · Bash · Xray | AI-managed private configuration, approved remote setup and server/client verification |
| Localization | YAML · Vite | Build-time English/Chinese catalogs |

<details>
<summary><strong>Repository map</strong></summary>

```text
assets/             Compressed brand logos and pixel-art DMG background
src/                Desktop UI, browser test page and localization
src-tauri/src/      Rust commands and transaction boundaries
src-tauri/native/   Small macOS API bridges
browser-runtime/    Engine metadata, launcher, entitlements and licenses
scripts/            Icons, local builds and installation helpers
skills/             Installable AI Network Skill and its execution scripts
packaging/arch/     Native Arch Linux PKGBUILD and desktop entry
docs/               README artwork and platform development guides
vendor/cromma-ui/   Vendored interface components
```

The large browser application and archive are generated/runtime inputs, excluded from Git. `CLAUDE.md` files map module boundaries; source headers describe inputs, outputs and responsibilities.

</details>

## Development

### Prerequisites

On Linux, follow [Linux setup](docs/linux.md). On macOS, use an Apple Silicon Mac with Xcode Command Line Tools, stable Rust and Bun. Follow [Tauri's macOS prerequisites](https://v2.tauri.app/start/prerequisites/#macos) for the native toolchain.

The browser engine is an external build input. `python3 scripts/fetch-browser.py` downloads the pinned macOS arm64 **Fingerprint Chromium 148.0.7778.215** engine from the [upstream project](https://github.com/adryfish/fingerprint-chromium), verifies its SHA-256 and preserves its bundle structure and notices. Browser runtime binaries are not included in the source repository.

From your checkout:

```sh
bun install --frozen-lockfile
python3 scripts/fetch-browser.py
sh scripts/make-icons.sh
sh scripts/package-browser.sh
bun run tauri dev
```

Local builds are for development and testing; they are not official project downloads.

### Fast local build and install (macOS)

```sh
bun run dev:install
```

This builds a debug app with the embedded UI, uses local ad-hoc signatures and installs/starts both apps. It does not use Apple credentials or create an installer. It preserves profiles and settings. The development installer accepts only locally built debug apps; downloaded installers still require the normal signature and Gatekeeper checks.

### Build

```sh
MACOSX_DEPLOYMENT_TARGET=12.0 \
  bun run tauri build --bundles app --features custom-protocol
```

The host bundle is written to `src-tauri/target/release/bundle/macos/CCBaga-A.app`. The companion bundle remains at `browser-runtime/Browser.app` until copied to Applications as `CCBaga-B.app`.

Production builds require `custom-protocol` to load the embedded UI. Compilation without it fails explicitly, preventing an installed app from depending on the development server.

### Checks

```sh
bun run build
bun test
MACOSX_DEPLOYMENT_TARGET=12.0 \
  cargo test --manifest-path src-tauri/Cargo.toml
python3 scripts/test-sign-browser.py
python3 skills/ccbaga-network/scripts/test_network.py
bash -n skills/ccbaga-network/scripts/setup-vps.sh
```

Pull requests run the existing frontend, native Rust, offline Skill and npm skill-installation checks on Ubuntu, macOS and Windows, plus the relevant platform regression tests. The workflow uses a read-only token, no signing secrets and no artifact uploads; it does not publish releases or download the browser engine. Complete engine and installer validation remains part of the separate delivery checks.

The identity UI simulation uses 40 fictitious accounts, mocked Tauri IPC and an isolated headless context with external requests blocked. It covers card navigation, retention choices, switching, irreversible-delete alerts, failure recovery and both languages. Run `bun run test:identities` with Node.js 22.23.2 or later after preparing the pinned browser engine. The browser receives a temporary HOME/XDG configuration. Native archive tests use temporary SQLite databases and files; the macOS credential bridge is also checked against a separate disposable Keychain.

Automated tests exercise disposable fixtures and local test servers. They do not deploy a real VPS, delete your Claude account data or perform administrator-authorized system changes. Real SSH connectivity, client-side TUN routing and permission-dependent operations require validation in the target environment.

## Data & permissions

- **Identities are local.** Login state is archived before switching. You choose which personal data stays in the workspace and which is stored with the identity. Actual project source and cloud-side account records are outside this scope.
- **Secrets stay out of UI logs.** The apps have no SSH key picker or server deployment command. After explicit confirmation, the native identity vault reads only the recognized default Claude credentials needed for restoration; credential contents never enter the GUI, command-line arguments or network. The separate Skill uses private local files and temporary private PEM copies (0600 on POSIX, verified user-only NTFS ACLs on Windows); data supplied to your coding agent is subject to that agent provider’s handling.
- **Network checks make real requests.** Exit detection uses `ipwho.is` with `ipapi.co` as a fallback on the same route. IP location is a service estimate.
- **Remote changes are explicit.** The separate Skill contacts only the VPS you provide after server-change approval. Required credentials remain in private local/server configuration, outside Git and public logs.
- **Permissions are feature-specific.** Timezone changes require macOS administrator authorization. Keychain access follows system prompts. Location controls may require Accessibility permission or a manual action in System Settings.

Custom `CLAUDE_CONFIG_DIR` locations are not automatically included in the default identity scope. The app reports this limitation rather than silently claiming they were switched.

## Contributing

Start with [CONTRIBUTING.md](CONTRIBUTING.md). Small, focused changes with clear behavior and relevant checks are welcome. For bug reports, include the operating system and version, app version, operation and sanitized error; never attach private keys, passwords or subscription URLs.

## License & credits

CCBaga's original project code is released under the **[MIT License](LICENSE)**. Third-party code and binaries retain their own terms; see [NOTICE.md](NOTICE.md).

Built with [Tauri](https://tauri.app/), [React](https://react.dev/) and [Fingerprint Chromium](https://github.com/adryfish/fingerprint-chromium), with a native macOS implementation of Claude maintenance workflows. Cleanup discovery was informed by the [claude-cleanup skill](https://github.com/suyuan2022/suyuan-skill/tree/main/claude-cleanup).

<div align="center">
  <sub>An independent project. Not affiliated with or endorsed by Anthropic or Apple.</sub>
</div>
