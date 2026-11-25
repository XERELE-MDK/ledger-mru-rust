# Data safety — how it works

[← All features](../README.md) · [Code walkthrough](CODE_WALKTHROUGH.md)

- Before writing, the app **reads back what it is about to save**, and refuses if it does not pass.
- Writing is atomic.
- An unreadable book is renamed `ledger.damaged-<date>.json`, **untouched**. A banner explains it.
- A file that cannot be opened is left in place; changes go to `ledger.recovered-<ms>.json`.
- In both cases sync is disconnected: the demo cannot overwrite the good copy.
