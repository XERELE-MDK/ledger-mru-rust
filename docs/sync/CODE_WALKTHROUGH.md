# Sync — code walkthrough

[← All features](../README.md) · [How it works](FUNCTIONALITY.md) · [Setup](INSTALL.md)

`ledger-core/src/folder.rs`

| Method | Role |
|---|---|
| `peers` | reads the neighbours' files |
| `peers_stamp` | name, date and size of **every** neighbour |
| `preview` | what the merge would change |
| `sync` | merges and writes |
| `write_own` | `.name.tmp` then rename, **only when the content differs** |

- Without that last rule, two idle devices wake each other forever (16 rewrites in 30 s before the fix, 0 after).
- The fingerprint is taken **before** reading: a file that changes in between is seen on the next pass.
- `Book::merge`: commutative and idempotent, checked on the same files as the Flutter version (`CONTRACT.sha256`).
