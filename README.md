# restarilo

Minimal restic GUI wrapper (Tauri 2 + React).

Esperanto: *restarilo* — restoration tool (`restarigi` + `-ilo`).

Select a snapshot to browse its file tree (folders and files from restic tree objects via `cat blob`, keyed by tree id and reused across snapshots that share subtrees). From the file browser you can restore or dump a selection, or open a file.

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
[restic]
# optional; after ~ expansion this path must be absolute
# binary = "/usr/local/bin/restic"

cache_dir = "cache/tree"

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

### Restore / dump / open (manual)

With the same `FIX=/tmp/restarilo-z1` repos (local + SFTP) configured in `config.toml`:

1. Browse a snapshot, select a **file**, then Restore / Dump / Open (Open uses `restarilo-open-*` under the OS temp dir; restarilo does not clean these up).
2. Select a **directory**, then Restore and Dump (zip). Open stays disabled.
3. Folder dialog cancel → no change and no error.
4. **In-progress Cancel** (app Cancel button or Escape, only after Working…): start Restore / Dump / Open on `large.bin`, cancel while busy — UI leaves Working… without an error toast. Restore may leave partial files under the target. Open may leave a partial `restarilo-open-*` temp.
5. **Dump cancel with existing destination:** write known bytes to the dump target path, start Dump of `large.bin`, cancel — destination bytes unchanged; no leftover `.restarilo-dump-*` in the target folder. Overwrite confirm starts a **new** action id (Cancel enabled again for the second attempt).
6. Repeat Cancel checks on the SFTP repo.

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
