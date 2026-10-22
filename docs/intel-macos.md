<!--
[INPUT]: Current upstream lifecycle/identity-workspace contracts and an external pinned Intel fingerprint engine.
[OUTPUT]: Experimental complete Intel A+B source build, scoped Intel/ARM validation evidence and rollback guidance.
[POS]: Community build guide; does not enable official Intel releases or change data/identity rules.
[PROTOCOL]: Update this contract and docs/CLAUDE.md when changing this file.
-->
# Experimental Intel macOS source build

There is no official Intel macOS installer or Intel update feed. This community adaptation builds a matched **CCBaga-A + CCBaga-B** pair for x86_64. It preserves the current upstream launcher, per-identity browser workspaces, recovery blocking, data boundaries and default arm64 release behavior.

The integration baseline is `bdfb90ceadfc5fae6772ffff597bddecb0b73cc3` (main inspected on 2026-10-22). The older personal startup-r2 package was based on `2ab3e6f29b42cba6caa59743b67b6347e813d668`. Its broader startup patch must not overwrite current upstream workspace selection or identity recovery. Current upstream already waits for current host arguments, refuses fallback to another profile and detects an actual profile process instead of treating the launcher as a running browser. This is source evidence, not physical Intel acceptance.

## Engine input

A normal Chromium/Chrome build does not implement the fingerprint switches. Use Fingerprint Chromium **148.0.7778.215**, the version currently pinned by this project, built with the matching ungoogled macOS packaging patches for `target_cpu="x64"`.

The previously compiled Intel engine used these source revisions:

- `adryfish/fingerprint-chromium`: `13b89eae304123f0710f2d33fd0816a2f61d7ffc`.
- `ungoogled-software/ungoogled-chromium-macos`: `6a00dfe08591a15cfb0aed0edd292ba789d9a0d7`.
- GN configuration: the corresponding fingerprint `flags.gn` plus macOS `flags.macos.gn`, then `target_cpu="x64"`, `symbol_level=0`, `use_thin_lto=false`, `use_v8_context_snapshot=false`.

The last flag skips optional Blink context preinitialization. Regular V8 startup and fingerprint patches remain enabled. Engine compilation is an external prerequisite; the packaging helper consumes and validates the complete app and records local provenance. It does not download an unverified Intel binary or automatically rebuild Chromium. The portable, pinned source-build script, stage recovery rules and verification commands are documented in the [engine build guide](intel-engine-build.md). A build recipe does not turn this community package into an official release.

For a new engine, inspect and run `scripts/build-intel-engine.py plan`, then follow the engine guide's `prepare` and `build` commands. Both native ARM hosts cross-building Intel and native Intel build hosts are covered by the locked tool selection.

## Build both applications

Use macOS with Xcode Command Line Tools, Bun, Node.js, Python 3.11+ and stable Rust with the `x86_64-apple-darwin` target. An Apple Silicon build host needs Rosetta to execute Intel test binaries. Follow the existing development prerequisites and lockfiles.

```sh
bun install --frozen-lockfile
rustup target add x86_64-apple-darwin
python3 scripts/bundle-intel-macos.py \
  --browser-app /absolute/path/to/compiled/Chromium.app \
  --output /absolute/path/to/new-output-directory \
  --build-label upstream-bdfb90c
```

The helper validates every engine Mach-O, the actual version, and CPU/language/timezone/Canvas behavior in disposable profiles, then sets `CC_MACOS_ARCH=x86_64`, builds A with the matching Rust target, signs the complete B bundle, embeds that exact browser archive in A, and creates a DMG with both applications. It mounts the DMG read-only, checks both identifiers, architectures and strict signatures, and writes SHA-256 checksums. It does not install or launch the real A, publish releases, access signing credentials or use Apple notarization.

`CC_MACOS_ARCH` defaults to **arm64** in the existing branding script. Official Apple Silicon builds keep the original behavior unless an Intel target is explicitly selected; an incompatible engine fails the architecture check before changing the launcher.

