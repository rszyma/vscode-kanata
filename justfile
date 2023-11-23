_default:
    @just -l --unsorted

install:
    git submodule update
    make package
    code --install-extension kanata.vsix

# Creates a commit that updates kanata to latest git and adds notice about it to CHANGELOG.md
bump_kanata:
    #!/bin/sh
    set -euxo pipefail
    git submodule update --remote
    cd kanata
    HASH=$(git rev-parse --short HEAD)
    cd ..
    # Exit early without updating changelog if a bump notice was already added in "Unreleased" section.
    ! grep -q "$HASH" CHANGELOG.md
    awk '/^### [0-9]/ && found==0 {found=1} found==0 && /Updated kanata to/ {next} 1' CHANGELOG.md > temp && mv temp CHANGELOG.md
    just _add_to_changelog "Updated kanata to \[$HASH\]\(https\:\/\/github\.com\/jtroo\/kanata\/tree\/$HASH\)"
    just _ensure_no_staged_changes
    git add CHANGELOG.md kanata
    git commit -m "chore: bump kanata to $HASH"

# Bumps version number, pushes a "new version" commit/tags, builds, uploads to VS Code marketplace.
release VERSION:
    just _ensure_clean_directory

    git checkout main
    git pull
    git checkout -b release-v{{VERSION}}
    sed -i 's/\"version\": \"[^\"]*\"/\"version\": \"{{VERSION}}\"/' package.json
    sed -i 's/### Unreleased/### Unreleased\n\n* no changes yet\n\n### {{VERSION}}/' CHANGELOG.md
    git commmit -m "Release v{{VERSION}}"
    git push origin release-v{{VERSION}}

    git tag v{{VERSION}}
    git push --tags

    vsce publish {{VERSION}}

_add_to_changelog TEXT:
    sed -i '/no changes yet/Id' CHANGELOG.md
    sed -i "N;s/^### Unreleased\n/\0\n\* {{TEXT}}/" CHANGELOG.md

_ensure_clean_directory:
    git diff-index --quiet HEAD --

_ensure_no_staged_changes:
    git diff --cached --quiet
