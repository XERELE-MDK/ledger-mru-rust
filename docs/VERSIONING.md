# Gestion des versions

[← Documentation](README.md) · [Journal des versions](../CHANGELOG.md)

## Numéros

[SemVer](https://semver.org/lang/fr/) : `MAJEUR.MINEUR.CORRECTIF`.

| Changement | On augmente |
|---|---|
| le format JSON n'est plus lisible par les anciennes versions | MAJEUR |
| nouvelle fonctionnalité | MINEUR |
| correction | CORRECTIF |

Les trois dépôts avancent avec **le même numéro**. Une version peut n'apporter que « aucun changement » dans l'un d'eux.

## Branches et commits

- `main` est toujours publiable : la CI doit y être verte.
- Une branche par sujet : `feat/…`, `fix/…`, `docs/…`, fusionnée sans avance rapide.
- Messages : [Conventional Commits](https://www.conventionalcommits.org/fr/) (`feat(opening): …`, `fix(sync): …`).

## Publier une version

1. Changer le numéro dans `Cargo.toml` et la dépendance `ledger-core` de `ledger-desktop/Cargo.toml`.
2. Écrire la section `## [X.Y.Z] — date` de `CHANGELOG.md`.
3. Vérifier : `tools/release_notes.sh X.Y.Z` (échoue si les numéros ne concordent pas).
4. Commit, puis étiquette : `git tag -a vX.Y.Z -m "Ledger MRU X.Y.Z"` et `git push --follow-tags`.
5. La CI compile et publie `.deb`, `.rpm`, `.tar.gz`. Le paquet Arch est ajouté depuis une machine Arch.

Ordre entre dépôts : **flutter**, puis **mobile** (qui pointe dessus), **rust** à part.

## Le contrat entre Dart et Rust

Les deux logiques lisent les **mêmes fichiers de test** (`amounts.json`, `merge_*.json`). Leurs empreintes sont dans `CONTRACT.sha256`, identique dans les dépôts rust et flutter, et vérifiée par la CI. Corriger une règle d'un côté = ajouter un cas à ces fichiers, des deux côtés.
