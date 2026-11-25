# Accounting core — code walkthrough

[← All features](../README.md) · [How it works](FUNCTIONALITY.md)

`ledger-core/` has no UI dependency: `serde`, `serde_json`, `chrono`.

| File | Role |
|---|---|
| `src/lib.rs` | `Book`, queries, mutations, merge |
| `src/format.rs` | dates, amounts, `parse_amount` |
| `src/folder.rs` | shared folder |

- `AccountType` is an `enum`: an unknown type cannot be represented, so `serde` rejects the file.
- `Book::from_json` checks the balance and the accounts referenced.
- Queries take `&self` and return references, mutations take `&mut self`.
- The `parse_amount` cases live in `tests/fixtures/amounts.json`, **shared with the Flutter version**.

```bash
cargo test -p ledger-core
```
