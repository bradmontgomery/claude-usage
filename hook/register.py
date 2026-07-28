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

    # Look for an entry that already points at *this exact* command, and
    # separately for stale ones (the pre-Rust python hook, or a binary at a
    # different path) that should be upgraded in place rather than duplicated.
    upgraded = []
    for entry in session_end:
        hook = (entry.get("hooks") or [{}])[0]
        command = hook.get("command", "")
        if command == HOOK_COMMAND:
            print("Hook already registered — nothing to do.")
            return
        if "collect-session-stats" in command:
            hook["command"] = HOOK_COMMAND
            upgraded.append(command)

    if upgraded:
        SETTINGS.write_text(json.dumps(config, indent=2) + "\n")
        for old in upgraded:
            print(f"Upgraded stale SessionEnd hook: {old} -> {HOOK_COMMAND}")
        return

    session_end.append(HOOK_ENTRY)
    SETTINGS.write_text(json.dumps(config, indent=2) + "\n")
    print(f"Registered SessionEnd hook in {SETTINGS}")


if __name__ == "__main__":
    main()
