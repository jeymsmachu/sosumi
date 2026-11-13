#!/usr/bin/env bash
# Fetches the optional FairPlay runtime payload.
#
# Apple's libraries are not distributed with this project and never will be.
# They are downloaded to the user's own machine, only when the user asks for the
# lossless tier, and every file is verified against a pinned SHA-256 before it is
# used. A file that fails is deleted, not warned about.
#
# This runs only on an explicit action from the app's capabilities screen, or
# when a developer asks for it directly. Nothing fetches in the background.
#
# Usage:
#   scripts/fetch-fairplay-runtime.sh [--dest DIR] [--lock FILE] [--check]
#
#   --dest   where the payload goes (default: the app's data directory)
#   --lock   the pinned manifest (default: crates/sosumi-fairplay/runtime.lock)
#   --check  verify what is present and exit; download nothing
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
lock="$here/crates/sosumi-fairplay/runtime.lock"
dest="${XDG_DATA_HOME:-$HOME/.local/share}/sosumi/runtime/lib64"
check_only=0

while [ $# -gt 0 ]; do
  case "$1" in
    --dest) dest="$2"; shift 2 ;;
    --lock) lock="$2"; shift 2 ;;
    --check) check_only=1; shift ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
done

if [ ! -f "$lock" ]; then
  echo "no pin manifest at $lock" >&2
  exit 1
fi

# The manifest lists `sha256  name  source...` per line, comments with #.
sources=()
names=()
hashes=()
while read -r hash name rest; do
  case "$hash" in ''|'#'*) continue ;; esac
  [ -z "$name" ] && continue
  hashes+=("$hash")
  names+=("$name")
  sources+=("$rest")
done < "$lock"

missing=0
bad=0
for i in "${!names[@]}"; do
  name="${names[$i]}"
  want="${hashes[$i]}"
  path="$dest/$name"

  if [ ! -f "$path" ]; then
    if [ "$check_only" -eq 1 ]; then
      echo "missing: $name"
      missing=$((missing + 1))
    fi
    continue
  fi

  got="$(sha256sum "$path" | cut -d' ' -f1)"
  if [ "$got" != "$want" ]; then
    echo "MISMATCH: $name" >&2
    echo "  expected $want" >&2
    echo "  found    $got" >&2
    bad=$((bad + 1))
    # Refuse rather than warn: the loader would otherwise dlopen whatever is on
    # disk, and that code runs with the process's full privileges.
    rm -f "$path"
  fi
done

if [ "$check_only" -eq 1 ]; then
  if [ "$missing" -eq 0 ] && [ "$bad" -eq 0 ]; then
    echo "runtime payload is complete and verified"
    exit 0
  fi
  echo "$missing missing, $bad altered" >&2
  exit 1
fi

mkdir -p "$dest"
fetched=0
for i in "${!names[@]}"; do
  name="${names[$i]}"
  want="${hashes[$i]}"
  path="$dest/$name"

  if [ -f "$path" ] && [ "$(sha256sum "$path" | cut -d' ' -f1)" = "$want" ]; then
    continue
  fi

  ok=0
  for source in ${sources[$i]}; do
    tmp="$(mktemp "$dest/.$name.XXXXXX")"
    if curl -fsSL --retry 3 --max-time 300 "$source" -o "$tmp"; then
      if [ "$(sha256sum "$tmp" | cut -d' ' -f1)" = "$want" ]; then
        mv "$tmp" "$path"
        ok=1
        fetched=$((fetched + 1))
        break
      fi
      echo "rejected $name from $source: checksum did not match" >&2
    fi
    rm -f "$tmp"
  done

  if [ "$ok" -ne 1 ]; then
    echo "could not fetch $name from any pinned source" >&2
    exit 1
  fi
done

echo "fetched $fetched file(s) into $dest"
echo "every file verified against $lock"
