# restarilo

Minimal restic GUI wrapper (Tauri 2 + React).

Esperanto: *restarilo* — restoration tool (`restarigi` + `-ilo`).

## Configuration

Search order for `config.toml`:

1. Next to the `restarilo` executable
2. `$XDG_CONFIG_HOME/restarilo/config.toml` when `XDG_CONFIG_HOME` is set
3. `$HOME/.config/restarilo/config.toml` (Windows: `%APPDATA%\restarilo\config.toml`)

Passwords are never stored as plaintext. Each `[[repos]]` entry must set **exactly one** of:

- `password_file`
- `password_command`
- `insecure_no_password = true` (requires restic ≥ 0.17.0)

### Example

```toml
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

Before each restic run, inherited `RESTIC_PASSWORD`, `RESTIC_PASSWORD_FILE`, and `RESTIC_PASSWORD_COMMAND` are cleared so only the repo’s configured password mode applies.

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
