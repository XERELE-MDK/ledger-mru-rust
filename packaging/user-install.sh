#!/bin/sh
# Installs this Ledger MRU build for the current user only (no sudo), in ~/.local.
#   ./install.sh            install or update
#   ./install.sh --remove   uninstall (your ledger data is not touched)
# Layout of this folder: usr/ (→ ~/.local, ~/.local/share) and maybe opt/ (→ ~/.local/opt).
set -e
case ${1:-} in -h|--help) sed -n '2,5p' "$0"; exit 0 ;; esac
cd "$(dirname "$0")"
[ -d usr ] || { echo "Run the install.sh found inside the unpacked archive." >&2; exit 1; }
L=$HOME/.local
D=${XDG_DATA_HOME:-$L/share}

dest() { # path inside this folder → installed path
  case $1 in
    usr/bin/*)   echo "$L/bin/${1#usr/bin/}" ;;
    usr/share/*) echo "$D/${1#usr/share/}" ;;
  esac
}

if [ "$1" = --remove ]; then
  for f in $(find usr -type f -o -type l); do rm -f "$(dest "$f")"; done
  for o in opt/*; do [ -e "$o" ] && rm -rf "$L/$o"; done
  echo "Removed."
  exit 0
fi

for o in opt/*; do # private app folders (Flutter bundle)
  [ -e "$o" ] || continue
  rm -rf "$L/$o"; mkdir -p "$L/opt"; cp -a "$o" "$L/$o"
done
for f in $(find usr -type f -o -type l); do
  t=$(dest "$f"); mkdir -p "$(dirname "$t")"; rm -f "$t"
  if [ -L "$f" ]; then # usr/bin/x -> ../../opt/... becomes ~/.local/bin/x -> ~/.local/opt/...
    ln -s "$L/opt/$(readlink "$f" | sed 's|^\.\./\.\./opt/||')" "$t"
  elif [ "${f%.desktop}" != "$f" ]; then
    # Absolute Exec: launchers may not have ~/.local/bin in PATH.
    sed "s|^Exec=|Exec=$L/bin/|" "$f" > "$t"
  else
    cp -a "$f" "$t"
  fi
  echo "  $t"
done
command -v update-desktop-database >/dev/null && update-desktop-database -q "$D/applications" || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -qtf "$D/icons/hicolor" 2>/dev/null || true
echo "Installed. It is now in your application launcher."
