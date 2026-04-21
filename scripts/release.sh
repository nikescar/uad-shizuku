#!/usr/bin/env bash
set -e

cargo clippy --fix

# Fetch latest release version from GitHub
echo "Fetching latest release version..."
LATEST_TAG=$(curl -s https://api.github.com/repos/nikescar/uad-shizuku/releases/latest | grep '"tag_name"' | cut -d'"' -f4)
echo "Latest release: $LATEST_TAG"

# Extract version number (remove 'v' prefix) and increment patch version
CURRENT_VERSION=${LATEST_TAG#v}
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"
NEW_PATCH=$((PATCH + 1))
NEW_VERSION="$MAJOR.$MINOR.$NEW_PATCH"
echo "New version: v$NEW_VERSION"

# Update version in Cargo.toml files
echo "Updating Cargo.toml files..."
sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
sed -i "s/^versioncode = \".*\"/versioncode = \"$NEW_VERSION\"/" Cargo.toml

cargo fmt --all
git-cliff -o CHANGELOG.md
embedmd -w README.md

echo "Release preparation complete for version v$NEW_VERSION"