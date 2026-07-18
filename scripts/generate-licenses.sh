#!/usr/bin/env bash
# Regenerates LICENSES/{rust,js}.json from Cargo.lock/package-lock.json and
# stages the result. Wired into lefthook's pre-commit; run with --force to
# regenerate even when the lockfiles haven't changed.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "$REPO_ROOT"

if [[ "${1:-}" != "--force" ]] && git diff --quiet HEAD -- src-tauri/Cargo.lock package-lock.json; then
  echo "Cargo.lock/package-lock.json unchanged, skipping license update. Use --force to regenerate."
  exit 0
fi

node scripts/generate-licenses.mjs

git add LICENSES/rust.json LICENSES/js.json
echo "License files updated and staged."
