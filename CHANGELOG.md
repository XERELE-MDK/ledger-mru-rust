# Journal des versions

Format : [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) · numéros : [SemVer](https://semver.org/lang/fr/).
Les trois dépôts ([rust](https://github.com/XERELE-MDK/ledger-mru-rust), [flutter](https://github.com/XERELE-MDK/ledger-mru-flutter), [mobile](https://github.com/XERELE-MDK/ledger-mru-mobile)) avancent avec **le même numéro**.

> Le suivi git commence à la **1.1.3**. Les versions antérieures sont décrites ici pour mémoire : leur code n'a pas été conservé, elles n'ont donc ni étiquette ni fichiers publiés.

## [1.2.0] — 2026-11-25

### Ajouté
- **Opening balances** : ce qu'on a et ce qu'on doit au début, en une écriture équilibrée ([doc](docs/soldes-de-depart/FUNCTIONALITY.md)).
- Documentation par fonctionnalité, compilations automatiques, licence GPL-3.0.
### Corrigé
- `1,250` est lu comme 1250, et non 1,25 : mêmes règles que Flutter, sur des cas partagés.

## [1.1.3] — 2026-11-25

### Ajouté
- « Start with an empty book » : retire la démo, et rien d'autre.
### Corrigé
- Deux démos dans deux langues n'entrent plus en conflit.

## 1.1.2 — 2026-11-25

### Corrigé
- Synchro en boucle : un fichier identique n'est plus réécrit.
- Un fichier arrivé tard, mais plus ancien que les autres, passait inaperçu.
- Annuler une suppression déjà synchronisée ne tenait pas.
- « Reset » et « Replace » sont refusés pendant la synchro.

## 1.1.1 — 2026-11-25

### Corrigé
- Un montant infini (`inf`, `1e400`) rendait le livre illisible.
- Un livre illisible est mis de côté, intact ; la synchro est débranchée.

## 1.1.0 — 2026-11-24

### Ajouté
- Nouvelle icône, paquets Linux, compatibilité glibc 2.34.

[1.2.0]: https://github.com/XERELE-MDK/ledger-mru-rust/releases/tag/v1.2.0
[1.1.3]: https://github.com/XERELE-MDK/ledger-mru-rust/releases/tag/v1.1.3
