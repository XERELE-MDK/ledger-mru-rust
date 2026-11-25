# Empty book and demo — code walkthrough

[← All features](../README.md) · [How it works](FUNCTIONALITY.md)

- `Book::seed`; ids `s<YYYYMMDD>-<n>`, recognised by `is_demo_entry`.
- `Book::remove_demo`: tombstones, so the removal holds against another device that has the same demo.
- `FolderSync::without_local_demo`: what is left out when joining a folder.
- A demo in another language is not a conflict (`merge`).
