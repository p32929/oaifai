# oaifai

> **oaifai** is how **ওয়াইফাই** (WiFi) sounds in Bangla.

A tiny terminal app to scan for WiFi and connect, on Ubuntu Server and other netplan systems that have no NetworkManager. Normally that means editing netplan YAML files by hand and hoping you got the indentation right. With oaifai you pick a network from a list, type the password, and it writes the config for you.

It is one small program with a single library behind it (ratatui, for the screen), so there is very little to install or trust.

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

Press **`d` anywhere to disconnect** from the current network — it happens straight away,
no confirmation. Note that if you reached the machine over SSH via that network, this drops
your session.

In the list: `↑/↓` move, `⏎` connect to the selected network (or disconnect, if it is the
one you are on), `r` rescan, `q` quit.

Non-interactive:

```sh
sudo oaifai --list                        # show current network + scan results
sudo oaifai --connect "SSID" "PASSWORD"   # connect (omit the password for open networks)
sudo oaifai --disconnect                  # drop the current network
oaifai --version                          # check which build you have
```

Add `--dry-run` to print what would be written without applying it.

## License

MIT License — Copyright (c) 2026 p32929. See [LICENSE](LICENSE) for the full text.

## Contributing

Contributions are very welcome, thank you! Before you start on a pull request, please open an issue and tell me what you would like to add or change. That way we can agree on it first, and your time is not wasted on something that might not fit. Thanks for helping make oaifai better.

## You might also like

<a href="https://openstore.pfay.workers.dev/cards/p32929?n=4&amp;repo=oaifai"><picture><source media="(prefers-color-scheme: dark)" srcset="https://openstore.pfay.workers.dev/cards/p32929.svg?n=4&amp;repo=oaifai&amp;theme=dark"><img src="https://openstore.pfay.workers.dev/cards/p32929.svg?n=4&amp;repo=oaifai&amp;theme=light" width="100%" alt="More apps by p32929 on OpenStore"></picture></a>

<p><a href="https://p32929.github.io/share/?repo=oaifai"><picture><source media="(prefers-color-scheme: dark)" srcset="https://p32929.github.io/readme/footer-share-dark.svg"><img src="https://p32929.github.io/readme/footer-share-light.svg" width="50%" align="top" alt="Share this project with your friends in one click"></picture></a><a href="https://p32929.github.io/support/"><picture><source media="(prefers-color-scheme: dark)" srcset="https://p32929.github.io/readme/footer-support-dark.svg"><img src="https://p32929.github.io/readme/footer-support-light.svg" width="50%" align="top" alt="If you like my work and want to support me, feel free to support"></picture></a><br><a href="https://openstore.pfay.workers.dev/add"><picture><source media="(prefers-color-scheme: dark)" srcset="https://p32929.github.io/readme/footer-join-dark.svg"><img src="https://p32929.github.io/readme/footer-join-light.svg" width="100%" align="top" alt="Put your own apps on OpenStore: free, your listing lives in your own repo"></picture></a></p>
