# Synchronisation — mise en place

[← Toutes les fonctionnalités](../README.md) · [Fonctionnement](FUNCTIONALITY.md) · [Parcours du code](CODE_WALKTHROUGH.md)

1. Installer [Syncthing](https://syncthing.net) et partager un dossier, par exemple `~/Sync/Ledger`.
2. *Accounts → Sync* : vérifier le chemin, **Connect**.
3. Sur les autres appareils, choisir le même dossier.

```bash
sudo pacman -S syncthing                     # Arch
systemctl --user enable --now syncthing
```

Fichiers : identité et dossier dans `~/.local/share/ledger-mru/ledger.sync.conf`.
