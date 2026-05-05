# claude-usage

A terminal dashboard for Claude Code session statistics, powered by a `SessionEnd` hook that writes a local JSONL log.

## What it shows

- **Bar chart** — daily aggregated cost (or token) usage scaled to max, with green bars and yellow values
- **Summary line** — session count, total cost, total tokens, and models seen
- **Session table** — date/time, git branch, duration, message and tool-call counts, tokens, cost, plus a bold totals row

![screenshot](screenshot.png)



## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) — `cargo` 1.65+
- Claude Code with a `SessionEnd` hook that appends session records to `~/.claude/usage-log.jsonl`

## Installation

```sh
make install
```

This builds a release binary and copies it to `~/.local/bin/claude-usage`. Make sure `~/.local/bin` is on your `PATH`.

## Usage

```sh
claude-usage                        # last 30 days, cost chart
claude-usage --days 7               # last 7 days
claude-usage --chart tokens         # token chart instead of cost
claude-usage --chart tokens --days 14
```

| Flag | Default | Description |
|------|---------|-------------|
| `-d`, `--days N` | `30` | How many days back to include |
| `--chart cost\|tokens` | `cost` | Metric shown in the bar chart |

## Development

```sh
make build    # debug build
make release  # optimized build (also used by install)
make run      # cargo run with default flags
make check    # fast compile check, no binary produced
make clean    # remove the target/ directory
```

## Log format

Each line in `~/.claude/usage-log.jsonl` is a JSON object written by the `SessionEnd` hook:

```json
{
  "session_id": "...",
  "session_start": "2026-05-01T14:00:00Z",
  "duration_ms": 3600000,
  "message_count": 42,
  "tool_call_count": 18,
  "git_branch": "main",
  "total_cost_usd": 0.2341,
  "models": {
    "claude-sonnet-4-6": {
      "input_tokens": 50000,
      "output_tokens": 8000,
      "cache_write_tokens": 12000,
      "cache_read_tokens": 30000,
      "cost_usd": 0.2341
    }
  },
  "unpriced_models": []
}
```
