use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// (input, output, cache_write, cache_read) per million tokens (USD)
// Last updated: 2026-06-13  https://www.anthropic.com/pricing
fn model_price(model: &str) -> Option<(f64, f64, f64, f64)> {
    match model {
        "claude-fable-5" => Some((10.00, 50.00, 12.50, 1.00)),
        "claude-opus-4-8" => Some((5.00, 25.00, 6.25, 0.50)),
        "claude-opus-4-7" => Some((15.00, 75.00, 18.75, 1.50)),
        "claude-sonnet-4-6" => Some((3.00, 15.00, 3.75, 0.30)),
        "claude-haiku-4-5-20251001" | "claude-haiku-4-5" => Some((0.80, 4.00, 1.00, 0.08)),
        _ if model.starts_with("claude-fable-5") => Some((10.00, 50.00, 12.50, 1.00)),
        _ if model.starts_with("claude-opus-4-8") => Some((5.00, 25.00, 6.25, 0.50)),
        _ if model.starts_with("claude-opus-4-7") => Some((15.00, 75.00, 18.75, 1.50)),
        _ if model.starts_with("claude-sonnet-4-6") => Some((3.00, 15.00, 3.75, 0.30)),
        _ if model.starts_with("claude-haiku-4-5") => Some((0.80, 4.00, 1.00, 0.08)),
        _ => None,
    }
}

#[derive(Deserialize)]
struct HookPayload {
    session_id: Option<String>,
    transcript_path: Option<String>,
}

#[derive(Deserialize)]
struct TranscriptEntry {
    #[serde(rename = "type")]
    entry_type: Option<String>,
    timestamp: Option<String>,
    message: Option<AssistantMessage>,
    cwd: Option<String>,
    #[serde(rename = "gitBranch")]
    git_branch: Option<String>,
    version: Option<String>,
}

#[derive(Deserialize)]
struct AssistantMessage {
    model: Option<String>,
    usage: Option<TokenUsage>,
    content: Option<Vec<Value>>,
}

#[derive(Deserialize, Default)]
struct TokenUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
}

#[derive(Serialize, Default)]
struct ModelTokens {
    input_tokens: u64,
    output_tokens: u64,
    cache_write_tokens: u64,
    cache_read_tokens: u64,
    cost_usd: f64,
}

#[derive(Serialize)]
struct SessionRecord {
    session_id: String,
    recorded_at: String,
    session_start: Option<String>,
    session_end: Option<String>,
    duration_ms: Option<i64>,
    message_count: u64,
    tool_call_count: u64,
    models: HashMap<String, ModelTokens>,
    total_cost_usd: f64,
    unpriced_models: Vec<String>,
    cwd: Option<String>,
    git_branch: Option<String>,
    version: Option<String>,
}

fn round6(v: f64) -> f64 {
    (v * 1_000_000.0).round() / 1_000_000.0
}

fn parse_ts(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&s.replace('Z', "+00:00"))
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return;
    }

    let payload: HookPayload = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(_) => return,
    };

    let session_id = payload.session_id.unwrap_or_default();
    let transcript_path = match payload.transcript_path.as_deref() {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => return,
    };

    if !transcript_path.exists() {
        return;
    }

    let file = match File::open(&transcript_path) {
        Ok(f) => f,
        Err(_) => return,
    };

    let mut tokens_by_model: HashMap<String, ModelTokens> = HashMap::new();
    let mut unpriced: HashSet<String> = HashSet::new();
    let mut timestamps: Vec<String> = Vec::new();
    let mut first_user: Option<TranscriptEntry> = None;
    let mut message_count = 0u64;
    let mut tool_call_count = 0u64;

    for line in BufReader::new(file).lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        let entry: TranscriptEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue,
        };

        if let Some(ts) = &entry.timestamp {
            timestamps.push(ts.clone());
        }

        match entry.entry_type.as_deref() {
            Some("user") => {
                if first_user.is_none() {
                    first_user = Some(entry);
                }
            }
            Some("assistant") => {
                message_count += 1;
                if let Some(msg) = &entry.message {
                    let model = msg.model.as_deref().unwrap_or("");
                    if model == "<synthetic>" || model.is_empty() {
                        continue;
                    }

                    let usage = msg.usage.as_ref();
                    let input = usage.and_then(|u| u.input_tokens).unwrap_or(0);
                    let output = usage.and_then(|u| u.output_tokens).unwrap_or(0);
                    let cache_write = usage.and_then(|u| u.cache_creation_input_tokens).unwrap_or(0);
                    let cache_read = usage.and_then(|u| u.cache_read_input_tokens).unwrap_or(0);

                    let tok = tokens_by_model.entry(model.to_string()).or_default();
                    tok.input_tokens += input;
                    tok.output_tokens += output;
                    tok.cache_write_tokens += cache_write;
                    tok.cache_read_tokens += cache_read;

                    if let Some(content) = &msg.content {
                        for block in content {
                            if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                                tool_call_count += 1;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if tokens_by_model.is_empty() {
        return;
    }

    let mut total_cost = 0.0f64;
    for (model, tok) in &mut tokens_by_model {
        let cost = match model_price(model) {
            Some((ip, op, cwp, crp)) => {
                tok.input_tokens as f64 * ip / 1_000_000.0
                    + tok.output_tokens as f64 * op / 1_000_000.0
                    + tok.cache_write_tokens as f64 * cwp / 1_000_000.0
                    + tok.cache_read_tokens as f64 * crp / 1_000_000.0
            }
            None => {
                unpriced.insert(model.clone());
                0.0
            }
        };
        tok.cost_usd = round6(cost);
        total_cost += cost;
    }

    let (session_start, session_end, duration_ms) = if timestamps.len() >= 2 {
        let t0 = timestamps.first().unwrap();
        let t1 = timestamps.last().unwrap();
        let dur = match (parse_ts(t0), parse_ts(t1)) {
            (Some(dt0), Some(dt1)) => Some((dt1 - dt0).num_milliseconds()),
            _ => None,
        };
        (Some(t0.clone()), Some(t1.clone()), dur)
    } else {
        let ts = timestamps.into_iter().next();
        (ts.clone(), ts, None)
    };

    let (cwd, git_branch, version) = match first_user {
        Some(u) => (u.cwd, u.git_branch, u.version),
        None => (None, None, None),
    };

    let mut unpriced_models: Vec<String> = unpriced.into_iter().collect();
    unpriced_models.sort();

    let record = SessionRecord {
        session_id,
        recorded_at: Utc::now().to_rfc3339(),
        session_start,
        session_end,
        duration_ms,
        message_count,
        tool_call_count,
        models: tokens_by_model,
        total_cost_usd: round6(total_cost),
        unpriced_models,
        cwd,
        git_branch,
        version,
    };

    let log_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
        .join("usage-log.jsonl");

    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&log_path) {
        if let Ok(json) = serde_json::to_string(&record) {
            let _ = writeln!(f, "{}", json);
        }
    }
}
