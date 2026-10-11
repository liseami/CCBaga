<!--
[INPUT]: 仓库实际模块边界、验证命令与 GEB 文档契约。
[OUTPUT]: 英文贡献指南，限定变更说明、测试与公开问题中的脱敏信息。
[POS]: README 的贡献入口；不假设已存在公共仓库、CI 或发布账号。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
-->
# Contributing to CCTool

Thank you for helping make native maintenance predictable and understandable.

## Getting started

Follow the [development guide](README.md#development). The desktop host and native tests require macOS; the companion browser currently targets Apple Silicon. Use the lockfiles and the vendored UI components already in this repository.

## Reporting a bug

Include the macOS and app versions, the operation you attempted, expected behavior, actual behavior and a sanitized error message. For browser checks, distinguish the browser exit from the machine exit. For a build failure, include the failing command and toolchain versions.

Do not include private keys, account tokens, passwords, full subscription URLs or personal file contents. Report security-sensitive details through a maintainer-provided private channel once the public repository is configured.

## Making a change

1. Read the relevant `CLAUDE.md` module map and the file's input/output header.
2. Make one focused change and follow the existing module boundaries.
3. Keep UI strings in both YAML catalogs, with identical keys and interpolation slots.
4. Add a focused regression for behavior changes where a failure would matter.
5. Update the source header, module map and public documentation when their contracts change.
6. Run the checks relevant to the changed behavior.

```sh
bun run build
bun test
MACOSX_DEPLOYMENT_TARGET=12.0 \
  cargo test --manifest-path src-tauri/Cargo.toml
python3 scripts/test-sign-browser.py
bash -n scripts/setup-vps.sh
```

Documentation-only changes should check local links, code examples and factual accuracy. Do not generate application screenshots to represent untested behavior.

## Pull requests

Describe the concrete problem, the resulting behavior, the checks you ran and any limits that remain. Keep native operations in Rust/platform bridges and presentation in the UI. Do not broaden cleanup targets, permissions or remote actions without documenting the scope and testing the boundary.

Keep generated application bundles, browser binaries, credentials and release archives out of source commits. New original contributions follow the project's [MIT License](LICENSE); preserve all applicable third-party notices.
