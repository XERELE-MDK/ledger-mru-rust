# Scénario de captures — parcours du code

[← Toutes les fonctionnalités](../README.md) · [Fonctionnement](FUNCTIONALITY.md)

Module `shots` de `src/main.rs`.

- `MinimalSoftwareWindow` remplace la plateforme ; le rendu logiciel dessine dans un tableau de pixels.
- Les vrais callbacks sont appelés (`invoke_…`) : c'est un test de bout en bout.
- La CI le joue à chaque envoi et garde les images 14 jours.
