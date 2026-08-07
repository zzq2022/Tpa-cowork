# TPA CoWork Hard Rebrand Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the Hope Agent identity from source, runtime interfaces, and release artifacts, replacing it with TPA CoWork without legacy compatibility.

**Architecture:** `tpa/product.toml` is the product identity source. Rust, TypeScript, packaging, and audit consumers derive or validate their constants against it. Migration is split into identity infrastructure, visible copy, runtime identity, compatibility deletion, and artifact verification so each boundary can be tested independently.

**Tech Stack:** Rust workspace, Tauri 2, React 19/TypeScript, Node.js scripts, Vitest, Cargo.

## Global Constraints

- Display name is exactly `TPA CoWork`; default agent name is exactly `TPA-Agent`.
- Data root is `~/.tpa-cowork/`; binary and service names are `tpa-cowork`.
- Environment variables use the `TPA_COWORK_` prefix.
- Do not read, migrate, delete, or fall back to `.hope-agent`, `HA_*`, `HOPE_*`, old services, old binaries, or old Native Host IDs.
- Preserve third-party names and required upstream attribution only through an exact audit allowlist.
- Do not commit or push unless the user separately requests it.
- Ask before running `pnpm test`, `pnpm lint`, Cargo tests, or clippy; targeted `cargo check` and `pnpm typecheck` are allowed during development.

---

### Task 1: Product identity source and audit gate

**Files:**
- Create: `tpa/product.toml`
- Create: `src/tpa/product.ts`
- Create: `scripts/check-product-identity.mjs`
- Create: `scripts/__tests__/check-product-identity.test.mjs`
- Modify: `package.json`
- Modify: `src/lib/appMeta.ts`

**Interfaces:**
- Produces TypeScript exports `PRODUCT_DISPLAY_NAME`, `PRODUCT_SLUG`, `PRODUCT_COMPACT_NAME`, `PRODUCT_AGENT_NAME`, `PRODUCT_DATA_DIR`, `PRODUCT_ENV_PREFIX`, `PRODUCT_BINARY_NAME`, and `PRODUCT_USER_AGENT`.
- Produces `pnpm product:check` for source or artifact scanning.

- [ ] Write a failing Node test that creates a temporary source tree containing `Hope Agent`, verifies the checker reports its file and line, and verifies an approved attribution entry is accepted.
- [ ] Run `node --test scripts/__tests__/check-product-identity.test.mjs`; expect failure because the checker does not exist.
- [ ] Add `tpa/product.toml` with the confirmed identity values and implement `src/tpa/product.ts` from those exact values with a generated-source warning.
- [ ] Implement `scripts/check-product-identity.mjs` with source and `--artifact <dir>` modes, exact allowlist entries, file/line diagnostics, and non-zero exit on unapproved legacy patterns.
- [ ] Add `product:check` to `package.json` and rename `HOPE_AGENT_URLS` to `TPA_COWORK_URLS` in `src/lib/appMeta.ts` and its consumers.
- [ ] Re-run the Node test and `pnpm typecheck`; expect both to pass.

### Task 2: Frontend and generated-document visible identity

**Files:**
- Modify: all non-test matches under `src/`
- Modify: `skills/office-docx/scripts/*.py`
- Modify: `skills/office-xlsx/scripts/build_xlsx.py`
- Modify: `skills/office-pptx/scripts/build_pptx.py`
- Test: affected frontend tests and `scripts/office-skill-smoke-test.py`

**Interfaces:**
- Consumes product constants from `src/tpa/product.ts` where code needs a product name.
- Produces GUI, notification, terminal, update, and Office metadata using only TPA CoWork identity.

- [ ] Add or update focused tests for Help, About/provider headings, default notifications, terminal prefixes, and Office core properties so old copy fails the tests.
- [ ] Run only those focused tests; expect failures showing old identity.
- [ ] Replace visible hardcoded names and fallback copy with product constants or TPA CoWork i18n copy; rename internal `HOPE_AGENT_*` TypeScript identifiers to `TPA_COWORK_*`.
- [ ] Replace Office author, revision author, creator, lastModifiedBy, and Application metadata with `TPA CoWork`, and initials `TPA` where applicable.
- [ ] Re-run focused tests, Office smoke checks, and `pnpm typecheck`; expect success.

### Task 3: Tauri, CLI, Server, OAuth, and tool-visible identity

**Files:**
- Modify: `src-tauri/src/menu_labels.rs`
- Modify: `src-tauri/Info.plist`
- Modify: `src-tauri/src/cli_onboarding/**`
- Modify: `src-tauri/src/commands/update_bridge.rs`
- Modify: `crates/ha-server/src/banner.rs`
- Modify: `crates/ha-server/src/web_assets.rs`
- Modify: `crates/ha-server/build.rs`
- Modify: `crates/ha-server/src/bin/hope-agent.rs` and rename the bin target to `tpa-cowork`
- Modify: user-visible strings in `crates/ha-core/src/oauth.rs`, `mcp/oauth.rs`, `tools/**`, `slash_commands/**`, `permissions.rs`, memory backup diagnostics, and recap prompts.

