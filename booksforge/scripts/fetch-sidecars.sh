#!/usr/bin/env bash
# Reproducibly fetch + verify sidecar binaries (Pandoc, EPUBCheck)
# into booksforge/binaries/ for the current host platform.
#
# Usage:
#   ./scripts/fetch-sidecars.sh                 # current host only
#   ./scripts/fetch-sidecars.sh --all-platforms  # all four matrix targets
#
# Refs: booksforge/binaries/README.md, MILESTONES.md M5,
#       EXTERNAL_AUDIT_BACKLOG.md M4.

set -euo pipefail

cd "$(dirname "$0")/.."

PANDOC_VERSION="3.5"
EPUBCHECK_VERSION="5.1.0"

# Pinned SHA-256 checksums.  Updating a version requires updating
# the checksum below from the upstream release notes.  Never trust
# a redirected download — always verify the hash.
declare -A PANDOC_CHECKSUMS=(
  ["aarch64-apple-darwin"]="REPLACE_ME_pandoc_3.5_macos_arm64_sha256"
  ["x86_64-apple-darwin"]="REPLACE_ME_pandoc_3.5_macos_x64_sha256"
  ["x86_64-pc-windows-msvc"]="REPLACE_ME_pandoc_3.5_windows_x64_sha256"
  ["x86_64-unknown-linux-gnu"]="REPLACE_ME_pandoc_3.5_linux_x64_sha256"
)

EPUBCHECK_SHA256="REPLACE_ME_epubcheck_5.1.0_zip_sha256"

DEST_DIR="binaries"
mkdir -p "${DEST_DIR}"

# ── Detect host target ────────────────────────────────────────────
host_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "${os}/${arch}" in
    Darwin/arm64)        echo "aarch64-apple-darwin" ;;
    Darwin/x86_64)       echo "x86_64-apple-darwin" ;;
    Linux/x86_64)        echo "x86_64-unknown-linux-gnu" ;;
    MINGW*|MSYS*|CYGWIN*) echo "x86_64-pc-windows-msvc" ;;
    *) echo "ERROR: unsupported host ${os}/${arch}" >&2; exit 1 ;;
  esac
}

ALL_TARGETS=(
  "aarch64-apple-darwin"
  "x86_64-apple-darwin"
  "x86_64-pc-windows-msvc"
  "x86_64-unknown-linux-gnu"
)

if [[ "${1:-}" == "--all-platforms" ]]; then
  TARGETS=("${ALL_TARGETS[@]}")
else
  TARGETS=("$(host_target)")
fi

echo "Fetching for targets: ${TARGETS[*]}"

# ── Pandoc ────────────────────────────────────────────────────────
fetch_pandoc() {
  local target="$1"
  local pandoc_url
  local archive_path
  local extracted_path

  case "${target}" in
    aarch64-apple-darwin)
      pandoc_url="https://github.com/jgm/pandoc/releases/download/${PANDOC_VERSION}/pandoc-${PANDOC_VERSION}-arm64-macOS.zip"
      archive_path="pandoc-${PANDOC_VERSION}-arm64-macOS.zip"
      extracted_path="pandoc-${PANDOC_VERSION}-arm64/bin/pandoc"
      ;;
    x86_64-apple-darwin)
      pandoc_url="https://github.com/jgm/pandoc/releases/download/${PANDOC_VERSION}/pandoc-${PANDOC_VERSION}-x86_64-macOS.zip"
      archive_path="pandoc-${PANDOC_VERSION}-x86_64-macOS.zip"
      extracted_path="pandoc-${PANDOC_VERSION}-x86_64/bin/pandoc"
      ;;
    x86_64-pc-windows-msvc)
      pandoc_url="https://github.com/jgm/pandoc/releases/download/${PANDOC_VERSION}/pandoc-${PANDOC_VERSION}-windows-x86_64.zip"
      archive_path="pandoc-${PANDOC_VERSION}-windows-x86_64.zip"
      extracted_path="pandoc-${PANDOC_VERSION}/pandoc.exe"
      ;;
    x86_64-unknown-linux-gnu)
      pandoc_url="https://github.com/jgm/pandoc/releases/download/${PANDOC_VERSION}/pandoc-${PANDOC_VERSION}-linux-amd64.tar.gz"
      archive_path="pandoc-${PANDOC_VERSION}-linux-amd64.tar.gz"
      extracted_path="pandoc-${PANDOC_VERSION}/bin/pandoc"
      ;;
    *) echo "ERROR: unknown target ${target}" >&2; return 1 ;;
  esac

  local final_name="pandoc-${PANDOC_VERSION}-${target}"
  if [[ "${target}" == *windows* ]]; then
    final_name="${final_name}.exe"
  fi

  local final_path="${DEST_DIR}/${final_name}"

  if [[ -f "${final_path}" ]]; then
    echo "  → ${final_name} already present; skipping"
    return 0
  fi

  echo "  → Fetching ${pandoc_url}"
  curl --location --fail --output "${DEST_DIR}/${archive_path}" "${pandoc_url}"

  # Verify SHA-256
  local expected="${PANDOC_CHECKSUMS[${target}]:-}"
  if [[ -z "${expected}" || "${expected}" == REPLACE_ME_* ]]; then
    echo "  ⚠  No pinned SHA-256 for ${target} — populate PANDOC_CHECKSUMS in this script"
    echo "     Computed:"
    shasum -a 256 "${DEST_DIR}/${archive_path}"
  else
    local actual
    actual="$(shasum -a 256 "${DEST_DIR}/${archive_path}" | awk '{print $1}')"
    if [[ "${actual}" != "${expected}" ]]; then
      echo "  ✗ SHA-256 mismatch for Pandoc ${target}"
      echo "    expected: ${expected}"
      echo "    actual:   ${actual}"
      rm -f "${DEST_DIR}/${archive_path}"
      return 1
    fi
    echo "  ✓ SHA-256 verified"
  fi

  # Extract just the binary we need.
  echo "  → Extracting ${extracted_path}"
  case "${archive_path}" in
    *.zip)     ( cd "${DEST_DIR}" && unzip -j "${archive_path}" "${extracted_path}" -d "extract-${target}" ) ;;
    *.tar.gz)  ( cd "${DEST_DIR}" && tar -xzf "${archive_path}" --strip-components=2 -C "." -- "${extracted_path}" 2>/dev/null || tar -xzf "${archive_path}" -C "." ) ;;
  esac

  # Tauri's externalBin convention: <name>-<target>{-suffix}.
  if [[ -f "${DEST_DIR}/extract-${target}/pandoc" ]]; then
    mv "${DEST_DIR}/extract-${target}/pandoc" "${final_path}"
  elif [[ -f "${DEST_DIR}/extract-${target}/pandoc.exe" ]]; then
    mv "${DEST_DIR}/extract-${target}/pandoc.exe" "${final_path}"
  fi

  chmod +x "${final_path}"
  rm -rf "${DEST_DIR}/${archive_path}" "${DEST_DIR}/extract-${target}"
  echo "  ✓ ${final_name}"
}

