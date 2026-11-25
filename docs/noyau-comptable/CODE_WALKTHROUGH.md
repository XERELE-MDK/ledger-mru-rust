# Noyau comptable — parcours du code

[← Toutes les fonctionnalités](../README.md) · [Fonctionnement](FUNCTIONALITY.md)

`ledger-core/` n'a aucune dépendance d'interface : `serde`, `serde_json`, `chrono`.

| Fichier | Rôle |
|---|---|
| `src/lib.rs` | `Book`, requêtes, mutations, fusion |
| `src/format.rs` | dates, montants, `parse_amount` |
| `src/folder.rs` | dossier partagé |

- `AccountType` est une `enum` : un type inconnu est impossible à représenter, `serde` refuse le fichier.
- `Book::from_json` contrôle l'équilibre et les comptes cités.
- Requêtes en `&self` qui rendent des références, mutations en `&mut self`.
- Les cas de `parse_amount` sont dans `tests/fixtures/amounts.json`, **partagés avec la version Flutter**.

```bash
cargo test -p ledger-core
```