**Interfaces:**
- Produces all desktop-native, CLI, server, OAuth, and model-visible product copy as TPA CoWork.

- [ ] Update existing menu/server/OAuth/status tests to expect TPA CoWork and run the smallest matching test targets to see them fail.
- [ ] Replace visible strings precisely, including all supported locale branches in native menus and macOS permission descriptions.
- [ ] Rename CLI binary declarations and command examples from `hope-agent` to `tpa-cowork` without changing unrelated crate names.
- [ ] Update assertions and snapshots, then run `cargo check -p ha-core`, `cargo check -p ha-server`, and `cargo check -p tpa-cowork`; expect success.

### Task 4: Runtime environment and data-root hard cut

**Files:**
- Modify: `crates/ha-core/src/paths.rs`
- Modify: all runtime and test matches for `HA_*` and `HOPE_*` under `crates/`, `src-tauri/`, `scripts/`, `.github/`, Docker and packaging files
- Modify: `src-tauri/src/setup.rs`, `main.rs`, `commands/crash.rs`
- Modify: `crates/ha-core/src/guardian.rs`, `hooks/env.rs`, `ffmpeg.rs`, browser host discovery, eval runtime, server configuration, and test helpers
- Modify: docs describing environment variables

**Interfaces:**
- `paths::root_dir()` recognizes only `TPA_COWORK_DATA_DIR` and otherwise returns the platform home joined with `.tpa-cowork`.
- All private process coordination and public configuration variables use `TPA_COWORK_*`.

- [ ] Change path tests first: new override wins, old override is ignored, default suffix is `.tpa-cowork`, and old directory presence does not trigger migration.
- [ ] Run the focused path tests and observe failures against the current fallback behavior.
- [ ] Remove the `HA_DATA_DIR` fallback and old venv fallback; delete old migration/sentinel logic rather than renaming it.
- [ ] Mechanically rename remaining owned environment variables to `TPA_COWORK_*`, including tests, scripts, CI, Docker, guardian child state, eval isolation, FFmpeg, hooks, and browser host discovery.
- [ ] Update env help text and examples, then run targeted path tests plus `cargo check -p ha-core`, `cargo check -p ha-server`, and `cargo check --workspace` because environment contracts cross crates.

### Task 5: Service, binary, Native Host, packaging, and update identity

**Files:**
- Modify: workspace and crate `Cargo.toml` files
- Modify: `src-tauri/tauri.conf.json`, `tauri.windows.conf.json`, installer hooks, and packaging scripts
- Modify: `extensions/chrome/manifest.json`, locale/store text, native host manifest, service worker
- Modify: browser host, service control, source detector, updater, Docker, Homebrew/Scoop/Linux repository and release workflows

**Interfaces:**
- Produces only `tpa-cowork` executable/service names and the new `com.tpacowork.chrome` Native Host identity.

- [ ] Update packaging and Native Host tests/verification scripts first so old names fail.
- [ ] Rename binary targets, sidecar/resource paths, service install/control, updater source detection, process matching, and command examples.
- [ ] Remove old service/binary/Host probing branches instead of retaining aliases.
- [ ] Update extension manifest, native host manifest, allowed origins, registration scripts, store text, Docker entrypoint, package repositories, and release asset names.
- [ ] Run extension verification, release-path checks, updater-key verification, relevant Cargo checks, and a local packaging dry run that does not publish artifacts.

### Task 6: Compatibility deletion, documentation, and final audit

**Files:**
- Delete: old brand migration modules, sentinels, aliases, and compatibility-only tests discovered by Task 4/5
- Modify: `docs/user-guide/**`, `docs/architecture/**`, release process docs, README files, skill docs, and release notes for the breaking change
- Modify: `.husky/pre-push` and matching CI workflow to invoke product identity checks

**Interfaces:**
- Produces zero unapproved legacy identity matches in source and release artifacts.

- [ ] Delete compatibility-only code and update architecture docs to describe the hard cut and export-before-upgrade requirement.
- [ ] Add the product identity check to local and CI gates in matching positions.
- [ ] Run `pnpm product:check`, docs parity, i18n check, `pnpm typecheck`, workspace Cargo check, focused tests, extension verification, and artifact scanning.
- [ ] With user approval, run the repository's broader `pnpm test`, relevant Cargo tests, lint, and clippy gates; fix every regression.
- [ ] Inspect `git diff --check`, `git status`, and the final legacy-match report; document any attribution-only allowlist entries with reasons.
