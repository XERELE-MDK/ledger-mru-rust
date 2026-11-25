# Noyau comptable — fonctionnement

[← Toutes les fonctionnalités](../README.md) · [Parcours du code](CODE_WALKTHROUGH.md)

Tout repose sur une **liste d'écritures équilibrées** (Σ débits = Σ crédits). Soldes, totaux et rapports sont recalculés à partir d'elle, jamais stockés.

| Type de compte | Côté normal | Sens |
|---|---|---|
| actif | débit | ce que j'ai |
| dépense | débit | ce qui sort |
| passif | crédit | ce que je dois |
| capitaux propres | crédit | point de départ |
| revenu | crédit | ce qui entre |

Les montants se tapent comme on veut : `1 250,50`, `1,250.50`, `1.250,50`. `1,250` vaut **1250**. `inf`, `1e400`, négatifs et plus de 10¹² sont refusés.
