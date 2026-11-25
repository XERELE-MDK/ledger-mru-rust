# Import et export — fonctionnement

[← Toutes les fonctionnalités](../README.md) · [Parcours du code](CODE_WALKTHROUGH.md)

*Accounts → Data* : le chemin se tape.

- **Merge** garde les deux côtés, suppressions comprises.
- **Replace** prend le fichier tel quel ; refusé pendant la synchro.

Undo est proposé. Limite connue : annuler une fusion ne tient pas si la synchro est active.
