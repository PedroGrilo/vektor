#!/usr/bin/env bash
# Downloads NVIDIA Profile Inspector (MIT) into src-tauri/resources/npi and verifies its hash.
set -euo pipefail
VERSION="3.0.2.1"
SHA256="88dcf3514111e8de630688467c03c36d8c2a8ad9ebc8073f27c069f82b75bb40"
DEST="$(cd "$(dirname "$0")/.." && pwd)/src-tauri/resources/npi"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

curl -fsSL -o "$TMP/npi.zip" "https://github.com/Orbmu2k/nvidiaProfileInspector/releases/download/v${VERSION}/nvidiaProfileInspector.zip"
if command -v sha256sum >/dev/null; then got=$(sha256sum "$TMP/npi.zip" | cut -d' ' -f1); else got=$(shasum -a 256 "$TMP/npi.zip" | cut -d' ' -f1); fi
[ "$got" = "$SHA256" ] || { echo "hash mismatch: $got" >&2; exit 1; }

rm -rf "$DEST" && mkdir -p "$DEST"
if command -v unzip >/dev/null; then
  unzip -q -o "$TMP/npi.zip" nvidiaProfileInspector.exe nvidiaProfileInspector.exe.config Reference.xml -d "$DEST"
else
  python - "$TMP/npi.zip" "$DEST" <<'PYEOF'
import sys, zipfile
z = zipfile.ZipFile(sys.argv[1])
for name in ("nvidiaProfileInspector.exe", "nvidiaProfileInspector.exe.config", "Reference.xml"):
    z.extract(name, sys.argv[2])
PYEOF
fi
curl -fsSL -o "$DEST/LICENSE.txt" "https://raw.githubusercontent.com/Orbmu2k/nvidiaProfileInspector/master/LICENSE.txt"
echo "NVIDIA Profile Inspector ${VERSION} -> $DEST"
