# Paquets Linux — fonctionnement

[← Toutes les fonctionnalités](../README.md) · [Parcours du code](CODE_WALKTHROUGH.md) · [Mise en place](INSTALL.md)

| Fichier | Pour | Installer | Retirer |
|---|---|---|---|
| `.pkg.tar.zst` | Arch, Manjaro | `sudo pacman -U <fichier>` | `sudo pacman -R ledger-mru` |
| `.deb` | Debian 12+, Ubuntu 22.04+ | `sudo apt install ./<fichier>` | `sudo apt remove ledger-mru` |
| `.rpm` | Fedora 35+, openSUSE | `sudo dnf install ./<fichier>` | `sudo dnf remove ledger-mru` |
| `.tar.gz` | toutes, sans root | `tar xf <fichier>` puis `install.sh` | `install.sh --remove` |

Données : `~/.local/share/ledger-mru/`, jamais supprimées par la désinstallation.
