#!/bin/sh
set -e # exit on error

# sed avoids extra tooling. Alternatives:
# - https://github.com/suzuki-shunsuke/pinact - Go binary, pins to SHAs
# - https://github.com/azat-io/actions-up - npm, `--style preserve` keeps tags
# - https://docs.github.com/en/code-security/dependabot - automatic PRs, also updates actions.lock
# - https://docs.github.com/en/actions/how-tos/secure-your-work/lock-workflow-dependencies - pins, doesn't upgrade

if ! git diff --cached --quiet; then
  echo "Aborted: staged changes detected in git"
  exit 1
fi

# owner/repo of every non-local action
actions=$(grep -ho 'uses: [^.][^@ ]*@' .github/workflows/*.yml | sed 's/^uses: //; s/@$//' | cut -d/ -f1,2 | sort -u)

for action in $actions; do
  echo "+ gh api repos/$action/releases/latest --jq .tag_name"
  latest=$(gh api "repos/$action/releases/latest" --jq .tag_name) || { echo "No release found for $action, skipping"; continue; }
  # Major tag only (v4.2.2 -> v4) so patch releases flow in without commits
  major="${latest%%.*}"
  echo "$action -> $major"
  sed -i -E "s|(uses: $action(/[^@ ]*)?)@[^ ]+|\1@$major|" .github/workflows/*.yml
done

if git diff --quiet .github/workflows/*.yml; then
  echo "Already on latest actions, nothing to do"
  exit 0
fi

echo "Committing upgrades..."
git commit --include .github/workflows/*.yml \
--message "ci: Upgrade GitHub Actions" \
--message 'Upgraded by `upgrade-actions.sh`'
