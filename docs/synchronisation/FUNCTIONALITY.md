# Synchronisation — fonctionnement

[← Toutes les fonctionnalités](../README.md) · [Parcours du code](CODE_WALKTHROUGH.md) · [Mise en place](INSTALL.md)

Chaque appareil écrit **son** fichier `ledger-<appareil>.json` et fusionne ceux des autres. Le dossier est regardé toutes les 4 s.

Rejoindre un dossier déjà utilisé propose **Merge** ou **Use folder data**. *Undo* quitte aussi le dossier.

Un fichier illisible est sauté ; les copies `.sync-conflict-` sont fusionnées puis supprimées.
