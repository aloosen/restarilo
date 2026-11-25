# restarilo

Minimal restic GUI wrapper (Tauri 2 + React).

Esperanto: *restarilo* — restoration tool (`restarigi` + `-ilo`).

Browse snapshots in an Explorer-style layout with a brand header (archive mark + restarilo wordmark) and repository/snapshot selectors in the same bar. Surfaces use a warm light palette (cream main, sand sidebar, petrol accent) and a matching dark hierarchy with clear selection and focus. A folder tree stays left of the listing at medium widths (including ~800px); Hide / Show folders collapses it (and the splitter) for narrow windows and restores the prior width on expand. The resize splitter is a 1px line with a wider drag hit area. A status bar under both panes (sidebar background) shows entry count and selection. When a repository loads, the newest snapshot is selected automatically (by normalized snapshot time). Snapshot options show local date/time without seconds plus hostname (to tell backup sources apart); short/full snapshot id stays in the option/select `title`. The header center lists the snapshot’s backed-up top directories (basename of each path; full paths on hover). Overwrite dialog still uses the full readable label. In the tree, a drive icon marks All files and a folder icon sits beside each name; the twist control expands or collapses without navigating; clicking the icon or name navigates. Expand failures are retryable on that branch. Tree, breadcrumb, and listing stay on one current path. Above the listing, a clickable breadcrumb sits over a prominent current-folder heading with Restore / Export / Open (with icons) beside it. The listing uses Tabler outline icons by type (dir/symlink first, then case-insensitive extension including compounds like `.tar.gz`), folders before files (name order within each group), KiB/MiB sizes when available, and a short Modified column (full timestamp via `title`). A single click anywhere on a row selects it; double-click or Enter opens a folder. Keyboard: arrows move the row focus, Enter opens a folder, Space selects a file or folder. Switching snapshots in the **same** repository keeps the current absolute folder when it still exists as a directory (including empty); otherwise the nearest existing ancestor is opened and a short requested→displayed note appears. Changing repositories always starts at `/`. After a snapshot or repository change, Restore / Export / Open stay disabled until you select a row again in the active listing. Restore is the primary action; Export replaces the old Dump label (same `dump_path` backend). Overwrite confirmation names the source path, snapshot label, and exact destination. Success shows the returned destination and clears when a new action starts, after Cancel returns to idle, when the repo or snapshot changes, or when the listing selection changes. While an action runs, the busy bar names the operation (`Restoring…` / `Exporting…` / `Opening…`); Cancel stays available. Success, Cancel, and errors stay distinguishable. Empty and error states: config load failure asks you to fix `config.toml` and restart (the process caches config for its lifetime); zero repositories is a distinct message; a failed snapshot list offers Retry for the same repository and is not the same as an empty repository (successful list with no snapshots); empty folders and retryable directory load errors stay in the listing pane.

After the snapshot list loads, the newest snapshot’s root tree is prefetched; after you open a directory, up to four child folders are queued (one background fetch). An optional on-disk cache (see `cache_dir`) keeps listings across restarts. The in-memory session cache is capped at about 32 MiB (LRU).

## Configuration

Search order for `config.toml`:

1. Next to the `restarilo` executable
2. `$XDG_CONFIG_HOME/restarilo/config.toml` when `XDG_CONFIG_HOME` is set
3. `$HOME/.config/restarilo/config.toml` (Windows: `%APPDATA%\restarilo\config.toml`)

Passwords are never stored as plaintext. Each `[[repos]]` entry must set **exactly one** of:

- `password_file`
- `password_command`
- `insecure_no_password = true` (requires restic ≥ 0.17.0)

### Optional tree listing cache

```toml
# Relative to the restarilo executable (portable). Stores filenames and directory
# structure under v1/<restic-repo-id>/<tree-id>.json. Soft cap 256 MiB of those
# entries only; a single oversized entry is not written. Omit or leave empty to disable.
cache_dir = "cache/tree"
```

**Portable restart check:** point `[restic].binary` at a wrapper that logs each
`cat blob <tree_id>` to a file, browse with `cache_dir` set, quit, copy the
portable folder, reopen the same directory — the log should gain no new line for
that tree id; a changed tree in a later snapshot should append its id.

### Example

```toml
# top-level (not under [restic])
cache_dir = "cache/tree"

[restic]
# optional; after ~ expansion this path must be absolute
# binary = "/usr/local/bin/restic"

[[repos]]
name = "local"
path = "/var/backup/restic-repo"
password_file = "repo.pass"
```

### Restic binary

Resolution order:

1. `[restic].binary` if set (must be absolute after `~` expansion)
2. `restic` / `restic.exe` next to the `restarilo` executable
3. `PATH`

### SFTP and SSH keys

Use restic’s `sftp:` backend. Host, user, and port come from the repo URL.

Optional per-repo `ssh_key` (path relative to the executable directory, or absolute). Paths may contain spaces; on Windows, backslashes are fine. restarilo passes the key via restic `-o sftp.args=…` (not `RESTIC_SSH_COMMAND`, which restic does not read). A `.pub` file may sit next to the private key; only the private key is used for auth.

Before each restic run, inherited `RESTIC_PASSWORD`, `RESTIC_PASSWORD_FILE`, `RESTIC_PASSWORD_COMMAND`, and `RESTIC_REPOSITORY_FILE` are cleared so only the repo’s configured location and password mode apply.

```toml
[[repos]]
name = "nas"
path = "sftp:user@nas.local:/restic/repo"
password_file = "nas.pass"
ssh_key = "id_ed25519"

[[repos]]
name = "lab"
path = "/tmp/test-repo"
insecure_no_password = true
```

