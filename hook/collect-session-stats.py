#!/usr/bin/env python3
"""
Collect Claude Code session stats and append to ~/.claude/usage-log.jsonl.
Triggered by SessionEnd hook. Receives JSON payload via stdin.

Pricing table last updated: 2026-05-04
https://www.anthropic.com/pricing
"""

import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

# Pricing per million tokens (USD) — update when Anthropic changes pricing
MODEL_PRICING = {
    "claude-opus-4-7": {
        "input": 15.00,
        "output": 75.00,
        "cache_write": 18.75,
        "cache_read": 1.50,
    },
    "claude-sonnet-4-6": {
        "input": 3.00,
        "output": 15.00,
        "cache_write": 3.75,
        "cache_read": 0.30,
    },
    "claude-haiku-4-5-20251001": {
        "input": 0.80,
        "output": 4.00,
        "cache_write": 1.00,
        "cache_read": 0.08,
    },
    "claude-haiku-4-5": {
        "input": 0.80,
        "output": 4.00,
        "cache_write": 1.00,
        "cache_read": 0.08,
    },
}

USAGE_LOG = Path.home() / ".claude" / "usage-log.jsonl"


def compute_cost(model: str, input_tokens: int, output_tokens: int,
                 cache_write_tokens: int, cache_read_tokens: int) -> tuple[float, bool]:
    """Returns (cost_usd, was_priced). was_priced=False means model not in table."""
    pricing = MODEL_PRICING.get(model)
    if not pricing:
        return 0.0, False
    cost = (
        input_tokens * pricing["input"] / 1_000_000
        + output_tokens * pricing["output"] / 1_000_000
        + cache_write_tokens * pricing["cache_write"] / 1_000_000
        + cache_read_tokens * pricing["cache_read"] / 1_000_000
    )
    return cost, True


def parse_transcript(transcript_path: str) -> dict:
    path = Path(transcript_path)
    if not path.exists():
        return {}

    tokens_by_model: dict[str, dict] = {}
    unpriced_models: set = set()
    timestamps: list[str] = []
    first_message = {}
    message_count = 0
    tool_call_count = 0

    with path.open() as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                entry = json.loads(line)
            except json.JSONDecodeError:
                continue

            ts = entry.get("timestamp")
            if ts:
                timestamps.append(ts)

            entry_type = entry.get("type")

            if entry_type == "user" and not first_message:
                first_message = entry

            if entry_type == "assistant":
                message_count += 1
                msg = entry.get("message", {})
                model = msg.get("model", "")

                # Skip synthetic/error messages (zero tokens, not real API calls)
                if model == "<synthetic>" or not model:
                    continue

                usage = msg.get("usage", {})
                input_tok = usage.get("input_tokens", 0)
                output_tok = usage.get("output_tokens", 0)
                cache_write = usage.get("cache_creation_input_tokens", 0)
                cache_read = usage.get("cache_read_input_tokens", 0)

                if model not in tokens_by_model:
                    tokens_by_model[model] = {
                        "input_tokens": 0,
                        "output_tokens": 0,
                        "cache_write_tokens": 0,
                        "cache_read_tokens": 0,
                    }

                tokens_by_model[model]["input_tokens"] += input_tok
                tokens_by_model[model]["output_tokens"] += output_tok
                tokens_by_model[model]["cache_write_tokens"] += cache_write
                tokens_by_model[model]["cache_read_tokens"] += cache_read

                # Count tool calls from content blocks
                for block in msg.get("content", []):
                    if isinstance(block, dict) and block.get("type") == "tool_use":
                        tool_call_count += 1

    # Compute cost per model
    total_cost = 0.0
    models_detail = {}
    for model, tok in tokens_by_model.items():
        cost, was_priced = compute_cost(
            model,
            tok["input_tokens"],
            tok["output_tokens"],
            tok["cache_write_tokens"],
            tok["cache_read_tokens"],
        )
        if not was_priced:
            unpriced_models.add(model)
        total_cost += cost
        models_detail[model] = {**tok, "cost_usd": round(cost, 6)}

    # Session duration
    duration_ms = None
    session_start = None
    session_end = None
    if len(timestamps) >= 2:
        try:
            t0 = datetime.fromisoformat(timestamps[0].replace("Z", "+00:00"))
            t1 = datetime.fromisoformat(timestamps[-1].replace("Z", "+00:00"))
            duration_ms = int((t1 - t0).total_seconds() * 1000)
            session_start = timestamps[0]
            session_end = timestamps[-1]
        except ValueError:
            pass
    elif timestamps:
        session_start = timestamps[0]
        session_end = timestamps[0]

    return {
        "session_start": session_start,
        "session_end": session_end,
        "duration_ms": duration_ms,
        "message_count": message_count,
        "tool_call_count": tool_call_count,
        "models": models_detail,
        "total_cost_usd": round(total_cost, 6),
        "unpriced_models": sorted(unpriced_models),
        "cwd": first_message.get("cwd"),
        "git_branch": first_message.get("gitBranch"),
        "version": first_message.get("version"),
    }


def main():
    try:
        payload = json.loads(sys.stdin.read())
    except (json.JSONDecodeError, OSError):
        sys.exit(0)

    session_id = payload.get("session_id", "")
    transcript_path = payload.get("transcript_path", "")

    if not transcript_path:
        sys.exit(0)

    stats = parse_transcript(transcript_path)
    if not stats.get("models"):
        sys.exit(0)

    record = {
        "session_id": session_id,
        "recorded_at": datetime.now(timezone.utc).isoformat(),
        **stats,
    }

    with USAGE_LOG.open("a") as f:
        f.write(json.dumps(record) + "\n")


if __name__ == "__main__":
    main()
