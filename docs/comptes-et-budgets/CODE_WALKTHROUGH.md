# Comptes et budgets — parcours du code

[← Toutes les fonctionnalités](../README.md) · [Fonctionnement](FUNCTIONALITY.md)

- `AccountForm` (`ui/app.slint`), `Book::save_account`, `Book::delete_account`.
- Chaque modification est datée (`updated_at`) : à la fusion, la plus récente gagne.
- Supprimer laisse une pierre tombale.
