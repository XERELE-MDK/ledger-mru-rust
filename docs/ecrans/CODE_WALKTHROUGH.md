# Écrans — parcours du code

[← Toutes les fonctionnalités](../README.md) · [Fonctionnement](FUNCTIONALITY.md)

Le flux va toujours dans le même sens :

```
clic → callback de Store → Rust modifie State → refresh() → propriétés de Store → redessin
```

- `refresh()` recalcule **tous** les écrans : simple, et impossible à désynchroniser.
- Les propriétés métier de `Store` sont `in` : l'interface ne peut pas les écrire.
- Rust envoie des textes déjà formatés ; Slint ne fait qu'afficher.
- La macro `on!` branche chaque callback en clonant l'état et une référence **faible** à la fenêtre.
- Ne pas appeler un callback depuis un autre : l'état est déjà emprunté.
