#!/bin/sh
set -e
# Calculate the next release version from the highest vX.Y.Z tag and any
# "bump:" footer (see detect-bump.sh). Defaults to a patch bump.
# Highest tag rather than git describe, so a release tag that isn't an
# ancestor of HEAD can't be reused.
# Outputs: version without "v" prefix, e.g. 1.18.0
#
# Not git-cliff --bumped-version: it won't bump without release-worthy commits

SCRIPT_DIR="$(dirname "$0")"
BUMP_TYPE=$("$SCRIPT_DIR/detect-bump.sh")

LAST_TAG=$(git tag --list 'v[0-9]*' | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -n 1)
LAST_TAG="${LAST_TAG:-v0.0.0}"
VERSION="${LAST_TAG#v}"
MAJOR="${VERSION%%.*}"
REST="${VERSION#*.}"
MINOR="${REST%%.*}"
PATCH="${REST#*.}"

case "$BUMP_TYPE" in
  major) echo "$((MAJOR + 1)).0.0" ;;
  minor) echo "$MAJOR.$((MINOR + 1)).0" ;;
  *)     echo "$MAJOR.$MINOR.$((PATCH + 1))" ;;
esac
