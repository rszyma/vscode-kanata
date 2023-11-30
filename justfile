_default:
    @just -l --unsorted

install:
    git submodule update
    make package
    code --install-extension kanata.vsix

# Creates a commit, that updates kanata to latest git and adds notice about it to CHANGELOG.md
bump_kanata:
    #!/bin/bash
    set -euxo pipefail
    git submodule update --remote
    cd kanata
    HASH=$(git rev-parse --short HEAD)
    cd ..
    # Exit early without updating changelog if a bump notice has already been added in "Unreleased" section.
    # This works because of a some weird behavior of bash. More about it here:
    # http://redsymbol.net/articles/unofficial-bash-strict-mode/#short-circuiting
    grep -q "$HASH" CHANGELOG.md && (exit 123)
    awk '/^### [0-9]/ && found==0 {found=1} found==0 && /Updated kanata to/ {next} 1' CHANGELOG.md > temp && mv temp CHANGELOG.md
    just _add_to_changelog "Updated kanata to \[$HASH\]\(https\:\/\/github\.com\/jtroo\/kanata\/tree\/$HASH\)"
    just _ensure_no_staged_changes
    git add CHANGELOG.md kanata
    git commit -m "chore: bump kanata to $HASH"

release VERSION:
    just _ensure_no_staged_changes
    git checkout main
    git pull
    sed -i 's/\"version\": \"[^\"]*\"/\"version\": \"{{VERSION}}\"/' package.json
    sed -i 's/### Unreleased/### Unreleased\n\n* no changes yet\n\n### {{VERSION}}/' CHANGELOG.md
    vsce publish {{VERSION}}
    git add CHANGELOG.md package.json
    git commit -m "Release v{{VERSION}}"
    git push
    git tag v{{VERSION}}
    git push --tags

pre_release VERSION:
    just _ensure_no_staged_changes
    git checkout main
    git pull
    sed -i 's/\"version\": \"[^\"]*\"/\"version\": \"{{VERSION}}\"/' package.json
    sed -i 's/### Unreleased/### Unreleased\n\n* no changes yet\n\n### {{VERSION}}/' CHANGELOG.md
    vsce publish {{VERSION}} --pre-release
    git add CHANGELOG.md package.json
    git commit -m "Pre-Release v{{VERSION}}"
    git push
    git tag v{{VERSION}}
    git push --tags

use_local_repo:
    sed -i 's/kanata\/parser/kanata-local\/parser/' kls/Cargo.toml

use_origin_repo:
    sed -i 's/kanata-local\/parser/kanata\/parser/' kls/Cargo.toml

_add_to_changelog TEXT:
    sed -i '/no changes yet/Id' CHANGELOG.md
    sed -i "N;s/^### Unreleased\n/\0\n\* {{TEXT}}/" CHANGELOG.md

_ensure_clean_directory:
    git diff-index --quiet HEAD --

_ensure_no_staged_changes:
    git diff --cached --quiet
