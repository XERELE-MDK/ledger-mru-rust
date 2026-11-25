# Synchronisation — parcours du code

[← Toutes les fonctionnalités](../README.md) · [Fonctionnement](FUNCTIONALITY.md) · [Mise en place](INSTALL.md)

`ledger-core/src/folder.rs`

| Méthode | Rôle |
|---|---|
| `peers` | lit les fichiers voisins |
| `peers_stamp` | nom, date et taille de **chaque** voisin |
| `preview` | ce que la fusion changerait |
| `sync` | fusionne et écrit |
| `write_own` | `.nom.tmp` puis renommage, **seulement si le contenu diffère** |

- Sans la dernière règle, deux appareils au repos se réveillent en boucle (16 réécritures en 30 s avant correction, 0 après).
- L'empreinte est prise **avant** la lecture : un fichier qui change entre-temps est vu au passage suivant.
- `Book::merge` : commutative et idempotente, vérifiée sur les mêmes fichiers que la version Flutter (`CONTRACT.sha256`).
