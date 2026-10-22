<!--
[INPUT]: intel-engine.lock.json, the pinned macOS packaging/fingerprint patches and an isolated build workspace.
[OUTPUT]: Reproducible Intel engine preparation/build commands, verified inputs, recovery rules and scoped engine/installer validation limits.
[POS]: External Chromium build prerequisite; no installation, Apple signing, publication or real identity operations.
[PROTOCOL]: Update this contract and docs/CLAUDE.md when changing this file.
-->
# Building the Intel fingerprint engine

Use `scripts/build-intel-engine.py`. It replaces the earlier machine-specific driver with a portable workspace and a checked input manifest. This is **process reproducibility**, not a claim that different macOS/Xcode/SDK machines produce byte-identical binaries. CCBaga still uses the latest host lifecycle and identity-workspace rules; no startup patches from the old personal build are restored.

Run the offline recipe regression separately; the macOS PR check also runs it without downloading the engine:

```sh
python scripts/test-intel-engine.py
```

## Fixed inputs

`scripts/intel-engine.lock.json` is the source of truth for URLs, archive roots, digests and expected patched-source fingerprints:

| Input | Fixed revision/version |
|---|---|
| Chromium lite source | 148.0.7778.215; SHA-256 from its official source hash list |
| Fingerprint patches and utilities | adryfish/fingerprint-chromium `13b89eae304123f0710f2d33fd0816a2f61d7ffc` |
| macOS packaging patches | ungoogled-software/ungoogled-chromium-macos `6a00dfe08591a15cfb0aed0edd292ba789d9a0d7` |
| bindgen source and Cargo.lock | rust-lang/rust-bindgen `d874de8d646d9b8a3e7ba2db2bcd52f2fba8f1f5` (0.72.1) |
| LLVM | Native host 22.1.0; upstream macOS archive SHA-512 |
| Rust compiler/stdlib | Nightly snapshot 2026-02-27; official archive SHA-256 for both hosts |
| Node.js | Native host 24.12.0; upstream macOS manifest SHA-512 |
| Python tooling | `scripts/intel-engine-requirements.txt`, Ninja 1.13.2 |

Sources are archive downloads of exact commits, not moving branches. On an ARM Mac the compiler/tools are **ARM**, while Chromium's target and extra Rust standard library are **Intel**. On an Intel build host the native Intel compiler/std are used. This distinction is explicit in the recipe; Rosetta does not make an ARM-only browser executable run on Intel.

