#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"

if [ -z "$VERSION" ]; then
  echo "Usage: ./release.sh <version>"
  exit 1
fi

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9._-]+)?$ ]]; then
  echo "Version must look like 1.2.3 or 1.2.3-rc.1"
  exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
  echo "Working tree is dirty. Commit or stash changes before releasing."
  exit 1
fi

echo "Updating dependencies..."
cargo update

echo "Pre-release checks..."
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test --all-features
cargo bench --bench examples --no-run

echo "Updating Cargo.toml to $VERSION"
sed -i.bak -E "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
rm Cargo.toml.bak

echo "Post-version checks..."
cargo check --all-features
cargo test --all-features
cargo publish --locked --dry-run --allow-dirty

git add Cargo.toml Cargo.lock
git commit -m "Release v$VERSION"

git tag -a "v$VERSION" -m "Release v$VERSION"

git push
git push origin "v$VERSION"

echo "Release v$VERSION complete"
echo "The v$VERSION tag will trigger the GitHub Actions publish workflow."
