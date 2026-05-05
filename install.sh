#!/usr/bin/env bash
# Install claude-usage binary + SessionEnd hook.
# Usage: curl -fsSL https://raw.githubusercontent.com/OWNER/REPO/main/install.sh | bash
set -euo pipefail

REPO="OWNER/REPO"   # <-- replace with your GitHub username/repo
BIN_NAME="claude-usage"
INSTALL_DIR="${HOME}/.local/bin"
CLAUDE_DIR="${HOME}/.claude"
RAW_BASE="https://raw.githubusercontent.com/${REPO}/main"
API_BASE="https://api.github.com/repos/${REPO}"

# ── Helpers ─────────────────────────────────────────────────────────────────

info()  { printf '\033[1;34m==> \033[0m%s\n' "$*"; }
ok()    { printf '\033[1;32m  ✓ \033[0m%s\n' "$*"; }
die()   { printf '\033[1;31mError: \033[0m%s\n' "$*" >&2; exit 1; }

need() { command -v "$1" &>/dev/null || die "'$1' is required but not found."; }

need curl
need python3

# ── Detect platform ──────────────────────────────────────────────────────────

OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
  Darwin) ARCHIVE="${BIN_NAME}-macos-universal.tar.gz" ;;
  Linux)
    [[ "$ARCH" == "x86_64" ]] || die "Unsupported architecture: $ARCH (only x86_64 is supported on Linux)"
    ARCHIVE="${BIN_NAME}-linux-x86_64.tar.gz"
    ;;
  *) die "Unsupported OS: $OS" ;;
esac

# ── Resolve latest release tag ───────────────────────────────────────────────

info "Fetching latest release..."
TAG=$(curl -fsSL "${API_BASE}/releases/latest" \
  | grep '"tag_name"' \
  | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
[[ -n "$TAG" ]] || die "Could not determine latest release tag."
ok "Latest release: ${TAG}"

# ── Download & install binary ────────────────────────────────────────────────

DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/${ARCHIVE}"
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

info "Downloading ${ARCHIVE}..."
curl -fsSL "$DOWNLOAD_URL" -o "${TMPDIR}/${ARCHIVE}"

info "Installing binary to ${INSTALL_DIR}..."
mkdir -p "$INSTALL_DIR"
tar xzf "${TMPDIR}/${ARCHIVE}" -C "$TMPDIR"
mv "${TMPDIR}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
chmod +x "${INSTALL_DIR}/${BIN_NAME}"
ok "Installed ${INSTALL_DIR}/${BIN_NAME}"

# ── Install hook script ──────────────────────────────────────────────────────

info "Installing SessionEnd hook..."
mkdir -p "$CLAUDE_DIR"
curl -fsSL "${RAW_BASE}/hook/collect-session-stats.py" \
  -o "${CLAUDE_DIR}/collect-session-stats.py"
ok "Saved hook to ${CLAUDE_DIR}/collect-session-stats.py"

# Register hook in settings.json
python3 - <<PYTHON
import json, sys
from pathlib import Path

CLAUDE_DIR = Path.home() / ".claude"
SETTINGS   = CLAUDE_DIR / "settings.json"
HOOK_CMD   = f"python3 {CLAUDE_DIR / 'collect-session-stats.py'}"
ENTRY      = {"matcher": "", "hooks": [{"type": "command", "command": HOOK_CMD}]}

CLAUDE_DIR.mkdir(exist_ok=True)
config = {}
if SETTINGS.exists():
    try:
        config = json.loads(SETTINGS.read_text())
    except json.JSONDecodeError:
        print(f"Warning: {SETTINGS} contains invalid JSON — skipping hook registration.", file=sys.stderr)
        sys.exit(0)

session_end = config.setdefault("hooks", {}).setdefault("SessionEnd", [])
already = any(
    "collect-session-stats.py" in (e.get("hooks") or [{}])[0].get("command", "")
    for e in session_end
)
if already:
    print("Hook already registered.")
else:
    session_end.append(ENTRY)
    SETTINGS.write_text(json.dumps(config, indent=2) + "\n")
    print(f"Registered SessionEnd hook in {SETTINGS}")
PYTHON

# ── PATH reminder ────────────────────────────────────────────────────────────

if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
  printf '\n\033[1;33mNote:\033[0m Add %s to your PATH:\n' "$INSTALL_DIR"
  printf '  echo '\''export PATH="%s:$PATH"'\'' >> ~/.bashrc\n' "$INSTALL_DIR"
  printf '  (or ~/.zshrc if you use zsh)\n\n'
fi

ok "Done! Run: ${BIN_NAME} --help"
