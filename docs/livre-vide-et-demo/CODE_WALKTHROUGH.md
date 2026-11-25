# Livre vide et démo — parcours du code

[← Toutes les fonctionnalités](../README.md) · [Fonctionnement](FUNCTIONALITY.md)

- `Book::seed` ; identifiants `s<AAAAMMJJ>-<n>`, reconnus par `is_demo_entry`.
- `Book::remove_demo` : pierres tombales, donc le retrait tient face à un autre appareil qui a la même démo.
- `FolderSync::without_local_demo` : ce qu'on laisse de côté en rejoignant un dossier.
- Une démo dans une autre langue n'est pas un conflit (`merge`).
