#!/usr/bin/env bash
set -e

# Sets the WINDOWS_CERT_P12_BASE64 repository secret used by
# .github/workflows/release.yml directly from certificate.pfx via gh CLI.
# Run from the scripts/ dir with certificate.pfx present (see mobile/README.md).
#
# Usage: ./wincert_gh_secret.sh [path/to/certificate.pfx]

CERT_FILE="${1:-certificate.pfx}"

if ! command -v gh >/dev/null 2>&1; then
  echo "Error: gh CLI not found. Install it from https://cli.github.com/" >&2
  exit 1
fi

if [ ! -f "$CERT_FILE" ]; then
  echo "Error: $CERT_FILE not found. Place certificate.pfx here or pass a path as the first argument." >&2
  exit 1
fi

base64 < "$CERT_FILE" | gh secret set WINDOWS_CERT_P12_BASE64

echo "WINDOWS_CERT_P12_BASE64 set from $CERT_FILE."
echo "Don't forget to also set WINDOWS_CERT_PASSWORD:"
echo "  gh secret set WINDOWS_CERT_PASSWORD --body '<password>'"
