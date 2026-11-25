# Screens — code walkthrough

[← All features](../README.md) · [How it works](FUNCTIONALITY.md)

Data always flows the same way:

```
click → Store callback → Rust changes State → refresh() → Store properties → redraw
```

- `refresh()` recomputes **every** screen: simple, and impossible to get out of sync.
- `Store`'s business properties are `in`: the UI cannot write them.
- Rust sends already formatted text; Slint only displays it.
- The `on!` macro wires each callback by cloning the state and a **weak** reference to the window.
- Never call a callback from another one: the state is already borrowed.
