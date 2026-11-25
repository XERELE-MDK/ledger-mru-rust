# Import and export — code walkthrough

[← All features](../README.md) · [How it works](FUNCTIONALITY.md)

- `on_export_json`, `on_import_json` (`src/main.rs`).
- `Book::from_json` checks before applying; `Undo::Book` keeps the previous book.
- Unknown keys (phone-only settings) are ignored, and not written back.
