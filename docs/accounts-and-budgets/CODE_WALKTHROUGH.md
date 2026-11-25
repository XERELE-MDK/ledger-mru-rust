# Accounts and budgets — code walkthrough

[← All features](../README.md) · [How it works](FUNCTIONALITY.md)

- `AccountForm` (`ui/app.slint`), `Book::save_account`, `Book::delete_account`.
- Every change is stamped (`updated_at`): on merge, the most recent wins.
- Deleting leaves a tombstone.