The sequence follows the [pinned macOS build script](https://github.com/ungoogled-software/ungoogled-chromium-macos/blob/6a00dfe08591a15cfb0aed0edd292ba789d9a0d7/build.sh): prune source binaries, apply fingerprint and macOS patches, perform domain substitutions, install native tools and the target standard library, bootstrap GN/bindgen, generate Ninja files and build Chromium. The portable script handles archive extraction/toolchain layout directly, so it does not require GNU `greadlink` or an absolute symlink to another local checkout.

## Prerequisites

- Native arm64 or x86_64 macOS with Xcode and an installed/selected macOS SDK; `xcrun --sdk macosx --show-sdk-version` must work. The previous successful engine build used Xcode 26.6 / SDK 26.5. Other SDKs must be validated, not assumed equivalent.
- Python **3.12 or 3.13** (the inspected packaging guide requires 3.13 or earlier; archive extraction uses Python's data filter).
- `curl`, system `patch`, and Ninja 1.13.2 in PATH. Use a dedicated Python environment; do not change the system Python or global packages.
- About **200 GiB initial free disk space is recommended** and low parallelism on a 16 GiB machine. Preparation and compilation each enforce a 60 GiB remaining-space threshold. Actual requirements depend on the toolchain/build output; monitor disk use.
- `bun install --frozen-lockfile` in CCBaga for the pinned Playwright test dependency. The recipe supplies a native Node.js runtime for verification after preparation.

```sh
python3.13 -m venv /absolute/path/to/engine-tools
/absolute/path/to/engine-tools/bin/python -m pip install \
  -r scripts/intel-engine-requirements.txt
export PATH="/absolute/path/to/engine-tools/bin:$PATH"
bun install --frozen-lockfile
```

The script records the actual macOS, Python, SDK and Ninja versions. It does not download or activate Xcode, install a browser, change the default browser, read Claude identity data or use Apple/CCBaga signing keys.

## Inspect, prepare, build

Choose a new workspace outside the Git checkout, with no whitespace in its path (the pinned Rust helper constructs compiler environment strings). Use the same workspace and cache for subsequent invocations.

```sh
python scripts/build-intel-engine.py plan

python scripts/build-intel-engine.py prepare \
  --workspace /absolute/path/to/intel-engine-work \
  --cache /absolute/path/to/download-cache \
  --report /absolute/path/to/reports/prepare.json

python scripts/build-intel-engine.py build \
  --workspace /absolute/path/to/intel-engine-work \
  --cache /absolute/path/to/download-cache \
  --jobs 2 \
  --report /absolute/path/to/reports/engine.json
```

`plan` lists selected pinned archives without downloading. `prepare` verifies cached/downloaded hashes; safely extracts pristine source; applies the exact patch sequence; checks fingerprint source hashes; installs relocatable host/target toolchains; downloads the fixed bindgen archive and enforces its dependency lock; bootstraps GN/bindgen; and produces `out/Default/build.ninja`. A successful preparation report explicitly says `full_engine_compiled: false`.

`build` resumes the same prepared workspace, runs `ninja ... chrome` and verifies every actual Mach-O contains x86_64. It then uses a loopback page and temporary browser profiles to test CPU, language, timezone, Canvas seed variation and the Intel sample's full engine version. The host A is not started. The completed app is:

```text
<workspace>/macos/build/src/out/Default/Chromium.app
```

The fixed GN overrides are `target_cpu="x64"`, `symbol_level=0`, `use_thin_lto=false`, `use_v8_context_snapshot=false`. They retain the regular V8 startup and fingerprint patches; the last option disables optional Blink context preinitialization to avoid a redundant host Blink build. They are part of the recorded recipe, not a hidden local edit.

## Recovery and verification

- Downloads go to a temporary file and are published only after digest verification. Existing corrupt cache entries are preserved and rejected; inspect/move those entries before retrying.
- A nonempty directory without the recipe ownership marker is refused. Changed GN flags are also rejected instead of overwriting local customization. A workspace from another manifest or build-host architecture is not silently adopted.
- An interrupted **download** can be retried. An interrupted **Chromium compilation** can be resumed with the same `build` command; Ninja keeps completed work.
- Interrupted source extraction/patching/toolchain/bootstrap preparation is not marked complete. Preserve that workspace and start another new workspace with the same verified cache. Do not blindly reapply patches or delete someone else's source tree.
- Fresh bindgen bootstrap uses `--locked` with a pinned Cargo.lock. This closes the earlier gap where a temporary hand-extracted bindgen checkout was used.

Read-only verification of a previously completed engine is supported:

```sh
python scripts/build-intel-engine.py verify \
  --source-tree /absolute/path/to/completed/macOS/build/src \
  --report /absolute/path/to/reports/existing-engine.json
```

Verification checks the patched fingerprint files, exact GN flags, bindgen lock and builder version, all code architectures, actual package version, engine file hashes and real fingerprint behavior. An older build's known unlocked bindgen helper can be recognized, but its report says `bindgen_cargo_locked: false`; it is not relabeled as a fresh locked bootstrap. This mode does not rewrite the old tree or rebuild it.

## Package A+B

Use the resulting app as the explicit input to the complete pair builder:

```sh
python scripts/bundle-intel-macos.py \
  --browser-app /absolute/path/to/intel-engine-work/macos/build/src/out/Default/Chromium.app \
  --output /absolute/path/to/new-installer-output \
  --build-label local-intel
```

It rebuilds the current upstream launcher for x86_64, signs all B code, embeds the exact B archive in A and produces the matched Intel DMG. Do not use stock Chromium, mix app pairs or point the standalone smoke test at `CCBrowser`, which may activate the real A. Official Developer ID signing, notarization and publication remain the owner's separate workflow.

ARM build/native tests and build-host Intel engine tests do not replace physical Intel GUI acceptance. The current installer subsequently passed independent physical Intel acceptance; its exact hash, tested scenarios and remaining limits are recorded in the [Intel guide](intel-macos.md#validation-recorded-on-2026-10-22). This round qualifies ARM-host preparation and cross-target tool invocation; the native Intel-host tool selection is covered by the manifest/tests, but was not executed on Intel hardware.
