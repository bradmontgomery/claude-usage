# CLAUDE.md
Guidance for Claude Code when working in this repository.

## Commands
```sh
make build        # debug build
make release      # optimized release build
make run          # cargo run (append -- <flags> for CLI args)
make check        # fast compile check without producing a binary
make install      # release build + copy to ~/.local/bin/claude-usage
make clean        # remove target/
```

Pass CLI flags via `cargo run -- --days 7 --chart tokens` or `make run -- --days 7`.

## Architecture

Single-file Rust CLI (`src/main.rs`). No modules — all logic lives in `main.rs`.

**Data flow:**
1. Reads `~/.claude/usage-log.jsonl` (written by a Claude Code `SessionEnd` hook)
2. Deserializes each line into `SessionRecord` / `ModelTokens` structs
3. Filters by `--days` cutoff, aggregates daily totals into a `BTreeMap<NaiveDate, f64>`
4. Prints: bar chart → summary line → `comfy-table` session table with a totals row

**Key types:**
- `SessionRecord` — one JSONL line; holds per-session cost, token counts, git branch, duration
- `ModelTokens` — per-model breakdown (input/output/cache_write/cache_read tokens + cost)
- `ChartMetric` — `cost` | `tokens` enum driving `--chart`

**Output formatting helpers:**
- `print_bar_chart` — scales bars to `BAR_MAX=46` columns, green bars, yellow values
- `fmt_tokens` — abbreviates to K/M
- `fmt_duration` — converts milliseconds to `Xh Xm` / `Xm Xs` / `Xs`

Dependencies: `clap` (arg parsing), `serde`/`serde_json` (deserialization), `chrono` (date math), `comfy-table` (session table), `colored` (terminal color), `dirs` (home dir).
