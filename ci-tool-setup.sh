#!/bin/sh -v
# Tools needed to run the ci checks locally, only needed when debugging ci failures
cargo binstall --locked cargo-deny
pip install --user yamllint
