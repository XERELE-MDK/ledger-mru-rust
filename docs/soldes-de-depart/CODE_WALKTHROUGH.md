# Soldes de départ — parcours du code

[← Toutes les fonctionnalités](../README.md) · [Fonctionnement](FUNCTIONALITY.md)

- `Book::set_opening`, `opening_rows`, `opening_date`, `is_opening_entry`.
- L'identifiant commence par `o` : c'est ce qui permet de remplacer sans toucher au reste.
- La différence est **arrondie au centième** : `0.1 + 0.2 - 0.3` n'est pas exactement 0 en binaire.
- `OpeningForm` (`ui/app.slint`) : un champ par ligne du modèle ; chaque frappe appelle `edit-opening(index, texte)`, gardé dans `State.opening`.
- La date doit être identique après aller-retour : `2026-1-015` est refusé.

Mêmes règles que `AppState.setOpening` (Flutter).