See also the [restic SFTP documentation](https://restic.readthedocs.io/en/stable/030_preparing_a_new_repo.html#sftp).

### MVP verification fixtures

Manual success check (local + SFTP, snapshot without tags):

```bash
FIX=/tmp/restarilo-z1
mkdir -p "$FIX/data" "$FIX/my keys"
echo hello > "$FIX/data/file.txt"
# ~32 MiB so Restore/Dump stay busy long enough to click Cancel
dd if=/dev/urandom of="$FIX/data/large.bin" bs=1M count=32 status=none
ssh-keygen -t ed25519 -f "$FIX/my keys/id_ed25519" -N ""
# add "$FIX/my keys/id_ed25519.pub" to ~/.ssh/authorized_keys for localhost SFTP
restic -r "$FIX/local-repo" init --insecure-no-password
restic -r "$FIX/local-repo" backup "$FIX/data" --insecure-no-password
restic -r "$FIX/sftp-repo" init --insecure-no-password
restic -r "$FIX/sftp-repo" backup "$FIX/data" --insecure-no-password
LOCAL_ID=$(restic -r "$FIX/local-repo" snapshots --insecure-no-password --json | python -c 'import sys,json; print(json.load(sys.stdin)[0]["short_id"])')
SFTP_ID=$(restic -r "$FIX/sftp-repo" snapshots --insecure-no-password --json | python -c 'import sys,json; print(json.load(sys.stdin)[0]["short_id"])')
printf '{"local":"%s","sftp":"%s"}\n' "$LOCAL_ID" "$SFTP_ID" > "$FIX/expected.json"
cd src-tauri && cargo test --lib live_local_and_sftp_snapshots -- --nocapture
```

Missing fixtures fail the live test (no silent skip).

### Explorer path-carry fixtures (#6)

Restic stores the **path argument** as the snapshot path. Backup `Documents` from `$FIX/docs` (not `$FIX/docs/Documents`) so entries appear as `/Documents/Taxes`, not a host-absolute path under `/tmp/…`.

```bash
FIX=/tmp/restarilo-z1
# Assumes local + SFTP repos already initialized and the $FIX/data backup exists
# (fallback: open /Documents/Taxes then switch to the data-only snapshot → /).

mkdir -p "$FIX/docs/Documents/Taxes"
echo v1 > "$FIX/docs/Documents/Taxes/note.txt"
# Snapshot with /Documents/Taxes/note.txt = v1 (both backends)
( cd "$FIX/docs" && restic -r "$FIX/local-repo" backup Documents --insecure-no-password )
( cd "$FIX/docs" && restic -r "$FIX/sftp-repo" backup Documents --insecure-no-password )

echo v2 > "$FIX/docs/Documents/Taxes/note.txt"
# Same path, different contents (restore-after-switch)
( cd "$FIX/docs" && restic -r "$FIX/local-repo" backup Documents --insecure-no-password )
( cd "$FIX/docs" && restic -r "$FIX/sftp-repo" backup Documents --insecure-no-password )

# Empty /Documents/Taxes
rm -f "$FIX/docs/Documents/Taxes/note.txt"
( cd "$FIX/docs" && restic -r "$FIX/local-repo" backup Documents --insecure-no-password )
( cd "$FIX/docs" && restic -r "$FIX/sftp-repo" backup Documents --insecure-no-password )

# /Documents without Taxes (ancestor fallback to /Documents)
rm -rf "$FIX/docs/Documents/Taxes"
( cd "$FIX/docs" && restic -r "$FIX/local-repo" backup Documents --insecure-no-password )
( cd "$FIX/docs" && restic -r "$FIX/sftp-repo" backup Documents --insecure-no-password )
```

Do **not** run `restic backup "$FIX/docs/Documents"` — that embeds the absolute host path in the snapshot.

### Restore / export / open (manual)

With the same `FIX=/tmp/restarilo-z1` repos (local + SFTP) configured in `config.toml`:

1. Browse a snapshot, select a **file**, then Restore / Export / Open (Open uses `restarilo-open-*` under the OS temp dir; restarilo does not clean these up).
2. Select a **directory**, then Restore and Export (zip). Open stays disabled.
3. Folder dialog cancel → no change and no error.
4. **In-progress Cancel** (app Cancel button or Escape, only after Restoring… / Exporting… / Opening…): start Restore / Export / Open on `large.bin`, cancel while busy — UI leaves the busy bar without an error toast; prior success lines clear. Restore may leave partial files under the target. Open may leave a partial `restarilo-open-*` temp.
5. **Export cancel with existing destination:** write known bytes to the export target path, start Export of `large.bin`, cancel — destination bytes unchanged; no leftover `.restarilo-dump-*` in the target folder. Overwrite confirm names source, snapshot, and destination and starts a **new** action id (Cancel enabled again for the second attempt).
6. Repeat Cancel checks on the SFTP repo.

## Icons (Tabler)

Listing, tree, and action icons are [Tabler Icons](https://github.com/tabler/tabler-icons) (outline), MIT License, Copyright (c) 2020-2026 Paweł Kuna. SVG sources and `LICENSE` / `NOTICE` live under `src/icons/tabler/`; the same license texts are copied into `public/licenses/tabler-icons/` for distributions (`dist/licenses/…` after `npm run build`).

## Dev

```bash
npm install
npm run tauri dev
```

## Build (Linux)

```bash
npm run tauri build
```

Binary: `src-tauri/target/release/restarilo`

## Cross-compile Windows (from Arch)

```bash
sudo pacman -S llvm lld
rustup target add x86_64-pc-windows-msvc
cargo install --locked cargo-xwin
npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc
```

Binary: `src-tauri/target/x86_64-pc-windows-msvc/release/restarilo.exe`
