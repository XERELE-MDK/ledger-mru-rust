# Accounting core — how it works

[← All features](../README.md) · [Code walkthrough](CODE_WALKTHROUGH.md)

Everything rests on **a list of balanced entries** (Σ debits = Σ credits). Balances, totals and reports are recomputed from it, never stored.

| Account type | Normal side | Meaning |
|---|---|---|
| asset | debit | what I have |
| expense | debit | what goes out |
| liability | credit | what I owe |
| equity | credit | starting point |
| income | credit | what comes in |

Type amounts any way you like: `1 250,50`, `1,250.50`, `1.250,50`. `1,250` means **1250**. `inf`, `1e400`, negatives and anything above 10¹² are refused.
