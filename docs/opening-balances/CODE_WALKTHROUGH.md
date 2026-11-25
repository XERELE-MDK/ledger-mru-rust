# Opening balances — code walkthrough

[← All features](../README.md) · [How it works](FUNCTIONALITY.md)

- `Book::set_opening`, `opening_rows`, `opening_date`, `is_opening_entry`.
- The id starts with `o`: that is what lets the app replace it without touching the rest.
- The difference is **rounded to the cent**: `0.1 + 0.2 - 0.3` is not exactly 0 in binary.
- `OpeningForm` (`ui/app.slint`): one field per model row; each keystroke calls `edit-opening(index, text)`, kept in `State.opening`.
- The date must survive a round trip unchanged: `2026-1-015` is refused.

Same rules as `AppState.setOpening` (Flutter).
