# Saisir une écriture — parcours du code

[← Toutes les fonctionnalités](../README.md) · [Fonctionnement](FUNCTIONALITY.md)

- `ui/app.slint`, `AddForm` : le formulaire, relié au `global Store`.
- `src/main.rs` : `on_open_add`, `on_save_entry`, `update_preview`.
- `Book::sides` convertit l'intention en (débit, crédit), `Book::add_entry` valide.
- `Book::restore_entry` recrée l'écriture sous un **nouvel identifiant** : l'ancien porte une pierre tombale, qui gagne toujours.
