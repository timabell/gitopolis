#!/bin/sh
set -e # exit on error

if ! git diff --cached --quiet; then
  echo "Aborted: staged changes detected in git"
  exit 1
fi

latest=$(mise ls-remote rust | grep -P '^\d+\.\d+\.\d+$' | sort --version-sort | tail -n 1)
echo "Updating to rust@$latest"

mise use "rust@$latest"

if git diff --quiet .tool-versions; then
  echo "Already on latest rust, nothing to do"
  exit 0
fi

echo "Running regression tests..."
cargo test

echo "Committing upgrades..."
git commit --include .tool-versions \
--message "chore: Upgrade build to rust $latest" \
--message 'Upgraded by `upgrade-rust.sh`' \
--message 'bump: patch'