The runtime architecture query prevents the Intel UI from contacting the official ARM updater. Replace both apps with a complete community Intel installer. Existing Apple Silicon, Windows and Linux update behavior is retained.

## Verification and limits

```sh
bun test
bun run build
MACOSX_DEPLOYMENT_TARGET=12.0 \
  cargo test --release --manifest-path src-tauri/Cargo.toml \
  --target x86_64-apple-darwin --features custom-protocol
node scripts/test-intel-browser.cjs \
  /absolute/path/to/CCBaga-B.app/Contents/MacOS/Chromium
```

Use the raw **Chromium** entry for disposable profiles. The current upstream `CCBrowser` launcher may activate A before checking explicit profile arguments; running it as an isolated smoke test could start the real host. Do not reuse the older r2 launcher-fixture claim for this build, whose launcher is intentionally upstream code.

Build-host tests and source review do not establish physical Intel GUI acceptance. On the target Intel Mac, verify hash, every actual code file, strict signatures and equality of A's embedded B with the installed B. Then test Finder cold B startup, A-running/B-stopped startup, A-button launch, repeated opens without uncontrolled instances, normal quit/reopen, fresh measurement reports, and Intel update protection. Compare actual parameters and reports with the **current selected identity workspace/configuration**, not the old globally shared profile or a hardcoded seed. An expected existing exit-IP/timezone conflict is separate from startup correctness.

The added upstream identity-binding, full-profile-reset and encrypted-transfer flows are not covered by the older personal build's hardware report. Use existing disposable regression fixtures; do not switch real identities, reset browser data, access real Keychain records or alter system/network settings as part of build validation. Test Apple Silicon behavior as well before declaring the contribution ready for maintained multi-architecture support.

## Validation recorded on 2026-10-22

A contributor-built complete A+B installer based on the baseline above was independently tested on an **Intel Core i5-8257U, macOS 15.7.7 (24G720)**. Its SHA-256 was `caff05b09f793f64f967a83426633ce074401b30617dc776e2c530a995dcb2f9`. This identifies the tested development artifact; it is not an official download or a guarantee for later builds.

- Installed A, B and A's embedded B passed all-code x86_64, strict signature and embedded-browser equality checks. Disposable-profile CPU/language/timezone/Canvas tests passed.
- All seven GUI scenarios listed above passed, including three Finder cold starts, repeated opens retaining one main browser process, quit/reopen with a new PID, and Intel complete-package update protection.
- Remeasurement used a user-assisted physical click; screenshots and host/browser reports showed the same newer callback time. The current empty-identity legacy profile matched A's arguments and configuration. Real identity switching, vault transfer/recovery, independent network leak checks and graphics/font equivalence were not exercised.
- A separate default-arm64 native A+B build passed architecture, signature, embedded-browser and resource checks; 117 Rust tests passed (2 ignored), native fingerprint smoke tests passed and 65 browser UI simulation checks passed using fictitious identities/mocked IPC. This did not install or launch the real host or touch real identity data.

The raw reports, screenshots and private backups are not included in the source tree. Ad-hoc signature verification is not Developer ID signing, notarization or Gatekeeper acceptance. The clean engine recipe preparation and real Ninja bindgen target were tested, but a second complete cold engine build and execution of the recipe on an Intel build host remain unverified; see the engine guide.

## Install and rollback

This is an **ad-hoc signed community development build**, not an official or notarized release. Verify the supplied checksum and use the system's per-app approval if macOS asks; do not disable Gatekeeper globally.

Before the first newer A launch, normally quit both apps and make a private, matched backup of **both old app bundles and this application's local browser/identity-vault state**. The newer upstream introduces additional database metadata, identity-linked profiles and archive formats. App-only replacement is not a complete data-format rollback. Keep that private backup local; it may contain sensitive user data and must not be attached to a public issue or PR.

Replace complete app bundles without merging them. Keep existing user data during upgrade. If rollback is needed, stop both apps, preserve the new state separately and restore the matching old app/state backup. Do not mix old/new bundles, merge identity vaults or erase newly created data. No existing users' data is migrated or tested by the packaging helper.
