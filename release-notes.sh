#!/bin/sh
# Preview git-cliff output

case "$1" in
  --preview)
    git cliff --config .github/cliff.toml --bump --unreleased
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
