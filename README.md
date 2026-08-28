# oaifai

> **oaifai** — how **ওয়াইফাই** (WiFi) sounds in Bangla.
> A tiny terminal UI to scan and connect to WiFi on Ubuntu Server / netplan systems (no NetworkManager).

## Screenshots

### Network list
<img width="972" height="647" alt="Network list" src="https://github.com/user-attachments/assets/93220cf7-d088-4a83-9f73-d5d637b7a7f2" />

### Enter password
<img width="972" height="647" alt="Enter password" src="https://github.com/user-attachments/assets/8c04d7e8-44ec-46e7-8fb1-c519387a1dba" />

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/p32929/oaifai/master/install.sh | sh
```

Then just run:

```sh
oaifai
```

It auto-asks for your `sudo` password (needed to scan and change network settings).

> Linux x86_64 with netplan only. Other systems: `git clone` and `cargo build --release`.

## Usage

In the list: `↑/↓` move, `⏎` connect to the selected network, `d` disconnect from the
current one, `r` rescan, `q` quit. Pressing `⏎` on the network you are already connected
to also offers to disconnect. Disconnecting asks for confirmation first — if you reached
the machine over SSH via that network, you will lose the session.

Non-interactive:

```sh
sudo oaifai --list                        # show current network + scan results
sudo oaifai --connect "SSID" "PASSWORD"   # connect (omit the password for open networks)
sudo oaifai --disconnect                  # drop the current network
```

Add `--dry-run` to print what would be written without applying it.

## License

[MIT](LICENSE) © p32929
