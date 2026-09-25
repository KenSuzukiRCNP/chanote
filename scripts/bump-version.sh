#!/usr/bin/env sh
# Set the app version in the three places that must agree, then show the git commands
# to tag a release.   Usage: scripts/bump-version.sh 0.3.0
set -e
V="$1"
[ -n "$V" ] || { echo "usage: $0 X.Y.Z"; exit 1; }
cd "$(dirname "$0")/.."
sed -i.bak -E "s/(\"version\": *\")[^\"]+(\")/\1$V\2/" package.json src-tauri/tauri.conf.json
sed -i.bak -E "0,/^version = \"[^\"]+\"/s//version = \"$V\"/" src-tauri/Cargo.toml
rm -f package.json.bak src-tauri/tauri.conf.json.bak src-tauri/Cargo.toml.bak
npm install --package-lock-only --silent
cargo update -p chanote --precise "$V" 2>/dev/null || cargo generate-lockfile >/dev/null 2>&1 || true
echo "version set to $V in package.json, src-tauri/tauri.conf.json, src-tauri/Cargo.toml"
echo "next:  git add -A && git commit -m \"v$V\" && git tag v$V && git push && git push --tags"
