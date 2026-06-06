# oaifai

> **oaifai** — say it out loud: it's how **ওয়াইফাই** (WiFi) sounds in Bangla.

A small, fast, nice-looking terminal UI to **scan and connect to WiFi networks** on Linux boxes that use **netplan → systemd-networkd → wpa_supplicant** (the default on Ubuntu Server, where there's no NetworkManager / `nmcli`).

Built with [ratatui](https://ratatui.rs). Single static-ish binary, ~640 KB, zero background daemons.

---

## Screenshots

### Network list
All networks in range — strongest first, deduplicated — with live signal bars and quality color. The network you're **already connected to** is highlighted and shows its IP beside the name (and isn't selectable). There's also a row at the top to enter a hidden/manual network by name.

![Network list](PASTE_LIST_IMAGE_URL_HERE)

### Asking for password
After choosing a network, enter the password (`Tab` toggles visibility). Leave it blank to connect to an open network.

![Password prompt](PASTE_PASSWORD_IMAGE_URL_HERE)

---

## Features

- **Live scan** with signal strength bars and quality coloring (red → green).
- **Two ways in:** pick from the list, or type a network name manually (hidden SSIDs).
- **Shows current connection** — the active network and its IP are displayed inline and locked from re-selection.
- **Open & secured networks** — password prompt is skipped automatically for open APs.
- **Safe by default** — backs up `/etc/netplan/*.yaml` before touching it and rolls back on failure.
- **Non-interactive CLI** for scripts and automation (`--list`, `--connect`, `--dry-run`).
- **Tiny** — ~640 KB release binary, no runtime dependencies beyond `iw`, `ip`, and `netplan`.

---

## Requirements

- Linux with **netplan + systemd-networkd** (e.g. Ubuntu Server). *Not* for NetworkManager systems — see [Platform support](#platform-support).
- `iw`, `ip`, and `netplan` available on `PATH`.
- **Root** — scanning and rewriting netplan both need it. Run with `sudo`.
- A wireless interface (default assumed: `wlp1s0` — see [Configuration](#configuration)).

---

## Build

Needs a Rust toolchain ([rustup](https://rustup.rs)).

```bash
git clone <your-repo-url> oaifai
cd oaifai
cargo build --release
# binary at: target/release/oaifai
```

Optionally drop it on your PATH:

```bash
sudo install -m755 target/release/oaifai /usr/local/bin/oaifai
```

---

## Usage

### Interactive TUI

```bash
sudo oaifai
```

| Key | Action |
| --- | --- |
| `↑` / `↓` | Move selection |
| `Enter` | Select network / confirm |
| `Tab` | Toggle password visibility |
| `r` | Rescan networks |
| `Ctrl+U` | Clear the current input field |
| `Esc` | Back / cancel |
| `q` | Quit (from the list/loading screen) |

### Command line

```bash
sudo oaifai --list                          # scan and print networks, change nothing
sudo oaifai --connect "SSID" "PASSWORD"     # connect non-interactively
sudo oaifai --connect "OpenNet"             # connect to an open network (no password)
sudo oaifai --dry-run                        # TUI that never writes netplan (safe demo)
```

---

## How it works

oaifai never shells out to NetworkManager. Connecting is:

1. **Back up** the existing `/etc/netplan/00-installer-config.yaml`.
2. **Generate** a fresh netplan YAML for your interface with the chosen SSID/password.
3. **Apply** it with `netplan apply`.
4. **Verify** the link comes up (polls `iw dev <iface> link` and `ip addr`), up to ~20 s.
5. On failure, **restore** the backup and report the error.

Scanning uses `iw dev <iface> scan`; the current connection and IP come from `iw dev <iface> link` and `ip -4 addr show <iface>`.

---

## Configuration

The interface and netplan paths are constants at the top of `src/main.rs`:

```rust
const IFACE: &str        = "wlp1s0";
const NETPLAN_FILE: &str = "/etc/netplan/00-installer-config.yaml";
```

Change `IFACE` to match your adapter (`ip link` to find it), then rebuild.

---

## Platform support

oaifai is **Linux-only** and specifically targets the **netplan/networkd** stack. It will compile elsewhere but won't do anything useful, because it depends on `iw`, `netplan`, and the netplan config file layout. Desktop distros that use **NetworkManager** are not supported (use `nmcli`/`nmtui` there). macOS and Windows are out of scope.

---

## Development

```bash
cargo test            # unit tests (parsing, netplan generation, signal math)
cargo run -- --list   # quick live check
```

Headless rendering of any screen, for snapshot inspection:

```bash
cargo run -- --snapshot list      # also: password | done
```

---

## License

MIT. Do whatever; no warranty.
