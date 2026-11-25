# Soldes de départ — fonctionnement

[← Toutes les fonctionnalités](../README.md) · [Parcours du code](CODE_WALKTHROUGH.md)

*Depuis la 1.2.0.*

Un montant par compte d'actif et de passif, à une date donnée. L'app en fait **une seule écriture équilibrée** ; la différence va dans les capitaux propres. Ce n'est **pas un revenu**.

- Accueil d'un livre vide, ou *Accounts → Opening balances*.
- Rouvrir l'écran montre les soldes ; les changer remplace l'écriture ; tout effacer la retire.
- Vos propres écritures de capitaux ne sont jamais touchées.
