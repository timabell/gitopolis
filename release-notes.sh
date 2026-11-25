#!/bin/sh
# Preview git-cliff output

SCRIPT_DIR="$(dirname "$0")"

case "$1" in
  --preview)
    NEXT_TAG="v$("$SCRIPT_DIR/.github/workflows/next-version.sh")"
    git cliff --config .github/cliff.toml --unreleased --tag "$NEXT_TAG"
    ;;
  "")
    git cliff --config .github/cliff.toml --latest
    ;;
  *)
    echo "Unknown flag: $1" >&2
    echo "Usage: $0 [--preview]" >&2
    exit 1
    ;;
esac
