# Linux packages — how it works

[← All features](../README.md) · [Code walkthrough](CODE_WALKTHROUGH.md) · [Setup](INSTALL.md)

| File | For | Install | Remove |
|---|---|---|---|
| `.pkg.tar.zst` | Arch, Manjaro | `sudo pacman -U <file>` | `sudo pacman -R ledger-mru` |
| `.deb` | Debian 12+, Ubuntu 22.04+ | `sudo apt install ./<file>` | `sudo apt remove ledger-mru` |
| `.rpm` | Fedora 35+, openSUSE | `sudo dnf install ./<file>` | `sudo dnf remove ledger-mru` |
| `.tar.gz` | any, no root | `tar xf <file>` then `install.sh` | `install.sh --remove` |

Data: `~/.local/share/ledger-mru/`, never deleted by uninstalling.
