#!/bin/sh
set -e
./upgrade-rust.sh
./upgrade-crates.sh
./upgrade-actions.sh
