#!/usr/bin/env bash
# Launches Sosumi detached, waits for its window, then screenshots it.
#
# Detaching matters: a process started from a shell tool call is killed when
# that call ends, so the app vanishes before a later call can screenshot it.
# `setsid` puts it in its own session so it survives.
#
# Usage: scripts/run-and-shoot.sh [output.png] [wait-seconds]
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUNDLE="$HERE/apps/desktop/build/linux/x64/debug/bundle/sosumi"
OUT="${1:-/tmp/sosumi-shot.png}"
WAIT="${2:-8}"

if [ ! -x "$BUNDLE" ]; then
  echo "no built binary at $BUNDLE" >&2
  echo "build it first: (cd apps/desktop && flutter build linux --debug)" >&2
  exit 1
fi

# Stop any previous instance so the screenshot is unambiguous.
pkill -f "bundle/sosumi" 2>/dev/null || true
sleep 1

setsid "$BUNDLE" >/tmp/sosumi.log 2>&1 < /dev/null &
disown || true

echo "launched, waiting ${WAIT}s for the window"
sleep "$WAIT"

if ! pgrep -f "bundle/sosumi" >/dev/null; then
  echo "the app exited; log follows:" >&2
  tail -20 /tmp/sosumi.log >&2
  exit 1
fi

spectacle -b -n -f -o "$OUT" 2>/dev/null || true
sleep 1

if [ -f "$OUT" ]; then
  echo "screenshot: $OUT"
else
  echo "screenshot failed; is spectacle installed?" >&2
  exit 1
fi
