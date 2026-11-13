#!/usr/bin/env bash
# Clicks through the sidebar sections and screenshots each one.
#
# Needs `ydotool` for synthetic input and `spectacle` for captures. The
# coordinates are the sidebar rows at the default window size; they are the one
# part that has to change if the sidebar's metrics do.
set -euo pipefail

OUT_DIR="${1:-/tmp}"
SLEEP=2

click() {
  ydotool mousemove --absolute "$1" "$2"
  sleep 0.2
  ydotool click 0x40
  sleep "$SLEEP"
}

shoot() {
  spectacle -b -n -f -o "${OUT_DIR}/section_$1.png" 2>/dev/null || true
  echo "  ${OUT_DIR}/section_$1.png"
}

# ydotool needs a negative y offset for the compositor's title strip; the values
# below were measured against the running window.
SWITCH_TO_SECTION=(
  "search 90 83"
  "home 90 113"
  "new 90 143"
  "radio 90 179"
  "library 90 241"
  "settings 90 273"
)

for entry in "${SWITCH_TO_SECTION[@]}"; do
  read -r name x y <<<"$entry"
  echo "capturing $name"
  click "$x" "$y"
  shoot "$name"
done
