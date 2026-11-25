#!/usr/bin/env bash
# Prints the CHANGELOG section of a version, and fails when the version numbers
# disagree. Usage: tools/release_notes.sh 1.2.0
set -euo pipefail
cd "$(dirname "$0")/.."
v=${1#v}
found=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
[[ $found == "$v" ]] || { echo "tag $v, but the sources say $found" >&2; exit 1; }
notes=$(awk -v v="$v" '/^## /{on=index($0,"[" v "]")>0; next} /^\[[0-9]/{on=0} on' CHANGELOG.md)
[[ -n ${notes//[$'\n ']/} ]] || { echo "no section [$v] in CHANGELOG.md" >&2; exit 1; }
printf '%s\n' "$notes" | sed "s|](docs/|](https://github.com/XERELE-MDK/ledger-mru-rust/blob/v$v/docs/|g"
