# Screenshot scenario — code walkthrough

[← All features](../README.md) · [How it works](FUNCTIONALITY.md)

The `shots` module of `src/main.rs`.

- `MinimalSoftwareWindow` replaces the platform; the software renderer draws into a pixel buffer.
- The real callbacks are invoked (`invoke_…`): it is an end-to-end test.
- CI plays it on every push and keeps the images for 14 days.
