#!/bin/sh
set -e # exit on error
echo ""
echo "🧹 fmt..."
cargo fmt

echo ""
echo "🧹 clippy..."
./clippy-harsh.sh

echo ""
echo "🧹 cargo-deny..."
cargo deny check licenses

echo ""
echo "🧹 yaml-lint..."
# Check YAML files in .github
if command -v yamllint >/dev/null 2>&1; then
    echo "Checking GitHub YAML files..."
    for file in .github/**/*.yml .github/**/*.yaml; do
        if [ -f "$file" ]; then
            echo "Checking $file"
            yamllint -d '{extends: relaxed, rules: {line-length: disable}}' "$file"
        fi
    done
else
    echo "yamllint not installed, skipping YAML checks"
fi
