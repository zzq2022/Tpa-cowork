# Windows agent-venv packaging

The Windows NSIS package includes `agent-venv.zip` and installs it as an
application-owned Python environment. Users must not need a system Python or a
manual extraction step.

## Build contract

`agent-venv.zip` is the packaging source of truth. Its root directory must be
`agent-venv/`, containing `Scripts/python.exe`.

```powershell
py -3.12 -m venv agent-venv
.\agent-venv\Scripts\python.exe -m pip install -U pip
.\agent-venv\Scripts\python.exe -m pip install -r agent-venv-requirements.txt
pnpm prepare:zip-venv -- --force
```

An existing prebuilt `agent-venv.zip` is accepted by `pnpm prepare:zip-venv`
without requiring the expanded directory.

## Install contract

`src-tauri/tauri.windows.conf.json` places the archive at
`$INSTDIR\resources\agent-venv.zip`. The NSIS hook extracts into a temporary
directory, verifies `Scripts\python.exe`, atomically replaces
`$INSTDIR\agent-venv`, writes `.tpa-cowork-venv-complete`, and only then deletes
the archive.

If installer extraction fails, the archive remains. The Rust runtime retries
the same verified staging flow on first launch. A partial directory without the
completion marker is never treated as usable.

## Local packaging modes

See [`packaging-local.md`](./packaging-local.md) for compile / quick / bundle / ship usage and skip flags.
