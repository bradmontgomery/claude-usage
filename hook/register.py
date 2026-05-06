#!/usr/bin/env python3
"""
Register the collect-session-stats SessionEnd hook in ~/.claude/settings.json.
Safe to run multiple times — will not create duplicate entries.

Usage: python3 hook/register.py [/path/to/collect-session-stats]
Default path: ~/.local/bin/collect-session-stats
"""

import json
import sys
from pathlib import Path

CLAUDE_DIR = Path.home() / ".claude"
SETTINGS = CLAUDE_DIR / "settings.json"

hook_binary = sys.argv[1] if len(sys.argv) > 1 else str(Path.home() / ".local" / "bin" / "collect-session-stats")
HOOK_COMMAND = hook_binary

HOOK_ENTRY = {
    "matcher": "",
    "hooks": [{"type": "command", "command": HOOK_COMMAND}],
}


def main():
    CLAUDE_DIR.mkdir(exist_ok=True)

    if SETTINGS.exists():
        try:
            config = json.loads(SETTINGS.read_text())
        except json.JSONDecodeError:
            print(f"Error: {SETTINGS} contains invalid JSON — fix it manually.", file=sys.stderr)
            sys.exit(1)
    else:
        config = {}

    hooks = config.setdefault("hooks", {})
    session_end = hooks.setdefault("SessionEnd", [])

    # Matches both old python hook and new Rust binary
    already = any(
        "collect-session-stats" in e.get("hooks", [{}])[0].get("command", "")
        for e in session_end
        if e.get("hooks")
    )
    if already:
        print("Hook already registered — nothing to do.")
        return

    session_end.append(HOOK_ENTRY)
    SETTINGS.write_text(json.dumps(config, indent=2) + "\n")
    print(f"Registered SessionEnd hook in {SETTINGS}")


if __name__ == "__main__":
    main()
