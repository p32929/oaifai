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

[MIT](LICENSE) © p32929

## Contributing

Contributions are warmly welcomed and greatly appreciated! Whether it's a bug fix, new feature, or improvement, your input helps make this project better for everyone.

Before submitting a pull request, please:

1. Create an issue describing the feature or bug fix you'd like to work on
2. Wait for discussion and approval to ensure alignment with project goals
3. Fork the repository and create your feature branch
4. Submit your pull request with a clear description of changes

This approach helps avoid duplicate efforts and ensures smooth collaboration. Thank you for considering contributing!

## Share

Sharing this repository with your friends is just one click away from here

[![facebook](https://user-images.githubusercontent.com/6418354/179013321-ac1d1452-0689-493f-9066-940cf2302b6e.png)](https://www.facebook.com/sharer/sharer.php?u=https://github.com/p32929/oaifai/)
[![twitter](https://user-images.githubusercontent.com/6418354/179013351-7d8d6d1c-4ce2-46ab-bef8-4c4765a1b888.png)](https://twitter.com/intent/tweet?url=https://github.com/p32929/oaifai/)
[![tumblr](https://user-images.githubusercontent.com/6418354/179013343-3111f55a-3b90-40c7-8487-9777348672b0.png)](https://www.tumblr.com/share?v=3&u=https://github.com/p32929/oaifai/)
[![pocket](https://user-images.githubusercontent.com/6418354/179013334-b095c45f-becf-49f4-9ee1-5a731a9b1f85.png)](https://getpocket.com/save?url=https://github.com/p32929/oaifai/)
[![pinterest](https://user-images.githubusercontent.com/6418354/179013331-44cd9206-11b1-4b65-becb-5863b61c828f.png)](https://pinterest.com/pin/create/button/?url=https://github.com/p32929/oaifai/)
[![reddit](https://user-images.githubusercontent.com/6418354/179013338-7416ae3f-73ba-4522-86e1-1374d7082d22.png)](https://www.reddit.com/submit?url=https://github.com/p32929/oaifai/)
[![linkedin](https://user-images.githubusercontent.com/6418354/179013327-ca7b7102-1da8-4b1c-858f-1a6e5f21bd70.png)](https://www.linkedin.com/shareArticle?mini=true&url=https://github.com/p32929/oaifai/)
[![whatsapp](https://user-images.githubusercontent.com/6418354/179013353-f477fa0b-3e6f-4138-a357-c9991b23ff88.png)](https://api.whatsapp.com/send?text=https://github.com/p32929/oaifai/)
