# Sync — setup

[← All features](../README.md) · [How it works](FUNCTIONALITY.md) · [Code walkthrough](CODE_WALKTHROUGH.md)

1. Install [Syncthing](https://syncthing.net) and share a folder, for example `~/Sync/Ledger`.
2. *Accounts → Sync*: check the path, **Connect**.
3. On the other devices, choose the same folder.

```bash
sudo pacman -S syncthing                     # Arch
systemctl --user enable --now syncthing
```

Files: identity and folder are kept in `~/.local/share/ledger-mru/ledger.sync.conf`.
