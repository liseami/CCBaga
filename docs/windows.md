<!--
[INPUT]: Windows 原生实现、NSIS 安装契约与可执行平台验收。
[OUTPUT]: 安装、更新、平台功能和开发依赖的英文说明。
[POS]: README 的 Windows 入口；只描述已实现边界，真实桌面权限验证独立记录。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
-->
# Windows

## Install and update

Download **one** `CCBaga-<version>-windows-x86_64-setup.exe` from [Releases](https://github.com/liseami/CCBaga/releases/latest). It installs CCBaga-A and the complete CCBaga-B browser engine for the current user. Start-menu shortcuts open either application; both use the same dedicated profile. The installer downloads Microsoft's WebView2 runtime if it is missing.

The installer does not have an Authenticode certificate. Windows may show SmartScreen; ordinary installations can use **More info → Run anyway**. Organization policy or Smart App Control may prevent this. Do not disable security policy to install the app. Compare the download's SHA-256 with `SHA256SUMS`.

The sidebar checks for updates only from the official HTTPS manifest. Tauri verifies the installer signature and version, closes this companion browser, then exits A for the update. Windows updates install **both A and B**. Save unfinished work first. A manual reinstall also preserves profiles and personal Claude assets. By default, uninstall removes the programs and their own shortcuts/registrations, preserving profiles, skills and projects. The standard installer also offers an explicit **Delete application data** checkbox; selecting it removes CCBaga’s own saved profile and settings.

## Platform behavior

| Operation | Windows implementation |
| --- | --- |
| Timezone | Maps the selected IANA zone to a supported Windows ID, uses `tzutil /s`, then reads the actual dynamic timezone through the Windows API. Insufficient permission opens Date & time settings and reports failure. |
| Language and regional format | Uses the current-user International cmdlets, preserving other languages and input methods. Display language can require an installed language pack and sign-out/sign-in; a pending preference is not reported as an active language change. |
| Default browser | Registers B's capabilities and HTTP/HTTPS handlers under HKCU, opens Windows Default apps for user confirmation, then reads both actual associations separately. It does not overwrite protected UserChoice settings. |
| Fingerprint and restart | Normally closes only the browser using CCBaga's exact profile, updates the seed/region preferences, and restarts the bundled engine. Other Chrome profiles are unaffected. |
| Claude Code CLI | Recognizes official native, global npm/bun and current-user WinGet installations. Uses the detected installation method for uninstall/reinstall and verifies the resulting version. Native reinstall downloads, decodes and parses the official bootstrap script before removing the old program, preserving it when the bootstrap download or validation fails. Custom launchers are preserved and automatic modification is refused. |
| Claude identities | Uses private encrypted archives, SQLite recovery state, user-controlled retention, fixed file targets, current-user ownership, snapshots and reparse-point checks. Process termination checks the user's SID, exact executable/script, creation time and live handle. Credential Manager passwords are never enumerated or extracted; its entries are not automatically deleted. |
| Location | Opens the Windows privacy control when a disabled state cannot be verified. Managed policies can require an administrator. |
| Network | The app only provides provider buttons and a Skill prompt. SSH, supplied keys, private configuration and authorized deployment belong to the separately installed Skill. No VPS configuration UI is present. |

The x64 engine supports native Windows; no WSL installation is required. Windows on ARM is not a native target. Real desktop language packs, user-confirmed default associations, corporate policies, real VPS credentials and TUN routing require verification on the target machine. Hosted Windows runners test Windows API/runtime behavior but are Windows Server environments, not a substitute for every Windows 10/11 edition.

## Development

Use Rust's x64 **MSVC** toolchain, Visual Studio C++ Build Tools, WebView2, Bun and Python 3.12+. Follow [Tauri's Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows). For the network Skill, use Microsoft's native OpenSSH Client and `curl.exe`; the AI handles missing prerequisites after explaining required permissions.

```powershell
bun install --frozen-lockfile
python scripts/fetch-browser.py
bun run tauri dev
```

For an embedded frontend and NSIS installer:

```powershell
bun run tauri build --bundles nsis --features custom-protocol
```

The fixed upstream ZIP is verified by SHA-256, PE x64 architecture and product version before extraction into `browser-runtime/WindowsBrowser`. Large engine files and installers are excluded from Git. No Apple credentials are needed for Windows development.

Automated Windows checks use disposable CLI programs, private-key fixtures, NTFS permissions and a real installer. The installer check covers install, A/B launch and local communication, payload hashes, upgrade, uninstall and retained assets. The same installer is exercised on both Windows 2025 and 2022 hosted runners. Signing checks use Tauri's official updater without installing the test download.

References: [Windows installers](https://v2.tauri.app/distribute/windows-installer/), [Tauri updater](https://v2.tauri.app/plugin/updater/), [Windows timezone utility](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/tzutil), [International cmdlets](https://learn.microsoft.com/en-us/powershell/module/international/), [Default apps](https://learn.microsoft.com/en-us/windows/apps/develop/launch/launch-default-apps-settings), [Claude Code setup](https://code.claude.com/docs/en/setup).
