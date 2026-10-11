<!--
[INPUT]: 原创项目 MIT 授权、现有 Chromium 许可与本地 UI 来源记录。
[OUTPUT]: 英文第三方归属和分发边界；未发现的许可不推断为 MIT。
[POS]: LICENSE 的第三方补充说明，供 README 和发布包引用。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
-->
# Third-party notices

The top-level MIT license covers CCTool's original project work. It does not replace third-party copyright notices or license terms.

## Fingerprint Chromium and Chromium

The companion browser uses [Fingerprint Chromium](https://github.com/adryfish/fingerprint-chromium), based on Ungoogled Chromium. The existing BSD-3-Clause notice is retained in [browser-runtime/LICENSE.fingerprint-chromium](browser-runtime/LICENSE.fingerprint-chromium) and included in the application resources.

The browser bundle contains Chromium and additional dependencies with their own licenses. Preserve the engine's complete attribution/credits resources when preparing a binary distribution. Do not treat the single Fingerprint Chromium notice as a complete license inventory for the browser.

## Vendored interface

`vendor/cromma-ui` is a local copy of Cromma UI, including its Pika icon subset. The copied tree does not currently contain an independent upstream license notice. Confirm its redistribution terms and restore any required notices before publishing these components. The top-level MIT declaration is not evidence of permission from other copyright holders.

## Application dependencies

Tauri, React, Radix UI, Motion, Tailwind CSS, Vite, YAML and the Rust dependencies retain their respective licenses. `bun.lock` and `src-tauri/Cargo.lock` identify the versions used. Source distributions should retain upstream notices; binary releases should include the applicable notices for what they ship.

## Workflow references and branding

Claude cleanup discovery was informed by the [claude-cleanup skill](https://github.com/suyuan2022/suyuan-skill/tree/main/claude-cleanup). Its upstream license and attribution must be reviewed for any redistributed material.

The cleaner and browser artwork was supplied for use with this project. Anthropic, Claude, Apple, Chromium and other third-party names remain the property of their respective owners. Their mention does not imply sponsorship or endorsement.
