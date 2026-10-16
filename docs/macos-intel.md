<!--
[INPUT]: 2026-10-16 的固定引擎、SDK 15.2 原生构建、真实有窗口/宿主回环观测、候选安装盘与所有者审核交接。
[OUTPUT]: Intel 完整支持的首个依赖、固定源码构建步骤、候选验收与尚未通过的产品交付门槛。
[POS]: Intel 引擎开发说明；不宣称存在可安装的 Intel 版 CCBaga。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
-->
# macOS Intel port

**Status: the port is aligned with owner main/release base `30a171f`. The accepted Intel engine is unchanged. The current optimized host build, 22 frontend tests, 53 UI/profile checks, 96 native Rust tests and isolated Security checks passed, as did two real optimized-host companion reconnect/normal-quit cycles. The current complete ad-hoc signed development DMG passed payload/layout checks; the unchanged formal verifier rejects its missing Gatekeeper approval. Owner visual approval, fresh installation/full identity acceptance, formal signing/notarization and public-release acceptance remain pending. No accepted public Intel installer is available yet.**

The port shares the complete React UI, catalogs, styling, Rust/native operations, browser arguments, bundle identifiers and identity storage with arm64. No browser function is removed. It preserves the installed Xcode version and uses `DEVELOPER_DIR` only within build commands.

