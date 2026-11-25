# Adding an entry — code walkthrough

[← All features](../README.md) · [How it works](FUNCTIONALITY.md)

- `ui/app.slint`, `AddForm`: the form, bound to the `global Store`.
- `src/main.rs`: `on_open_add`, `on_save_entry`, `update_preview`.
- `Book::sides` turns the intent into (debit, credit), `Book::add_entry` validates.
- `Book::restore_entry` recreates the entry under a **new id**: the old one carries a tombstone, which always wins.
