#!/bin/sh
set -e
# Detect explicit version bump from commit footers since last tag
# Outputs: major, minor, patch, or nothing if no footer found
#
# Usage: Add a footer line "bump: major", "bump: minor" or "bump: patch"
# to any commit. Any of these forces a release even if no commits are
# otherwise release-worthy.

LAST_TAG=$(git describe --tags --abbrev=0 --match 'v[0-9]*' 2>/dev/null || echo "")

if [ -z "$LAST_TAG" ]; then
  COMMITS=$(git log --format=%B)
else
  COMMITS=$(git log "$LAST_TAG"..HEAD --format=%B)
fi

for type in major minor patch; do
  if echo "$COMMITS" | grep -qE "^bump: $type$"; then
    echo "$type"
    exit 0
  fi
done
