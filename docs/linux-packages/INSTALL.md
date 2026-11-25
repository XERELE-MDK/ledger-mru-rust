# Linux packages — setup

[← All features](../README.md) · [How it works](FUNCTIONALITY.md) · [Code walkthrough](CODE_WALKTHROUGH.md)

```bash
sudo apt install libfontconfig1-dev rpm        # Debian / Ubuntu
sudo pacman -S fontconfig rpm-tools dpkg       # Arch
packaging/build_packages.sh
```

Check a download: `sha256sum -c SHA256SUMS --ignore-missing`.
