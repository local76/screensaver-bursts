#!/bin/bash
set -e

# Change directory to project root
cd "$(dirname "$0")/.."

echo "Running tests..."
./scripts/test.sh

echo "Building release..."
./scripts/build.sh

VERSION=$(grep "^version" Cargo.toml | head -n 1 | cut -d '"' -f 2)
if [ -z "$VERSION" ]; then
    VERSION="v$(date +%Y.%m.%d)"
else
    VERSION="v$VERSION"
fi

echo "Tagging Git release $VERSION..."
if git rev-parse "$VERSION" >/dev/null 2>&1; then
    echo "Tag $VERSION already exists. Deleting local tag first..."
    git tag -d "$VERSION"
fi
git tag -a "$VERSION" -m "Release $VERSION"

echo "Pushing tag $VERSION to origin..."
if git remote | grep -q "origin"; then
    git push origin "$VERSION"
else
    echo "No 'origin' remote found, skipping tag push."
fi

echo "Release $VERSION complete!"
