#!/bin/sh
set -e # exit on error

if ! git diff --cached --quiet; then
  echo "Aborted: staged changes detected in git"
  exit 1
fi

cargo update
cargo install cargo-edit
cargo upgrade --incompatible # from cargo-edit

if git diff --quiet Cargo.toml Cargo.lock; then
  echo "No crate updates available, nothing to do"
  exit 0
fi

echo "Running regression tests..."
cargo test

echo "Committing upgrades..."
git commit --include Cargo.toml --include Cargo.lock \
--message "chore: Dependency upgrades (cargo)" \
--message 'Upgraded by `upgrade-crates.sh`' \
--message 'bump: patch'
