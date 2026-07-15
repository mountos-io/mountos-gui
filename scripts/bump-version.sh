#!/usr/bin/env bash
# Bump (or force-set) the version in both package.json and src-tauri/tauri.conf.json
# (kept in sync, npm and Tauri each read their own file).
# Usage: bump-version.sh <patch|minor|major|set> [X.Y.Z]  ('set' requires the version arg)
set -euo pipefail

BUMP="${1:?Usage: bump-version.sh <patch|minor|major|set> [X.Y.Z]}"
SET_VERSION="${2:-}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
PKG_JSON="${REPO_ROOT}/package.json"
TAURI_CONF="${REPO_ROOT}/src-tauri/tauri.conf.json"

command -v jq &>/dev/null || { echo "[fail] jq is required" >&2; exit 1; }

current=$(jq -r '.version' "$PKG_JSON")
[[ "$current" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "[fail] Invalid version in package.json: '${current}'" >&2; exit 1; }

if [[ "$BUMP" == "set" ]]; then
  [[ -n "$SET_VERSION" ]] || { echo "[fail] 'set' requires a version, e.g. bump-version.sh set 1.0.0" >&2; exit 1; }
  next="$SET_VERSION"
else
  IFS=. read -r major minor patch <<< "$current"
  case "$BUMP" in
    patch) patch=$(( patch + 1 )) ;;
    minor) minor=$(( minor + 1 )); patch=0 ;;
    major) major=$(( major + 1 )); minor=0; patch=0 ;;
    *) echo "[fail] Unknown bump type '${BUMP}'. Use: patch, minor, major, or set" >&2; exit 1 ;;
  esac
  next="${major}.${minor}.${patch}"
fi

tmp=$(mktemp)
jq --arg v "$next" '.version = $v' "$PKG_JSON" > "$tmp" && mv "$tmp" "$PKG_JSON"
tmp=$(mktemp)
jq --arg v "$next" '.version = $v' "$TAURI_CONF" > "$tmp" && mv "$tmp" "$TAURI_CONF"

echo "[ok] ${current} -> ${next} (package.json + tauri.conf.json)"
