#!/bin/sh -v
set -e # exit on error

latest=`mise ls-remote rust | grep -P '^\d+\.\d+\.\d+$' | sort --version-sort | tail -n 1`
echo "Updating to rust@$latest"

mise use "rust@$latest"

cargo test

git commit -i .tool-versions -m "chore: Upgrade build to latest rust"