The pinned upstream macOS DMG (`148.0.7778.215`, SHA-256 `b72f091e2e1a7583eed389c4b8e3534ed355e568af8c8bbf8fc30a25e23ca679`) contains only arm64 code in both Chromium and Chromium Framework. Intel prebuilt support remains an [open upstream request](https://github.com/adryfish/fingerprint-chromium/issues/46). Complete Intel support starts by building the same fingerprint engine for x86_64.

On 2026-10-16, a fresh download matched the pinned digest, and a read-only DMG inspection with `lipo -archs` and `file` again identified both binaries as arm64 only. The [148 release assets](https://github.com/adryfish/fingerprint-chromium/releases/expanded_assets/148.0.7778.215) include one macOS DMG; their explicitly named x64/x86_64 binaries target Windows and Linux. At that check, the newer [150 release assets](https://github.com/adryfish/fingerprint-chromium/releases/expanded_assets/150.0.7871.186) contained Windows/Linux binaries and source archives, with no macOS binary. These versioned inputs are the basis for the Intel source build below.

## Pinned inputs

| Input | Fixed commit |
| --- | --- |
| [Fingerprint Chromium 148.0.7778.215](https://github.com/adryfish/fingerprint-chromium/tree/13b89eae304123f0710f2d33fd0816a2f61d7ffc) | `13b89eae304123f0710f2d33fd0816a2f61d7ffc` |
| [ungoogled-chromium macOS 148.0.7778.215-1.1](https://github.com/ungoogled-software/ungoogled-chromium-macos/tree/6a00dfe08591a15cfb0aed0edd292ba789d9a0d7) | `6a00dfe08591a15cfb0aed0edd292ba789d9a0d7` |

The macOS builder's original ungoogled submodule is `94344c470df2ac723cdf27f2914cb46025d83b20`. The fingerprint commit descends from that exact commit with one additional commit containing its fingerprint patches. The new entry point substitutes the pinned fingerprint repository for that submodule and uses the fixed macOS build/download configuration. Download hashes remain enforced by the pinned upstream tools. It compiles `chrome` with `target_cpu="x64"`, release flags and debug symbols disabled.

## Build environment

The local `macos-sdk15-fileport.patch` retains the same Mach file-descriptor transport when an older SDK does not publish `sys/fileport.h`. SDK 15.2 still exports `fileport_makeport` and `fileport_makefd` from `libsystem_kernel`; the patch supplies their C declarations only when the header is absent. The declarations follow [Apple's XNU interface](https://github.com/apple-oss-distributions/xnu/blob/xnu-10063.141.1/bsd/sys/fileport.h). No SDK files or transport behavior change. The builder accepts an already-applied patch on resume, rejects a mismatching source tree, and includes the patch and its SHA-256 in candidate provenance. The build requires the standard `patch` command.

Use a **native Intel Mac**, an existing full Xcode with macOS SDK **15 or newer**, Python **3.11–3.13**, Git, Ninja, Node.js and a `greadlink` command supporting `-f`. The pinned source declares `mac_sdk_min="15"`; SDK 26.2 is its official-build SDK, not its general minimum. Xcode 16.2 / SDK 15.2 therefore passes the source minimum check; actual compilation remains the compatibility proof. The engine runtime deployment target remains macOS 12. Install Python `PySocks` and `httplib2` in a dedicated virtual environment.

Use a dedicated local APFS directory without spaces. The default initial budget is **100 GiB free**; `--minimum-free-gib` explicitly selects a smaller initial working budget. Every child command is monitored and stopped if the source/build volume drops below **10 GiB free**. Actual peak usage is not yet measured. `--cache-dir` can move verified source/toolchain archives to another volume while keeping metadata-heavy source/output on local APFS. Start with `--jobs 2` on a 16 GB machine and monitor memory/disk usage. No completion time has been measured for this configuration.

`macos-sdk15-coregraphics.patch` uses the existing `kCGBitmapByteOrder32Host` constant in the two macOS Skia/Blink bitmap-context call sites. SDK 15.2 defines it as the same 32-bit host byte order (little endian on Intel); the newer `kCGImageByteOrder32Host` spelling is not present in that SDK. The alpha flags, BGRA layout and rendering paths are unchanged. This second patch is also recorded and copied into candidate provenance.

`macos-sdk15-appkit.patch` supplies the `NSScreen.CGDirectDisplayID` getter declaration only for SDKs older than 26. It matches [Apple's property declaration](https://developer.apple.com/documentation/appkit/nsscreen/cgdirectdisplayid-7uvhw?language=objc). The original `@available(macOS 26, *)` branch and older-system `deviceDescription` fallback remain intact; no new getter is called on this macOS 14 machine. This patch also becomes part of candidate provenance.

`macos-sdk15-pasteboard.patch` supplies the SDK 15.4 pasteboard access enum and getter through the existing Objective-C utility header, imported by all three native callers. It follows [Apple's pasteboard policy interface](https://developer.apple.com/documentation/appkit/nspasteboard/accessbehavior-swift.enum?language=objc) and the [published native enum values](https://learn.microsoft.com/en-us/dotnet/api/appkit.nspasteboardaccessbehavior?view=net-macos-26.0-10.0). The original macOS 15.4 guards and all default/ask/allow/deny branches are preserved. No clipboard access or system setting is changed by compiling these declarations.

`macos-headless-shutdown.patch` moves the existing macOS application keep-alive release before the weak browser/window check in the headless command completion callback. The first real probe rendered its DOM correctly, but the macOS application keep-alive remained set after the command's browser window closed. The patch covers that already-closed window case without changing headed startup, page code or fingerprint arguments. It is recorded with the SDK patches in candidate provenance. Its cached incremental build and all six uninstrumented probes passed; the verifier still requires normal exit within 60 seconds and does not kill a hanging browser to accept its DOM output.

Select Xcode using `DEVELOPER_DIR` in the build shell and activate the Python environment. The script does not change the global Xcode selection or install tools. From the repository:

```sh
python scripts/build-macos-intel-engine.py --work-dir /path/to/ccbaga-intel-build --check
python scripts/build-macos-intel-engine.py --work-dir /path/to/ccbaga-intel-build --prepare-only
python scripts/build-macos-intel-engine.py --work-dir /path/to/ccbaga-intel-build --jobs 2
```

An optional example for a constrained local workspace is `--minimum-free-gib 35 --cache-dir /path/to/separate-cache`; this retains the 10 GiB stop threshold. A build stopped during Ninja can resume after more space becomes available. It does not clear user files automatically.

Preparation downloads source, applies the fingerprint and macOS patch sets, retrieves pinned compiler resources and generates Ninja files. The local SDK compatibility patches are checked before either compilation or a preparation-only exit. Repeating the build command resumes Ninja after a compilation failure. Interrupted preparation is rejected because blindly reapplying patches could create a mixed tree; preserve the directory for diagnosis and prepare a new dedicated directory. Existing unrelated files and another source version are refused.

## Candidate acceptance

The build uses local ad-hoc signing with the existing nested signing order and strips Apple/updater signing credentials from child environments. Its candidate stays in the work directory. It does not replace `browser-runtime/Browser.app`, install applications, notarize or publish.

Before exporting a candidate ZIP, `verify-macos-intel-engine.py` requires:

- All Mach-O executables/libraries, including nested helpers, to contain x86_64 code; bundle symlinks must remain inside the app.
- Bundle metadata and the actual executable to match the pinned version.
- Bundle runtime metadata to retain macOS 12 support.
- Two real headless runs to report the requested CPU counts, timezones, languages and Mac platform. A stock Chromium binary reporting the correct version cannot satisfy this check.
- Four additional software headless runs compare two seeds with the product's existing `--disable-spoofing` modes. Canvas pixel reads, PNG encoding, text width, bounding rectangles and client rectangles must be stable within a page, change with the seed in fingerprint mode, and ignore the seed in native graphics mode. Relative positioning intentionally exercises the upstream ClientRects patch; fixed absolute positions are exempt in that patch. Reports label each seed and mode. This check passed on the actual native Intel engine; it does not cover hardware GPU rendering.

The probes use temporary HOME/profile directories, a data URL, mock Keychain access, disabled background networking and a dead local proxy. They do not load account data or external test sites. Rosetta cannot substitute for native Intel acceptance. Each successful `artifacts/<generation>/` includes a candidate ZIP, SHA256SUMS, the fingerprint license and JSON source/SDK/flags/observation records. A failed export remains in a hidden `.candidate-*` directory; it is not accepted. Only a complete generation with matching source record and checksums can advance to application integration.

The first accepted native Intel engine archive is `FingerprintChromium-148.0.7778.215-macos-x86_64.zip`, SHA-256 `15c8377d688bb2005cbc900cc71088e910999fc2833d031285e2765464eb6566`, built with SDK 15.2 and the five recorded source patches. Its generation contains `engine-verification.json`, `engine-source.json`, `SHA256SUMS`, the license and copies of every patch. The accepted archive and these records are available as [contributor validation assets](https://github.com/Uncle-Gao/CCBaga/releases/tag/intel-engine-148.0.7778.215-x86_64-validation), explicitly marked as a prerelease for owner review. This does not constitute a formal CCBaga application release; upstream CI variables remain owner-controlled.

## Application and delivery integration

After a real candidate passes, prepare the normal ignored engine input with its recorded archive and checksum:

```sh
python scripts/fetch-browser.py --macos-archive /path/to/accepted/engine.zip --macos-sha256 <recorded-sha256>
```

The importer checks all embedded architectures and runs the real Intel fingerprint probes before publishing `browser-runtime/Browser.app`. It rejects unsafe ZIP paths, symlink escapes, wrong hashes and partial/mixed-architecture engines. The existing `bun run dev:install` then builds the same debug application and browser launcher for the native architecture.

Both macOS release jobs use the same signing, notarization, UI/profile and installer checks. Intel's accepted engine is a separate input configured through the repository variables `CCBAGA_INTEL_ENGINE_URL` and `CCBAGA_INTEL_ENGINE_SHA256`; use the candidate's immutable HTTPS archive and recorded digest. An unset input blocks the Intel release instead of dropping browser functionality or substituting a stock engine.

Each architecture emits its own `CCBaga-<version>-macos-<arch>.dmg`, signed `.app.tar.gz`, `latest-macos-<arch>.json`, `macOS-<arch>-SHA256SUMS` and internal notarization record. The assembly step requires both complete macOS inputs before creating public `latest.json` with `darwin-aarch64`, `darwin-x86_64` and `windows-x86_64`. It checks that each target references its own archive/signature. Public delivery contains seven installers, two macOS updates, the merged manifest and checksums; both applications retain their existing names and data identifiers.

## Earlier application acceptance checkpoint

The first application acceptance checkpoint below used project base `e5df9fa18cd78bc105e4176fc9ee514b25cb437b`. Its engine inputs and digest are fixed separately above. The current owner-aligned application checkpoint is recorded separately below.

On the existing Intel machine using Xcode 16.2 / SDK 15.2, with global `xcode-select` unchanged:

- The first full engine build completed all 55,418 Ninja tasks, including the final ThinLTO link, application assembly and ad-hoc signing. All embedded Mach-O architectures, bundle metadata and the real `--version` passed the verifier's prechecks. This establishes compilation of the four SDK patches, but does not establish complete engine runtime acceptance.
- The initial headless probe exceeded the 60-second exit deadline after producing correct fingerprint values. The shutdown patch then compiled and linked through the cached build with six Ninja workers. All six uninstrumented probes passed with normal exit: CPU 7 / Tokyo / en-US, CPU 11 / New York / fr-FR, and the four seeded/native graphics comparisons. The temporary log-only diagnostic library is outside the repository and is absent from the accepted candidate.
- The real accepted ZIP's checksum was recomputed, then `fetch-browser.py` validated ZIP paths, all embedded architectures and all six real probes before atomically preparing `browser-runtime/Browser.app`. Both the source build/export and actual import returned exit status zero.
- The actual Tauri debug/custom-protocol build produced `CCBaga-A.app`; branding produced the complete `CCBaga-B` bundle with the native `CCBrowser` launcher. Both retained their expected identifiers and macOS 12 bundle metadata, passed deep/strict ad-hoc signature verification and contained x86_64 code throughout. The host's embedded browser page references passed. All 14 Mach-O files across A/B also passed deployment-target inspection with the existing Xcode's `vtool`; no file declares a minimum newer than macOS 12. This is metadata evidence, not execution on macOS 12.
- The existing `test-identities-ui.ts` completed 45 checks with the real imported Intel browser, including 40-account UI fixtures, browser verification, network states, residue/operation flows and persistent profile separation/reopen checks. IPC is simulated in that suite; it does not perform native account changes.
- A separate temporary diagnostic started the actual debug host and launched the branded `CCBrowser` entry twice with a sandboxed, headed browser and explicit temporary profiles. Native `Browser.m` recognized the running profile and requested normal quit on both cycles. Virtual Cookie/LocalStorage data survived the quit/reopen; native-graphics rectangle dimensions, pixels and text width were correct. The browser reported GPU compositing, rasterization, video decode/encode, WebGL and WebGPU enabled on this Intel Mac. This establishes local headed startup/rendering and native quit, not exhaustive hardware/media feature coverage or owner visual acceptance.
- The optimized Tauri host build completed with the existing release profile (`opt-level=s`, LTO and one codegen unit), and its x86_64 architecture, deep/strict signature and embedded browser page references passed. Two real optimized-host start/normal-quit cycles reconnected an already-running staged browser profile. The actual bundled page displayed only the virtual account's allowed name/email/organization, fetched native context and posted observations accepted by the real Rust report endpoint. Both reports retained CPU 8, Tokyo, en-US, MacIntel, macOS 14.8.7, Chromium 148.0.7778.215 and denied geolocation. External requests were intentionally blocked and the page correctly retained an exit-check failure. Host normal quit removed `start.url`; the browser and profile survived the host restart and then quit through the actual native bridge. No mock IPC was used in this diagnostic. The first temporary directory spelling traversed macOS's `/var` symlink and was refused by the existing path checks; resolving the test home to its canonical `/private/var` path preserved those checks and passed.
- A 340 MiB optimized development DMG was built with the existing `dmg-settings.py`. A read-only mount verified both application identities/entries, all 14 x86_64 Mach-O files, matching built-app binary hashes, deep/strict ad-hoc signatures, the host's embedded engine ZIP, page resources, Applications shortcut and original pixel background/layout. The candidate digest is `654bf4b6f0e91d1ad5540df9a0094290bd4d5f22e8574bbb897b1899a926b48f`. Both app Gatekeeper assessments returned exit 3; the unchanged `verify-dmg.py` returned exit 1 at that gate. This is a development review artifact, not a successful public-release acceptance.
- Frontend TypeScript/Vite build and 17 frontend tests passed; UI/style/catalog sources are unchanged.
- Native Rust tests: 73 passed, 2 existing manual/release-only tests ignored. This includes all three updater targets and native identity/archive/rollback/browser-workspace coverage.
- A subsequent native updater test also confirmed that the compiled Intel plugin selects `darwin-x86_64` by default; it passed with the signature test.
- Real isolated Security API identity tests passed on Intel; the launcher compiled as x86_64.
- Both Mojo fileport call sites, both Skia/Blink bitmap call sites, the NSScreen utility and all three pasteboard callers compiled with SDK 15.2 and are included in the linked engine that passed the real probes. SDK availability guards, display lookup, byte order and permission branches remain unchanged. Headed display/clipboard behavior still requires application acceptance.
- The 21 engine boundary tests passed, including synthetic graphics completeness/stability, rejection of stock/partial controls and seed-dependent native graphics, and the existing real two-file patch fixtures. The patch fixtures verify one-time application and digests, plus refusal of partly applied or unrelated trees without changing files. The resumed build accepted the four SDK patches through strict reverse dry-runs and applied the headless shutdown patch. `--force` preserves the requested patch direction; `--batch` could otherwise accept a mixed tree by changing direction per file. These fixtures are separate from the real browser observations.
- Engine preparation and packaging/assembly fixture checks passed. These use synthetic inputs and do not establish complete browser or application runtime acceptance.
- A separate real Ninja interruption fixture confirmed that the disk guard stops both Ninja and its child process group. The Chromium build was unaffected.
- A native `ditto` round trip with a synthetic bundle passed ZIP safety validation and preserved Framework symlinks and Finder metadata. The test does not execute a browser.

The native Foundation test showed that `NSHomeDirectory`, tilde expansion and Application Support/Cache lookups ignore a child-process `HOME` override on this machine. A subsequent real Foundation probe confirmed that adding `CFFIXED_USER_HOME` points those lookups at the temporary home. The headed diagnostic used both variables, explicit browser profiles and mock Keychain access; the four real host data/cache/preference paths checked before and after remained absent. Rust/Tauri's `dirs` path lookup also uses the temporary `HOME`. Native GUI acceptance still needs to verify each data boundary; these observations do not establish isolation of every macOS API or LaunchServices child environment. Account-changing Keychain operations remain covered by the existing disposable native fixture rather than a real account switch.

## Current owner handoff checkpoint

The port was rebased onto owner main/release commit `30a171f5191fbb65dc24e4b1a0392d998f7c5719`. Application code was built from port commit `7dc03f6fe249b8d9f78a4a3bfee4be2fac4954d5`; subsequent handoff documentation does not change the compiled product. All five accepted engine patch digests and engine version 148 remained identical, so the accepted native engine was reused.

- The diff against that owner base is empty for UI, assets, vendored components and native Objective-C operations. Production Rust logic is also unchanged; the Rust delta contains the updater tests and module map only. This retains the owner's newer complete-profile-reset and data-retention behavior on both macOS architectures.
- The optimized x86_64 host rebuilt successfully in 4 minutes 11 seconds with existing Xcode 16.2 / SDK 15.2. Its actual Mach-O minimum remains macOS 12. The global Xcode selection remains CommandLineTools.
- Frontend tests: **22 passed**, **4,125 assertions**. The real Intel browser completed **53** existing UI/profile checks, including the owner's newer browser cleanup confirmation cases; that suite still uses mock IPC.
- Native Rust tests: **96 passed**, **2 ignored** (the existing manual performance measurement and actual published signed-archive gate). The isolated real Security fixture passed scope, shared-field, stale refusal, exact deletion, restore and unrelated-item preservation checks.
- Two real optimized-host start/reconnect/normal-quit cycles passed against a staged, sandboxed browser and the actual bundled detection page. The real Rust context/report endpoints, virtual identity card, CPU/language/timezone/platform/version observations, denied geolocation, report receive times and session cleanup passed again. Browser external routes were blocked and remained visibly unverified; the real application-data paths checked before/after remained absent.
- Engine boundary tests **21**, macOS packaging fixtures **8**, native macOS importer fixtures **4**, development installer fixtures **8** and release assembly fixtures **3** passed, along with signing order/JIT, temporary-versus-official signature, credential exclusion and ticket retry checks. Their synthetic installation/Gatekeeper messages do not replace the actual DMG result below.
- The current development DMG is `CCBaga-0.2.0-macos-x86_64-development.dmg`, SHA-256 `56e9e19d592be329e42360297344258cc4c7cda115f2bbb9bec5d50a28ce845d`. Its read-only payload/layout, identities, all 14 x86_64 Mach-O files, built-binary hashes, embedded engine ZIP/UI and deep/strict ad-hoc signatures passed. Both real Gatekeeper checks returned **exit 3**, and the unchanged formal DMG verifier returned **exit 1**. This current candidate supersedes the earlier `654bf4b6...` development image for owner review.

The local development version `0.2.0` is the repository's unchanged package version; the owner's release workflow supplies the formal release version. The user selected handoff to the owner for formal signing/publication. The original local handoff checkpoint preceded source PR submission and any separately authorized contributor artifact upload. Its package contains source changes, accepted engine and provenance, development candidate and evidence, plus a Chinese PR title/body draft. Source review and contributor validation artifacts remain separate from the owner's formal release acceptance.

## Gates still required for full CCBaga support

1. Obtain owner visual approval and complete fresh installation and full native identity/browser switching on target hardware.
2. Complete Developer ID signing, notarization/Gatekeeper and real signed-update acceptance.
3. Complete both macOS CI architectures and the existing full multi-platform delivery gates before advertising an Intel release.

The README support list retains its existing acceptance level until these gates pass. The new release matrix is configured, but has not produced a real Intel release.

## Owner release handoff

Official publication remains restricted to `liseami` by the repository's existing policy. The current contributor account has read access to the upstream repository. The local machine has no Developer ID Application identity or configured Apple/updater release credentials; private keys are not part of the handoff.

1. Review the Chinese-named port branch and the source changes, provenance and local acceptance records. Retain the same UI, native operations, identifiers and profile contracts as the owner's arm64 source.
2. Put the accepted Intel engine ZIP and its source/license/patch records at an owner-controlled immutable HTTPS location accessible to the release runner. Configure repository variable `CCBAGA_INTEL_ENGINE_URL` with that real download URL and `CCBAGA_INTEL_ENGINE_SHA256` with `15c8377d688bb2005cbc900cc71088e910999fc2833d031285e2765464eb6566`. No public engine URL has been invented or configured locally.
3. Use the existing release secrets: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_API_PRIVATE_KEY`, `APPLE_API_KEY`, `APPLE_API_ISSUER`, `CC_SIGN_IDENTITY`, `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. The workflow restores the certificate/API key only for signing; the source build, checks and engine import remain without those secrets.
4. Complete owner visual and installation/identity acceptance, then follow the repository's existing main/release synchronization procedure using the same accepted commit. The workflow rebuilds the host, signs/notarizes/staples the browser before embedding it, and signs/notarizes/staples the host and DMG on both macOS architectures. Its Intel input must be the accepted engine archive above.
5. Require Accepted notarization results, valid stapled tickets, Gatekeeper approval, the signed archive/version checks and both architecture-specific macOS outputs before assembling the eleven public files. Keep the local development DMG's failed formal-verifier record as a separate review result; the owner workflow creates the formal installer.

The `macos-15` arm64 and `macos-15-intel` x86_64 runner labels are listed in [GitHub's hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners). Those CI jobs are configured but have not been run for this local port checkpoint.
