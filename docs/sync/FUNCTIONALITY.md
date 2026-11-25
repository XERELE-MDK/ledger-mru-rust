# Sync — how it works

[← All features](../README.md) · [Code walkthrough](CODE_WALKTHROUGH.md) · [Setup](INSTALL.md)

Each device writes **its own** `ledger-<device>.json` file and merges the others'. The folder is checked every 4 s.

Joining a folder already in use offers **Merge** or **Use folder data**. *Undo* also leaves the folder.

An unreadable file is skipped; `.sync-conflict-` copies are merged, then deleted.
