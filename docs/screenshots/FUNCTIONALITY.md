# Screenshot scenario — how it works

[← All features](../README.md) · [Code walkthrough](CODE_WALKTHROUGH.md)

```bash
ledger-desktop --screenshots <folder>
```

Draws the whole UI **without opening a window**, on throwaway data, by playing a scenario: valid and invalid entries, empty book, opening balances, dark mode, merge, sync. Images are written as PPM.

It is also a test: any failed check stops the program.
