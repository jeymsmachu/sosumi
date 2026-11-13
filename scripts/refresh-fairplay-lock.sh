#!/usr/bin/env bash
# Regenerates the FairPlay runtime pin file from a verified local payload.
#
# The pin file exists so a fetched file can be checked before use. A wrong hash
# in it is worse than no hash at all: it would reject a legitimate download, or
# accept a corrupted one, and the failure would look like a network problem.
#
# This script therefore checks every hash it writes against the bytes actually on
# disk, and refuses to write a line it cannot verify.
#
# Usage:
#   scripts/refresh-fairplay-lock.sh <lib64-dir> [--check]
#
#   <lib64-dir>  a directory containing Apple's libraries, such as the one
#                .references/fairplay-wrapper/rootfs/system/lib64 provides
#   --check      report whether the current pin file matches; write nothing
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
lock="$here/crates/sosumi-fairplay/runtime.lock"

source_a="https://raw.githubusercontent.com/WorldObservationLog/wrapper/2fd7e0eaef081f93cf687428b965674937832ba4/rootfs/system/lib64"
source_b="https://raw.githubusercontent.com/silentone12725/wrapper/7402034f56dc63e34cea41fb93149d5a6e7c295e/rootfs/system/lib64"

if [ $# -lt 1 ]; then
  echo "usage: $0 <lib64-dir> [--check]" >&2
  exit 2
fi
lib64="$1"
check_only=0
[ "${2:-}" = "--check" ] && check_only=1

if [ ! -d "$lib64" ]; then
  echo "not a directory: $lib64" >&2
  exit 1
fi

# Every file Apple's libraries need, in the order the reference lock lists them.
files="libCoreFP.so libandroidappmusic.so libstoreservicescore.so libCoreLSKD.so
libCoreADI.so libCoreFoundation.so libmedialibrarycore.so libmediaplatform.so
libdaapkit.so libc++_shared.so libc.so libm.so libstdc++.so liblog.so libz.so
libxml2.so libcurl.so libBlocksRuntime.so libdispatch.so libicudata_sv_apple.so
libicui18n_sv_apple.so libicuuc_sv_apple.so"

# Verify every file first. Writing a line for a missing or unreadable file would
# produce a pin that can never be satisfied.
missing=0
for name in $files; do
  if [ ! -f "$lib64/$name" ]; then
    echo "missing: $name" >&2
    missing=$((missing + 1))
  fi
done
if [ "$missing" -gt 0 ]; then
  echo "$missing file(s) absent from $lib64; refusing to write a partial pin" >&2
  exit 1
fi

if [ "$check_only" -eq 1 ]; then
  # Compare what the pin says against what is on disk.
  drift=0
  while read -r hash name _; do
    case "$hash" in ''|'#'*) continue ;; esac
    actual="$(sha256sum "$lib64/$name" | cut -d' ' -f1)"
    if [ "$actual" != "$hash" ]; then
      echo "drift: $name" >&2
      echo "  pin  $hash" >&2
      echo "  disk $actual" >&2
      drift=$((drift + 1))
    fi
  done < "$lock"
  if [ "$drift" -eq 0 ]; then
    echo "every pinned file matches the payload on disk"
    exit 0
  fi
  exit 1
fi

tmp="$(mktemp)"
{
  echo "# Apple's Android FairPlay libraries, fetched on the user's machine."
  echo "#"
  echo "# These cannot be redistributed with Sosumi, so they are not in this"
  echo "# repository and not in any package. They are downloaded only when the user"
  echo "# asks for the lossless tier, and every file is verified against the SHA-256"
  echo "# below. A file that does not match is refused and deleted, never warned"
  echo "# about: the loader would otherwise dlopen whatever happens to be on disk,"
  echo "# and that code runs with the process's full privileges."
  echo "#"
  echo "# Format: <sha256>  <filename>  <source url> [...fallback urls]"
  echo "#"
  echo "# Both sources are pinned to exact commits, so an upstream change cannot"
  echo "# alter what users install. Regenerate with scripts/refresh-fairplay-lock.sh."
  echo "#"
  echo "# Our own empty stubs for libandroid.so, libOpenSLES.so, and libdl.so are"
  echo "# built from source in this repository, not fetched; the installer places"
  echo "# them alongside these files."
  echo
  for name in $files; do
    hash="$(sha256sum "$lib64/$name" | cut -d' ' -f1)"
    printf '%s  %s  %s/%s  %s/%s\n' "$hash" "$name" "$source_a" "$name" "$source_b" "$name"
  done
} > "$tmp"

mv "$tmp" "$lock"
echo "wrote $(grep -cE '^[0-9a-f]{64}' "$lock") pinned entries to $lock"
