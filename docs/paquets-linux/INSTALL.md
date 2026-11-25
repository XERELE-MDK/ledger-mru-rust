# Paquets Linux — mise en place

[← Toutes les fonctionnalités](../README.md) · [Fonctionnement](FUNCTIONALITY.md) · [Parcours du code](CODE_WALKTHROUGH.md)

```bash
sudo apt install libfontconfig1-dev rpm        # Debian / Ubuntu
sudo pacman -S fontconfig rpm-tools dpkg       # Arch
packaging/build_packages.sh
```

Vérifier un téléchargement : `sha256sum -c SHA256SUMS --ignore-missing`.
