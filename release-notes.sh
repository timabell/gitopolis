#!/bin/sh
# Preview git-cliff output

SCRIPT_DIR="$(dirname "$0")"
BUMP_TYPE=$("$SCRIPT_DIR/.github/workflows/detect-bump.sh")
BUMP_ARG=""
if [ -n "$BUMP_TYPE" ]; then
  BUMP_ARG="--bump $BUMP_TYPE"
fi

case "$1" in
  --preview)
    git cliff --config .github/cliff.toml $BUMP_ARG --unreleased
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
