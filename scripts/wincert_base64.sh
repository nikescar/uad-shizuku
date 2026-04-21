#!/usr/bin/env bash

# .github/workflows/release.yml
# WINDOWS_CERT_P12_BASE64: ${{ secrets.WINDOWS_CERT_P12_BASE64 }}
# WINDOWS_CERT_PASSWORD: ${{ secrets.WINDOWS_CERT_PASSWORD }}

base64 -i certificate.pfx