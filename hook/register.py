#!/usr/bin/env python3
"""
Register the collect-session-stats.py SessionEnd hook in ~/.claude/settings.json.
Safe to run multiple times — will not create duplicate entries.
"""

import json
import sys
from pathlib import Path

CLAUDE_DIR = Path.home() / ".claude"
SETTINGS = CLAUDE_DIR / "settings.json"
HOOK_SCRIPT = CLAUDE_DIR / "collect-session-stats.py"
HOOK_COMMAND = f"python3 {HOOK_SCRIPT}"

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

    # Idempotency check — skip if already registered
    already = any(
        "collect-session-stats.py" in e.get("hooks", [{}])[0].get("command", "")
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
