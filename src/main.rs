use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use clap::Parser;
use colored::Colorize;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, CellAlignment, Color, ContentArrangement, Table};
use serde::Deserialize;

#[derive(Parser)]
#[command(
    name = "claude-usage",
    about = "View Claude Code session usage statistics",
    long_about = None
)]
struct Args {
    /// Number of days to look back
    #[arg(short, long, default_value = "30")]
    days: u64,

    /// Metric to display in the bar chart: cost or tokens
    #[arg(long, value_enum, default_value = "cost")]
    chart: ChartMetric,
}

#[derive(Clone, clap::ValueEnum, PartialEq)]
enum ChartMetric {
    Cost,
    Tokens,
}

#[derive(Deserialize, Debug)]
struct SessionRecord {
    #[allow(dead_code)]
    session_id: String,
    session_start: Option<String>,
    duration_ms: Option<i64>,
    message_count: u64,
    tool_call_count: u64,
    models: HashMap<String, ModelTokens>,
    total_cost_usd: f64,
    unpriced_models: Vec<String>,
    git_branch: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ModelTokens {
    input_tokens: u64,
    output_tokens: u64,
    cache_write_tokens: u64,
    cache_read_tokens: u64,
    #[allow(dead_code)]
    cost_usd: f64,
}

fn total_tokens(models: &HashMap<String, ModelTokens>) -> u64 {
    models
        .values()
        .map(|m| m.input_tokens + m.output_tokens + m.cache_write_tokens + m.cache_read_tokens)
        .sum()
}

fn fmt_duration(ms: Option<i64>) -> String {
    match ms {
        None | Some(0) => "—".to_string(),
        Some(ms) if ms < 0 => "—".to_string(),
        Some(ms) => {
            let total_secs = ms / 1000;
            let hours = total_secs / 3600;
            let mins = (total_secs % 3600) / 60;
            let secs = total_secs % 60;
            if hours > 0 {
                format!("{}h {}m", hours, mins)
            } else if mins > 0 {
                format!("{}m {}s", mins, secs)
            } else {
                format!("{}s", secs)
            }
        }
    }
}

fn fmt_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn print_bar_chart(daily: &BTreeMap<NaiveDate, f64>, title: &str, fmt_val: impl Fn(f64) -> String) {
    const BAR_MAX: usize = 46;

    let max_val = daily.values().cloned().fold(0.0_f64, f64::max);
    if max_val == 0.0 {
        return;
    }

    println!("\n  {}", title.bold().underline());
    println!();

    for (date, val) in daily {
        let filled = ((val / max_val) * BAR_MAX as f64).round().max(1.0) as usize;
        let bar = "█".repeat(filled);
        println!(
            "  {}  {}  {}",
            date.to_string().dimmed(),
            bar.bright_green(),
            fmt_val(*val).bright_yellow()
        );
    }

    println!();
}

fn parse_session_start(s: &str) -> Option<DateTime<Utc>> {
    // Handles both "Z" suffix and explicit "+00:00" offset
    let normalized = s.replace('Z', "+00:00");
    DateTime::parse_from_rfc3339(&normalized)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn main() {
    let args = Args::parse();

    let log_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
        .join("usage-log.jsonl");

    if !log_path.exists() {
        eprintln!(
            "{} No usage log found at {}.",
            "Error:".red().bold(),
            log_path.display()
        );
        eprintln!("Run a Claude Code session first — the SessionEnd hook writes data there.");
        std::process::exit(1);
    }

    let cutoff: DateTime<Utc> = Utc::now() - Duration::days(args.days as i64);

    let file = fs::File::open(&log_path).expect("Cannot open usage log");
    let reader = BufReader::new(file);

    let mut sessions: Vec<(DateTime<Utc>, SessionRecord)> = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let record: SessionRecord = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let start_dt = match record.session_start.as_deref().and_then(parse_session_start) {
            Some(dt) => dt,
            None => continue,
        };

        if start_dt >= cutoff {
            sessions.push((start_dt, record));
        }
    }

    sessions.sort_by_key(|(dt, _)| *dt);

    // ── Header ──────────────────────────────────────────────────────────────

    println!(
        "\n  {}",
        format!("Claude Code Usage — Last {} Days", args.days)
            .bold()
            .bright_cyan()
    );

    if sessions.is_empty() {
        println!(
            "\n  {}\n",
            format!("No sessions recorded in the last {} days.", args.days).dimmed()
        );
        return;
    }

    // ── Aggregate daily values for chart ────────────────────────────────────

    let mut daily_chart: BTreeMap<NaiveDate, f64> = BTreeMap::new();
    for (dt, rec) in &sessions {
        let date = dt.date_naive();
        let val = match args.chart {
            ChartMetric::Cost => rec.total_cost_usd,
            ChartMetric::Tokens => total_tokens(&rec.models) as f64,
        };
        *daily_chart.entry(date).or_insert(0.0) += val;
    }

    match args.chart {
        ChartMetric::Cost => {
            print_bar_chart(&daily_chart, "Daily Cost (USD)", |v| format!("${:.4}", v));
        }
        ChartMetric::Tokens => {
            print_bar_chart(&daily_chart, "Daily Tokens", |v| {
                fmt_tokens(v as u64)
            });
        }
    }

    // ── Summary line ────────────────────────────────────────────────────────

    let total_cost: f64 = sessions.iter().map(|(_, r)| r.total_cost_usd).sum();
    let total_tok: u64 = sessions.iter().map(|(_, r)| total_tokens(&r.models)).sum();
    let total_msgs: u64 = sessions.iter().map(|(_, r)| r.message_count).sum();
    let total_tools: u64 = sessions.iter().map(|(_, r)| r.tool_call_count).sum();

    let mut all_models: Vec<String> = sessions
        .iter()
        .flat_map(|(_, r)| r.models.keys().cloned())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    all_models.sort();

    let mut all_unpriced: Vec<String> = sessions
        .iter()
        .flat_map(|(_, r)| r.unpriced_models.iter().cloned())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    all_unpriced.sort();

    println!(
        "  {}  │  {}  │  {}  │  {}",
        format!("{} session{}", sessions.len(), if sessions.len() == 1 { "" } else { "s" }).bold(),
        format!("${:.4} total", total_cost)
            .bright_yellow()
            .bold(),
        format!("{} tokens", fmt_tokens(total_tok)),
        all_models.join(", ").dimmed()
    );

    if !all_unpriced.is_empty() {
        println!(
            "\n  {} Cost is underestimated — unrecognized model(s): {}",
            "⚠".yellow().bold(),
            all_unpriced.join(", ").yellow()
        );
    }

    // ── Session table ────────────────────────────────────────────────────────

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        Cell::new("Session").add_attribute(Attribute::Bold),
        Cell::new("Branch").add_attribute(Attribute::Bold),
        Cell::new("Duration").add_attribute(Attribute::Bold),
        Cell::new("Msgs / Tools").add_attribute(Attribute::Bold),
        Cell::new("Tokens").add_attribute(Attribute::Bold),
        Cell::new("Cost (USD)").add_attribute(Attribute::Bold),
    ]);

    for (dt, rec) in &sessions {
        let session_label = dt.format("%Y-%m-%d %H:%M").to_string();
        let branch = rec.git_branch.as_deref().unwrap_or("—").to_string();
        let duration = fmt_duration(rec.duration_ms);
        let msgs_tools = format!("{} / {}", rec.message_count, rec.tool_call_count);
        let tokens = fmt_tokens(total_tokens(&rec.models));
        let cost = format!("${:.4}", rec.total_cost_usd);

        table.add_row(vec![
            Cell::new(session_label),
            Cell::new(branch),
            Cell::new(duration),
            Cell::new(msgs_tools).set_alignment(CellAlignment::Center),
            Cell::new(tokens).set_alignment(CellAlignment::Right),
            Cell::new(cost).set_alignment(CellAlignment::Right),
        ]);
    }

    // Totals row
    table.add_row(vec![
        Cell::new("TOTAL")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new(""),
        Cell::new(""),
        Cell::new(format!("{} / {}", total_msgs, total_tools))
            .set_alignment(CellAlignment::Center)
            .add_attribute(Attribute::Bold),
        Cell::new(fmt_tokens(total_tok))
            .set_alignment(CellAlignment::Right)
            .add_attribute(Attribute::Bold),
        Cell::new(format!("${:.4}", total_cost))
            .set_alignment(CellAlignment::Right)
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
    ]);

    println!("\n{table}\n");
}