# ── EPUBCheck ─────────────────────────────────────────────────────
# Cross-platform: a single .jar file.  Tauri can't directly invoke
# .jar — we ship a small per-platform launcher script that exec's
# `java -jar epubcheck-5.1.0.jar`.
fetch_epubcheck() {
  local jar_path="${DEST_DIR}/epubcheck/epubcheck-${EPUBCHECK_VERSION}.jar"

  if [[ -f "${jar_path}" ]]; then
    echo "  → EPUBCheck ${EPUBCHECK_VERSION} already present; skipping"
    return 0
  fi

  local url="https://github.com/w3c/epubcheck/releases/download/v${EPUBCHECK_VERSION}/epubcheck-${EPUBCHECK_VERSION}.zip"
  local archive="${DEST_DIR}/epubcheck-${EPUBCHECK_VERSION}.zip"

  echo "  → Fetching ${url}"
  curl --location --fail --output "${archive}" "${url}"

  if [[ "${EPUBCHECK_SHA256}" != REPLACE_ME_* ]]; then
    local actual
    actual="$(shasum -a 256 "${archive}" | awk '{print $1}')"
    if [[ "${actual}" != "${EPUBCHECK_SHA256}" ]]; then
      echo "  ✗ SHA-256 mismatch for EPUBCheck"
      echo "    expected: ${EPUBCHECK_SHA256}"
      echo "    actual:   ${actual}"
      rm -f "${archive}"
      return 1
    fi
    echo "  ✓ SHA-256 verified"
  else
    echo "  ⚠  No pinned SHA-256 — populate EPUBCHECK_SHA256 in this script"
    echo "     Computed:"
    shasum -a 256 "${archive}"
  fi

  mkdir -p "${DEST_DIR}/epubcheck"
  ( cd "${DEST_DIR}/epubcheck" && unzip -q "../epubcheck-${EPUBCHECK_VERSION}.zip" )
  rm -f "${archive}"

  echo "  ✓ EPUBCheck ${EPUBCHECK_VERSION}"
}

# ── Generate per-platform EPUBCheck launcher ──────────────────────
generate_epubcheck_runner() {
  local target="$1"
  local runner_name="epubcheck-runner-${target}"
  local runner_path

  if [[ "${target}" == *windows* ]]; then
    runner_name="${runner_name}.exe"
    # On Windows we ship a tiny .bat-equivalent rust launcher in
    # production.  For now write a placeholder so the externalBin
    # entry resolves in dev.
    runner_path="${DEST_DIR}/${runner_name}"
    cat > "${runner_path}" <<'EOF'
@echo off
java -jar "%~dp0epubcheck\epubcheck-5.1.0.jar" %*
EOF
  else
    runner_path="${DEST_DIR}/${runner_name}"
    cat > "${runner_path}" <<EOF
#!/usr/bin/env bash
DIR="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")" && pwd)"
exec java -jar "\${DIR}/epubcheck/epubcheck-${EPUBCHECK_VERSION}.jar" "\$@"
EOF
    chmod +x "${runner_path}"
  fi
  echo "  ✓ ${runner_name}"
}

# ── Generate CHECKSUMS.txt ────────────────────────────────────────
write_checksums() {
  ( cd "${DEST_DIR}" && shasum -a 256 pandoc-* epubcheck/epubcheck-${EPUBCHECK_VERSION}.jar 2>/dev/null > CHECKSUMS.txt || true )
  echo "  ✓ CHECKSUMS.txt regenerated"
}

# ── Run ───────────────────────────────────────────────────────────
echo ""
echo "── Pandoc ${PANDOC_VERSION} ────────────────────────────────────"
for t in "${TARGETS[@]}"; do
  fetch_pandoc "${t}"
done

echo ""
echo "── EPUBCheck ${EPUBCHECK_VERSION} ──────────────────────────────"
fetch_epubcheck
for t in "${TARGETS[@]}"; do
  generate_epubcheck_runner "${t}"
done

echo ""
echo "── Checksums ─────────────────────────────────────────────────"
write_checksums

echo ""
echo "Done.  Tauri can now reference these via bundle.externalBin in"
echo "booksforge/apps/desktop/tauri.conf.json."
