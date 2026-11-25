# Protection des données — fonctionnement

[← Toutes les fonctionnalités](../README.md) · [Parcours du code](CODE_WALKTHROUGH.md)

- Avant d'écrire, l'app **relit ce qu'elle va enregistrer**, et refuse si ça ne passe pas.
- L'écriture est atomique.
- Un livre illisible est renommé `ledger.damaged-<date>.json`, **intact**. Un bandeau l'explique.
- Un fichier impossible à ouvrir est laissé en place ; les changements vont dans `ledger.recovered-<ms>.json`.
- Dans les deux cas la synchro est débranchée : la démo ne peut pas écraser la bonne copie.
