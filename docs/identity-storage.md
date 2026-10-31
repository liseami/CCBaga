<!--
[INPUT]: 原生保险库的保留策略、事务/进程/浏览器边界与显式密码迁移。
[OUTPUT]: 身份保存、恢复、未索引完整归档保留与同系统迁移边界，以及 v0.2.49 根备份跨机验收失败的版本处理。
[POS]: README 与使用指南的存储契约，不包含真实账号或凭据。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
-->
# Local identity storage

CCBaga manages a local snapshot, not a server-side identity. Sign-in validity is determined by Anthropic. Only manage accounts you own and use them in accordance with the official terms.

## What an identity contains

Claude Code separates authentication from application data. Its [authentication reference](https://code.claude.com/docs/en/authentication) documents macOS Keychain credentials, the Linux/Windows credential file and separate configuration directories. The [directory reference](https://code.claude.com/docs/en/claude-directory#application-data) distinguishes transcripts, tool outputs, file snapshots, attachments, logs and configuration backups.

CCBaga archives the approved sign-in fields and selected Desktop login stores. The user's retention choices determine whether skills, plugins, MCP, settings, project state, history and caches stay in the workspace or follow the snapshot. Actual project source directories are excluded. Custom configuration directories and Console profiles outside the default directory are not automatically switched.

**Keep in workspace is a retention choice, not a deletion checkbox.** When conversation history is unchecked, it moves into the encrypted saved identity; it is not deliberately discarded. To restore that history, choose the corresponding saved card and leave history unchecked so it follows the identity back into the workspace. If history was checked, it stayed shared in the workspace and was excluded from that snapshot. Deleting a saved card or using explicit permanent deletion can remove the available copy; inspect the saved cards and recovery state before any further deletion. CCBaga cannot promise recovery of data that has already been deleted.

A large archive can contain conversation transcripts, images, tool results, file history and Desktop databases. These are not all authentication requirements. They remain recoverable because the user selected them for archiving. Keeping a category in the workspace avoids copying it into subsequent snapshots but also leaves that category visible there. Compression cannot guarantee a fixed size, particularly for images and compressed database content.

## One slot per recognized account

The native layer matches the account UUID and organization UUID. When the account UUID is unavailable, it uses a normalized email and the organization identifier; an organization name is a conservative fallback when its UUID is missing. The matching key is an HMAC-SHA256 digest under the local vault key, not a token or raw account identifier in SQLite. Display names and refreshed access tokens are not matching keys.

Saving a recognized account updates its existing slot and preserves its custom name. A newer backup replaces the slot’s previous archive generation after a verified commit; saved cards are not an unlimited history of backup versions. The current account has one current card, without another recent-use card. A manual sign-in to another account is matched against the actual local account metadata rather than the last restored slot. Unknown identities, including Desktop-only identities without a recognized Code account, are kept separate instead of being guessed or merged. This does not verify that local account metadata is current on the server.

## Safe replacement and recovery

Each update writes a new immutable archive generation. The archive is synchronized and fully authenticated before SQLite commits its pointer together with the recovery marker. Old generations are collected only after a committed replacement; a rejected database update leaves the previous generation intact. The recovery checkpoint is independent and cannot overwrite the snapshot needed for rollback.

Cleanup only considers generations known to the committed operation (including a completed rollback) and checks that the current catalog no longer references them. Opening a vault does not delete completed archives merely because their catalog entries are missing. Unindexed archives from an interrupted or rejected commit, or an inconsistent/restored catalog, remain available for recovery. This does not automatically reconstruct missing catalog entries; keep the vault and its `key.bin` together. A crash or unsuccessful cleanup can leave additional completed generations on disk.

SQLite v1/v2 catalogs migrate transactionally to v3. Legacy snapshots are indexed after archive verification during explicit identity operations; ordinary refresh does not decrypt the archive list or backfill missing summaries. Duplicate recognized slots converge after a completed operation. Damaged and unidentified snapshots are not silently merged.

New archives use [Zstandard](https://github.com/facebook/zstd) level 3 with a checksum inside the existing AES-256-GCM authenticated container. Old gzip archives remain readable. Decoders validate the encrypted terminator, reject appended or truncated data and bound the Zstd window. Scratch extraction files do not need per-file synchronization because the durable authenticated archive remains the recovery source; restored workspace files are synchronized before completion.

**Save backup / Update backup** stops recognized Claude processes after confirmation and saves the selected current state without changing workspace files, sign-in state or the active browser binding. The confirmation distinguishes data included in the encrypted backup from categories kept in the workspace. It does not automatically delete OAuth or configuration locks. Confirmed transient lock files and empty lock directories are excluded from new archives and old archive restores; nonempty same-name directories remain real data under the selected retention policy.

Plan and credential badges describe bounded local metadata only. A known future expiry is shown as locally unexpired; absent or legacy expiry data remains unknown. Expired credentials with a refresh token indicate that a refresh can be attempted, not that automatic renewal or server access is assured. The UI updates when a known local expiry passes. Process previews show at most 32 detected process names/PIDs; query failure is unknown, not an empty list. Old session files are not trusted to identify live workspaces.

Current snapshots skip only Claude’s default `.claude/debug/latest` generated symlink without following or archiving its target. Other selected or nested symlinks remain rejected. Root-level `.claude.json.backup` and `.claude.json.backup.*` paths stay valid backup sources when restoring after they have been moved out of the workspace.

The identity stop command shares the precise process matcher with CLI maintenance without rescanning all historical files. File traversal retains ownership and link checks, checks process state periodically and forces checks at transaction boundaries. It does not remove the recovery or validation steps to improve speed.

## Reproducible local checks

Run native regressions with `cargo test --manifest-path src-tauri/Cargo.toml`. They use temporary accounts and files, including repeated A/B switching, token refresh, organization isolation, rejected updates, interrupted recovery and v1 gzip migration.

The `portable_root_backups_import_when_source_home_is_absent` regression exports root configuration backups, moves the entire source home out of reach and imports into a different user's temporary home. It checks that import preserves the current identity, existing archive, vault key and browser selection, then verifies backup/history recovery after an explicit switch. Restoring only the v0.2.49 path-permission function reproduces the validation rejection; the fixed rule passes the [macOS, Ubuntu and Windows checks](https://github.com/liseami/CCBaga/actions/runs/40651090806).

Run the optional synthetic measurement with:

```sh
cargo test --manifest-path src-tauri/Cargo.toml synthetic_switch_and_compression_benchmark -- --ignored --nocapture
```

A local macOS debug run with 500 synthetic 256 KiB session files measured approximately 0.72 seconds to stash and 3.19 seconds to restore. A separate 13.5 MB JSONL sample compressed to 1.56 MB with Zstd versus 1.69 MB with gzip. These are synthetic measurements, not a latency or size guarantee for real accounts or other platforms. The normal test suite does not enforce machine-dependent timing thresholds.

The vault is protected local storage, not an isolation boundary against other programs running as the same user or an administrator. Encryption does not remove server records or guarantee that installed software cannot discover the vault.

## Linked browser data

**Link browser** is on by default for users without a saved preference; an explicit choice to turn it off is preserved. Initial browser data stays in place and is associated during an explicit linked identity switch or browser-link confirmation. Saving a backup alone never changes the active browser selection. Later identities get separate Chromium user data directories and configurations; existing data is retained. When re-enabling linking, the confirmation allows choosing whether to associate existing data with an unbound identity. Each directory uses Chromium's internal `Default` profile, with separate cookies and site storage. Switching closes the linked browser normally and reopens the selected directory. Save browser work first.

SQLite commits the browser selection with the identity switch and restores the previous selection during rollback. An interrupted identity operation blocks browser startup until recovery. An empty identity receives a fresh browser directory. Reopening a saved identity reuses its directory; browser data is not compressed, copied or merged during switching. A failure to reopen the browser is reported separately after a committed identity change, with a retry action.

The initial nonzero 32-bit fingerprint seed is derived from CCBaga's own random identity ID using SHA-256, with collisions registered and resolved locally. An unbound fresh browser directory reserves its random ID for the first saved identity, avoiding a seed change at that first save. It is not derived from an Anthropic account UUID or token. Renaming or refreshing credentials does not change it. Explicit fingerprint changes remain local to that identity and persist across switches.

Browser data remains outside the encrypted identity archive. Chromium and the OS manage its storage protection; this feature does not make the browser directory a CCBaga-encrypted vault. Unlinking returns to the original shared browser directory and retains identity directories. Deleting a saved identity retains browser data by default; deleting the associated browser data requires a separate selection and refuses a directory that is in use. No claim of secure disk erasure is made.

First use still requires website sign-in. Restoring Code/Desktop credentials does not create website cookies or verify that all clients use the same account. Separate profiles do not change the public IP, prevent server-side correlation, or guarantee that browsers, extensions and websites make no external connections. Custom profiles created manually inside Chromium remain outside the identity-card model. See the [Chromium user data directory reference](https://chromium.googlesource.com/chromium/src/+/HEAD/docs/user_data_dir.md).

The browser check page places a compact identity card above the environment checks. It shows bounded display fields for the local browser binding, not a verified website login. If the browser belongs to A but local Claude metadata changes to B, the card retains A and marks it out of sync. Account tokens, account UUIDs and matching digests are never included in the page context. Disconnected and unreadable states are explicit.

## Move a saved identity to another device

Use **Export** on a saved card or the current card’s saved backup, select a new `.bagapack` filename and enter a password of at least 12 characters (at most 1,024 UTF-8 bytes). The confirmation lists the snapshot's scope. Keep the file and password private: the package contains saved sign-in material and the files included by that snapshot's retention policy. Data left shared in the source workspace is excluded.

Import only your own backups or files from a source you trust. Password encryption and archive validation protect integrity; they do not establish the sender’s identity or sanitize hooks, plugins, MCP servers or other executable configuration. Import itself does not execute them, but restoring and later running Claude can.

On the other device, use **Import**, select the package and enter the same password. Imports require the same operating system. The package is fully authenticated and checked against the native path/member whitelist before a new local archive is committed. The new device re-encrypts it using its own vault key and derives its own account matching digest. Existing identities and browser bindings are never overwritten; duplicate recognized accounts or conflicting fingerprint settings are rejected. Import does not switch the current identity, open a browser or access the live Keychain. A failed import leaves existing saved identities intact.

**Cross-device imports from v0.2.49:** A snapshot containing root-level `.claude.json.backup` or `.claude.json.backup.*` files can fail immediately during validation when the source home directory does not exist on the receiving computer. The old whitelist depended on discovering those files on disk. [v0.2.50](https://github.com/liseami/CCBaga/releases/tag/v0.2.50) validates these backup paths independently of disk contents. Upgrade the receiving application to v0.2.50 or later and import the existing package with its original password; this fix does not require re-exporting or changing the package format. This reproduces the symptom in [Issue #69](https://github.com/liseami/CCBaga/issues/69); a failure that persists on the updated receiver needs separate diagnosis.

Browser transfer is available only for a saved identity with an existing browser binding; saving an unbound draft does not establish that binding. The optional browser transfer carries only the fingerprint seed and environment/display settings. It excludes cookies, website passwords, site sessions, explicit proxy settings and OS-protected browser keys. Website sign-in is still required. Windows Desktop may also require signing in again because OS/DPAPI-bound data is not re-keyed for the new device; a successful file import does not validate the transferred login. Historical file contents and project paths are not rewritten, so references to paths on the old computer may need adjustment. The local vault key, SQLite database and recovery log are never exported.

The versioned package uses a random salt, PBKDF2-HMAC-SHA256 with 600,000 iterations and the existing AES-256-GCM authenticated stream around tar/Zstd. Import rejects links, unsafe/duplicate members, incompatible versions and payloads above 8 GiB or 100,000 data members. Native file selection yields a one-use five-minute ticket; arbitrary paths are not accepted from the UI. Passwords are not saved or logged. There is no password recovery.

On macOS, saved Keychain account names are preserved. Claude Code looks up its default credential using its effective local username (including the `USER` environment setting); a different username on the new device may require signing in again with the official client. CCBaga does not guess account names from home-directory paths or rewrite Keychain ACLs. Custom secure-storage/configuration directories are outside the default scope.
