#!/usr/bin/env bash
# Checks that relative Markdown links in the docs resolve.
#
# Every link is resolved against the directory of the file that contains it,
# which is how a Markdown renderer resolves it. A check that assumed a single
# base directory would report false positives for the ADR index and for
# cross-references between `docs/` files.
set -euo pipefail

status=0

check_file() {
  local file="$1"
  local dir
  dir="$(dirname "$file")"

  # Pull every Markdown link target out of the file.
  while IFS= read -r target; do
    # Strip an anchor and skip anything external or empty.
    local path="${target%%#*}"
    [ -z "$path" ] && continue
    case "$path" in
      http://*|https://*|mailto:*) continue ;;
    esac

    # Absolute paths are rooted at the repository, not the file.
    local resolved
    if [[ "$path" == /* ]]; then
      resolved=".${path}"
    else
      resolved="${dir}/${path}"
    fi

    if [ ! -e "$resolved" ]; then
      echo "::error file=${file}::broken link '${target}'"
      status=1
    fi
  done < <(grep -oE '\]\([^)]+\)' "$file" | sed -E 's/^\]\(//; s/\)$//' | sort -u)
}

while IFS= read -r file; do
  check_file "$file"
done < <(find . -name '*.md' \
  -not -path './.references/*' \
  -not -path './target/*' \
  -not -path './apps/desktop/build/*' \
  -not -path './.git/*')

if [ "$status" -eq 0 ]; then
  echo "All documentation links resolve."
fi
exit "$status"
