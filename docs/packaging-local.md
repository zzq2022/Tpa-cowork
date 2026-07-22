# Local packaging ladder

Use the lightest mode that still answers your question. Full NSIS + fat-LTO
eval sidecar is for end-of-migration smoke, not every code tweak.

## Modes

| Command | What it builds | Typical cost | When |
|---|---|---|---|
| `pnpm pack:local` | Main app via `release-fast` (no LTO, no NSIS) | Medium | Daily migration / feature verification |
| `pnpm pack:local:quick` | Same as compile, reuses frontend / browser-host / eval sidecar | Low | Backend-only iteration after one full prepare |
| `pnpm pack:local:bundle` | NSIS installer; release with LTO forced off | High | Local install smoke before handoff |
| `pnpm pack:local:ship` | NSIS installer with default release profile | Highest | Ship-like binary size / performance check |

## Flags

All flags work on compile and bundle/ship:

- `--skip-frontend` — reuse `dist/` (needs `dist/index.html`)
- `--skip-host` — reuse `src-tauri/resources/browser-host`
- `--skip-eval-sidecar` — reuse `src-tauri/binaries/hope-agent-eval-*` (**skips fat LTO**, often 10–30+ min)
- `--skip-venv` — reuse `agent-venv.zip` (bundle/ship only; compile never zips)
- `--jobs N` — cap `CARGO_BUILD_JOBS`

`pack-local` owns prepare steps and passes an empty Tauri
`beforeBuildCommand`, so skip flags are not undone by `tauri.conf.json`.

### Low-memory hosts (≤16 GB)

On low free RAM the script auto-caps jobs (override with `--jobs`):

```bash
pnpm pack:local:bundle -- --skip-eval-sidecar --jobs 2
```

Prefer one cargo/rustc pipeline at a time. Avoid `cargo clean` unless artifacts
are corrupt — incremental `target/` is the main day-to-day speedup.

## Recommended migration loop

1. Code change + targeted check: `cargo check -p ha-core` / `pnpm typecheck`
2. Local binary: `pnpm pack:local:quick` (or full `pnpm pack:local` after UI/host/sidecar changes)
3. Installer once at the end: `pnpm pack:local:bundle -- --skip-eval-sidecar` if sidecar already prepared
4. Optional ship profile: `pnpm pack:local:ship`

## Related

- Agent venv zip / NSIS extract contract: [`packaging-agent-venv.md`](./packaging-agent-venv.md)
- Eval sidecar build: `pnpm prepare:eval-sidecar` (`eval-sidecar` profile: fat LTO, `codegen-units=1`)
