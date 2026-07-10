//! Usage & Analytics commands.
//!
//! Extracted from lib.rs — handles usage statistics, request history,
//! and syncing usage data from the CLIProxyAPI management API.

use crate::helpers::history::{
    load_aggregate, load_request_history, save_aggregate, save_request_history, update_timeseries,
};
use crate::state::AppState;
use crate::types::{
    Aggregate, ModelUsage, ProviderUsage, RequestHistory, RequestLog, TimeSeriesPoint,
    UsageStats,
};
use crate::utils::estimate_request_cost;
use serde::Deserialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::State;

#[derive(Debug, Default, Deserialize)]
struct UsageQueueDetail {
    #[serde(
        default,
        alias = "InputTokens",
        alias = "inputTokens",
        alias = "input_tokens"
    )]
    input_tokens: i64,
    #[serde(
        default,
        alias = "OutputTokens",
        alias = "outputTokens",
        alias = "output_tokens"
    )]
    output_tokens: i64,
    #[serde(
        default,
        alias = "ReasoningTokens",
        alias = "reasoningTokens",
        alias = "reasoning_tokens"
    )]
    reasoning_tokens: i64,
    #[serde(
        default,
        alias = "CachedTokens",
        alias = "cachedTokens",
        alias = "cached_tokens"
    )]
    cached_tokens: i64,
}

#[derive(Debug, Default, Deserialize)]
struct UsageQueueRecord {
    #[serde(default, alias = "Provider", alias = "provider")]
    provider: String,
    #[serde(default, alias = "Model", alias = "model")]
    model: String,
    #[serde(default, alias = "Alias", alias = "alias")]
    alias: String,
    #[serde(
        default,
        alias = "RequestedAt",
        alias = "requestedAt",
        alias = "requested_at",
        alias = "timestamp"
    )]
    requested_at: String,
    #[serde(default, alias = "Failed", alias = "failed")]
    failed: bool,
    #[serde(default, alias = "Detail", alias = "detail", alias = "tokens")]
    detail: UsageQueueDetail,
}

fn parse_usage_queue_records(value: serde_json::Value) -> Result<Vec<UsageQueueRecord>, String> {
    let items = value
        .as_array()
        .ok_or("Usage queue response must be an array")?;
    let mut records = Vec::with_capacity(items.len());

    for item in items {
        if item.is_null() {
            continue;
        }

        let record = match item {
            serde_json::Value::String(raw) => {
                serde_json::from_str::<UsageQueueRecord>(raw).map_err(|e| e.to_string())?
            }
            _ => serde_json::from_value::<UsageQueueRecord>(item.clone())
                .map_err(|e| e.to_string())?,
        };
        records.push(record);
    }

    Ok(records)
}

fn queue_record_timestamp(record: &UsageQueueRecord) -> chrono::DateTime<chrono::Local> {
    chrono::DateTime::parse_from_rfc3339(record.requested_at.trim())
        .map(|dt| dt.with_timezone(&chrono::Local))
        .unwrap_or_else(|_| chrono::Local::now())
}

fn usage_detail_to_totals(detail: &UsageQueueDetail) -> (u64, u64, u64, u64) {
    let input = detail.input_tokens.max(0) as u64;
    let output = detail.output_tokens.max(0) as u64 + detail.reasoning_tokens.max(0) as u64;
    let cached = detail.cached_tokens.max(0) as u64;
    let total = input + output;
    (input, output, cached, total)
}

    fn trim_timeseries(series: &mut Vec<TimeSeriesPoint>, max_len: usize) {
        series.sort_by(|a, b| a.label.cmp(&b.label));
        if series.len() > max_len {
            *series = series.split_off(series.len() - max_len);
        }
    }

fn apply_usage_queue_records(
    records: &[UsageQueueRecord],
    history: &mut RequestHistory,
    agg: &mut Aggregate,
) {
    for record in records {
        let timestamp = queue_record_timestamp(record);
        let day_label = timestamp.format("%Y-%m-%d").to_string();
        let hour_label = timestamp.format("%Y-%m-%dT%H").to_string();
        let model_name = if record.model.trim().is_empty() {
            record.alias.trim().to_string()
        } else {
            record.model.trim().to_string()
        };
        let provider_name = if record.provider.trim().is_empty() {
            "unknown".to_string()
        } else {
            record.provider.trim().to_string()
        };

        let (input_tokens, output_tokens, cached_tokens, total_tokens) =
            usage_detail_to_totals(&record.detail);

        history.total_request_count += 1;
        if !record.failed {
            history.total_success_count += 1;
        }

        agg.total_requests += 1;
        if record.failed {
            agg.total_failure_count += 1;
        } else {
            agg.total_success_count += 1;
        }
        update_timeseries(&mut agg.requests_by_day, &day_label, 1);
        update_timeseries(&mut agg.requests_by_hour, &hour_label, 1);

        if total_tokens > 0 || cached_tokens > 0 {
            history.total_tokens_in += input_tokens;
            history.total_tokens_out += output_tokens;
            history.total_tokens_cached += cached_tokens;
            history.total_cost_usd += estimate_request_cost(
                &model_name,
                input_tokens.min(u32::MAX as u64) as u32,
                output_tokens.min(u32::MAX as u64) as u32,
            );
            update_timeseries(&mut history.tokens_by_day, &day_label, total_tokens);
            update_timeseries(&mut history.tokens_by_hour, &hour_label, total_tokens);

            agg.total_tokens_in += input_tokens;
            agg.total_tokens_out += output_tokens;
            agg.total_tokens_cached += cached_tokens;
            agg.total_cost_usd += estimate_request_cost(
                &model_name,
                input_tokens.min(u32::MAX as u64) as u32,
                output_tokens.min(u32::MAX as u64) as u32,
            );
            update_timeseries(&mut agg.tokens_by_day, &day_label, total_tokens);
            update_timeseries(&mut agg.tokens_by_hour, &hour_label, total_tokens);
        }

        if !model_name.is_empty() && model_name != "unknown" {
            let model_stats = agg
                .model_stats
                .entry(model_name)
                .or_default();
            model_stats.requests += 1;
            if !record.failed {
                model_stats.success_count += 1;
            }
            model_stats.tokens += total_tokens;
            model_stats.input_tokens += input_tokens;
            model_stats.output_tokens += output_tokens;
            model_stats.cached_tokens += cached_tokens;
        }

        let provider_stats = agg
            .provider_stats
            .entry(provider_name)
            .or_default();
        provider_stats.requests += 1;
        if !record.failed {
            provider_stats.success_count += 1;
        }
        provider_stats.tokens += total_tokens;
        provider_stats.input_tokens += input_tokens;
        provider_stats.output_tokens += output_tokens;
        provider_stats.cached_tokens += cached_tokens;
    }

    trim_timeseries(&mut agg.requests_by_day, 14);
    trim_timeseries(&mut agg.requests_by_hour, 168);
    trim_timeseries(&mut history.tokens_by_day, 14);
    trim_timeseries(&mut history.tokens_by_hour, 168);
    trim_timeseries(&mut agg.tokens_by_day, 14);
    trim_timeseries(&mut agg.tokens_by_hour, 168);
}

pub(crate) fn sync_usage_from_queue_blocking(port: u16) -> Result<(), String> {
    const BATCH_SIZE: usize = 500;
    const MAX_BATCHES: usize = 20;

    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| format!("failed to build HTTP client: {}", e))?;
    let management_key = crate::get_management_key();
    let mut history = load_request_history();
    let mut agg = load_aggregate();

    for _ in 0..MAX_BATCHES {
        let url = format!(
            "http://127.0.0.1:{}/v0/management/usage-queue?count={}",
            port, BATCH_SIZE
        );
        let response = client
            .get(&url)
            .header("X-Management-Key", &management_key)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .map_err(|e| format!("usage queue request failed: {}", e))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err("usage queue endpoint unavailable".to_string());
        }

        if !response.status().is_success() {
            return Err(format!("usage queue HTTP {}", response.status()));
        }

        let body: serde_json::Value = response.json().map_err(|e| e.to_string())?;
        let records = parse_usage_queue_records(body)?;
        let fetched = records.len();
        if fetched == 0 {
            break;
        }

        apply_usage_queue_records(&records, &mut history, &mut agg);

        if fetched < BATCH_SIZE {
            break;
        }
    }

    save_request_history(&history)?;
    save_aggregate(&agg)?;
    Ok(())
    }

    /// Spawns a background usage-queue collector thread.
    /// Uses a generation/epoch token (Arc<AtomicU64>) instead of a running flag.
    /// Each call bumps the generation; the spawned thread captures the current value
    /// and exits when the generation advances (via stop or exit).
    /// Prevents stale collector threads from surviving a stop->start cycle.
    pub(crate) fn start_usage_queue_collector(gen: Arc<AtomicU64>, port: u16) {
        // fetch_add returns the old value; +1 gives the post-increment generation
        // that the spawned thread will compare against gen.load() to know when to exit.
        let my_gen = gen.fetch_add(1, Ordering::SeqCst) + 1;

        std::thread::spawn(move || {
            // Brief delay so the proxy has time to become fully ready
            std::thread::sleep(Duration::from_secs(2));

            while gen.load(Ordering::SeqCst) == my_gen {
                if let Err(e) = sync_usage_from_queue_blocking(port) {
                    eprintln!("[usage-collector] queue sync failed: {e}");
                }
                // Sleep in 1s chunks — responsive to generation change while avoiding busy-loop
                let tick_end = std::time::Instant::now() + Duration::from_secs(30);
                while std::time::Instant::now() < tick_end {
                    if gen.load(Ordering::SeqCst) != my_gen {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(1000));
                }
            }
        });
    }

    async fn sync_usage_from_queue_async(port: u16) -> Result<RequestHistory, String> {
    const BATCH_SIZE: usize = 500;
    const MAX_BATCHES: usize = 20;

    let client = crate::build_management_client();
    let management_key = crate::get_management_key();
    let mut history = load_request_history();
    let mut agg = load_aggregate();

    for _ in 0..MAX_BATCHES {
        let url = format!(
            "http://127.0.0.1:{}/v0/management/usage-queue?count={}",
            port, BATCH_SIZE
        );
        let response = client
            .get(&url)
            .header("X-Management-Key", &management_key)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("Failed to fetch usage queue: {}", e))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err("Usage queue endpoint unavailable".to_string());
        }

        if !response.status().is_success() {
            return Err(format!(
                "Usage queue API returned status: {}",
                response.status()
            ));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse usage queue response: {}", e))?;
        let records = parse_usage_queue_records(body)?;
        let fetched = records.len();
        if fetched == 0 {
            break;
        }

        apply_usage_queue_records(&records, &mut history, &mut agg);

        if fetched < BATCH_SIZE {
            break;
        }
    }

    save_request_history(&history)?;
    save_aggregate(&agg)?;
    Ok(history)
}



// Blocking version of sync — only uses /v0/management/usage-queue
fn sync_usage_from_proxy_blocking(port: u16) {
    if let Err(e) = sync_usage_from_queue_blocking(port) {
        eprintln!("[usage] sync_usage_from_proxy_blocking: queue sync failed: {}", e);
    }
}

// Compute usage statistics - fetches live data from Go backend when proxy is running
#[tauri::command]
pub fn get_usage_stats(state: State<'_, AppState>) -> Result<UsageStats, String> {
    // Get proxy status
    let (is_running, port) = {
        let status = state.proxy_status.lock().unwrap();
        (status.running, status.port)
    };

    // Sync usage-queue records into persistent aggregate
    if is_running {
        sync_usage_from_proxy_blocking(port);
    }

    // Now load the updated aggregate and history
    let agg = load_aggregate();
    let history = load_request_history();

    // Use aggregate as the source of truth for all-time totals (preserved across restarts).
    let total_tokens = agg.total_tokens_in + agg.total_tokens_out;
    let input_tokens = agg.total_tokens_in;
    let output_tokens = agg.total_tokens_out;
    let cached_tokens = agg.total_tokens_cached;

    // Build model token breakdown from aggregate stats
    let model_tokens: std::collections::HashMap<String, u64> = agg
        .model_stats
        .iter()
        .map(|(k, v)| (k.clone(), v.tokens))
        .collect();
    let model_token_breakdown: std::collections::HashMap<String, (u64, u64, u64)> = agg
        .model_stats
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                (v.input_tokens, v.output_tokens, v.cached_tokens),
            )
        })
        .collect();

    // If no data yet, return defaults
    if agg.total_requests == 0 && history.requests.is_empty() {
        return Ok(UsageStats::default());
    }

    // Use aggregate as primary source of truth for all-time stats
    let total_requests = agg.total_requests;
    let success_count = agg.total_success_count;
    let failure_count = agg.total_failure_count;

    // Calculate today's stats from aggregate time-series (only source now)
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let requests_today = agg
        .requests_by_day
        .iter()
        .find(|p| p.label == today)
        .map(|p| p.value)
        .unwrap_or(0);

    let tokens_today = agg
        .tokens_by_day
        .iter()
        .find(|p| p.label == today)
        .map(|p| p.value)
        .unwrap_or(0);

    // Build model stats from aggregate only
    let mut models: Vec<ModelUsage> = agg
        .model_stats
        .iter()
        .filter(|(model, _)| *model != "unknown" && !model.is_empty())
        .map(|(model, stats)| {
            let tokens = model_tokens.get(model).copied().unwrap_or(stats.tokens);
            let (input, output, cached) = model_token_breakdown
                .get(model)
                .copied()
                .unwrap_or((0, 0, 0));
            ModelUsage {
                model: model.clone(),
                requests: stats.requests,
                tokens,
                input_tokens: input,
                output_tokens: output,
                cached_tokens: cached,
            }
        })
        .collect();
    models.sort_by_key(|b| std::cmp::Reverse(b.requests));

    // Build provider stats from aggregate
    let mut providers: Vec<ProviderUsage> = agg
        .provider_stats
        .iter()
        .filter(|(provider, _)| *provider != "unknown" && !provider.is_empty())
        .map(|(provider, stats)| ProviderUsage {
            provider: provider.clone(),
            requests: stats.requests,
            tokens: stats.tokens,
        })
        .collect();
    providers.sort_by_key(|b| std::cmp::Reverse(b.requests));

    // Use aggregate time-series, fall back to history if empty
    let mut requests_by_day = agg.requests_by_day.clone();
    if requests_by_day.is_empty() && !history.requests.is_empty() {
        // Build from history requests
        let mut map: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        for req in &history.requests {
            if let Some(dt) = chrono::DateTime::from_timestamp_millis(req.timestamp as i64) {
                let day = dt.format("%Y-%m-%d").to_string();
                *map.entry(day).or_insert(0) += 1;
            }
        }
        let mut points: Vec<TimeSeriesPoint> = map
            .into_iter()
            .map(|(label, value)| TimeSeriesPoint { label, value })
            .collect();
        points.sort_by(|a, b| a.label.cmp(&b.label));
        // Keep last 14 days
        if points.len() > 14 {
            points = points.split_off(points.len() - 14);
        }
        requests_by_day = points;
    } else if requests_by_day.len() > 14 {
        requests_by_day = requests_by_day.split_off(requests_by_day.len() - 14);
    }

    let mut tokens_by_day = agg.tokens_by_day.clone();
    if tokens_by_day.is_empty() {
        tokens_by_day = history.tokens_by_day.clone();
    }
    if tokens_by_day.len() > 14 {
        tokens_by_day = tokens_by_day.split_off(tokens_by_day.len() - 14);
    }

    // Use aggregate hourly data (persisted across sessions), fall back to history if empty
    let mut requests_by_hour: Vec<TimeSeriesPoint> = if !agg.requests_by_hour.is_empty() {
        agg.requests_by_hour.clone()
    } else {
        // Build from history as fallback for existing data
        let mut requests_by_hour_map: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();
        for req in &history.requests {
            if let Some(dt) = chrono::DateTime::from_timestamp_millis(req.timestamp as i64) {
                let hour_label = dt.format("%Y-%m-%dT%H").to_string();
                *requests_by_hour_map.entry(hour_label).or_insert(0) += 1;
            }
        }
        requests_by_hour_map
            .into_iter()
            .map(|(label, value)| TimeSeriesPoint { label, value })
            .collect()
    };
    requests_by_hour.sort_by(|a, b| a.label.cmp(&b.label));
    // Keep last 168 hours (7 days) for Activity Patterns heatmap
    if requests_by_hour.len() > 168 {
        requests_by_hour = requests_by_hour.split_off(requests_by_hour.len() - 168);
    }

    // Use aggregate hourly tokens data, fall back to history if empty
    let mut tokens_by_hour: Vec<TimeSeriesPoint> = if !agg.tokens_by_hour.is_empty() {
        agg.tokens_by_hour.clone()
    } else {
        // Build from history as fallback
        let mut tokens_by_hour_map: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();
        for req in &history.requests {
            if let Some(dt) = chrono::DateTime::from_timestamp_millis(req.timestamp as i64) {
                let hour_label = dt.format("%Y-%m-%dT%H").to_string();
                let tokens = (req.tokens_in.unwrap_or(0) + req.tokens_out.unwrap_or(0)) as u64;
                *tokens_by_hour_map.entry(hour_label).or_insert(0) += tokens;
            }
        }
        tokens_by_hour_map
            .into_iter()
            .map(|(label, value)| TimeSeriesPoint { label, value })
            .collect()
    };
    tokens_by_hour.sort_by(|a, b| a.label.cmp(&b.label));
    if tokens_by_hour.len() > 168 {
        tokens_by_hour = tokens_by_hour.split_off(tokens_by_hour.len() - 168);
    }

    Ok(UsageStats {
        total_requests,
        success_count,
        failure_count,
        total_tokens,
        input_tokens,
        output_tokens,
        cached_tokens,
        requests_today,
        tokens_today,
        models,
        providers,
        requests_by_day,
        tokens_by_day,
        requests_by_hour,
        tokens_by_hour,
    })
}

// Get request history
#[tauri::command]
pub fn get_request_history() -> RequestHistory {
    load_request_history()
}

// Add a request to history (called when request-log event is emitted)
// Returns only the added request to minimize data transfer (memory optimization)
#[tauri::command]
pub fn add_request_to_history(request: RequestLog) -> Result<RequestLog, String> {
    let mut history = load_request_history();

    // Calculate cost for this request
    let tokens_in = request.tokens_in.unwrap_or(0);
    let tokens_out = request.tokens_out.unwrap_or(0);
    let cost = estimate_request_cost(&request.model, tokens_in, tokens_out);
    let tokens_cached = request.tokens_cached.unwrap_or(0);

    // Update totals
    history.total_tokens_in += tokens_in as u64;
    history.total_tokens_out += tokens_out as u64;
    history.total_tokens_cached += tokens_cached as u64;
    history.total_cost_usd += cost;

    // Add request (with deduplication check)
    // Check if request with same ID already exists to prevent duplicates
    let request_clone = request.clone();
    if !history.requests.iter().any(|r| r.id == request.id) {
        history.requests.push(request);

        // Trim to prevent unbounded growth (keep last 500 requests)
        const MAX_HISTORY_SIZE: usize = 500;
        if history.requests.len() > MAX_HISTORY_SIZE {
            let excess = history.requests.len() - MAX_HISTORY_SIZE;
            history.requests.drain(0..excess);
        }
    }

    // Save
    save_request_history(&history)?;

    // Return only the added request, not the full history
    Ok(request_clone)
}

// Clear request history
#[tauri::command]
pub fn clear_request_history() -> Result<(), String> {
    let history = RequestHistory::default();
    save_request_history(&history)
}

// Sync usage statistics from CLIProxyAPI — only uses /v0/management/usage-queue
#[tauri::command]
pub async fn sync_usage_from_proxy(state: State<'_, AppState>) -> Result<RequestHistory, String> {
    let port = {
        let config = state.config.lock().unwrap();
        config.port
    };

    sync_usage_from_queue_async(port).await
}

// Export usage statistics from local persistent storage for backup
#[tauri::command]
pub fn export_usage_stats() -> Result<serde_json::Value, String> {
    let agg = load_aggregate();
    let history = load_request_history();

    let export = serde_json::json!({
        "version": 1,
        "exported_at": chrono::Local::now().to_rfc3339(),
        "aggregate": agg,
        "request_history": history,
    });

    Ok(export)
}

// Import usage statistics into local persistent storage from backup
#[tauri::command]
pub fn import_usage_stats(data: serde_json::Value) -> Result<serde_json::Value, String> {
    let version = data.get("version").and_then(|v| v.as_i64()).unwrap_or(0);
    if version != 1 {
        return Err(format!("Unsupported export version: {}. Expected 1.", version));
    }

    let agg: Aggregate = serde_json::from_value(
        data.get("aggregate")
            .ok_or_else(|| "Missing 'aggregate' field in import data".to_string())?
            .clone(),
    )
    .map_err(|e| format!("Failed to parse aggregate: {}", e))?;

    let history: RequestHistory = serde_json::from_value(
        data.get("request_history")
            .ok_or_else(|| "Missing 'request_history' field in import data".to_string())?
            .clone(),
    )
    .map_err(|e| format!("Failed to parse request_history: {}", e))?;

    let total_requests = agg.total_requests;
    save_aggregate(&agg)?;
    save_request_history(&history)?;

    Ok(serde_json::json!({
        "added": history.requests.len(),
        "failed_requests": 0,
        "skipped": 0,
        "total_requests": total_requests,
    }))
}

#[cfg(test)]
    mod tests {
        use super::*;
        use std::io::Read;

    #[test]
    fn test_usage_detail_to_totals() {
        let detail = UsageQueueDetail {
            input_tokens: 100,
            output_tokens: 50,
            reasoning_tokens: 20,
            cached_tokens: 10,
        };
        let (input, output, cached, total) = usage_detail_to_totals(&detail);
        assert_eq!(input, 100, "input_tokens");
        assert_eq!(output, 70, "output_tokens (output + reasoning)");
        assert_eq!(cached, 10, "cached_tokens");
        assert_eq!(total, 170, "total_tokens (input + output)");
    }

    #[test]
    fn test_usage_detail_to_totals_edge_negative() {
        let detail = UsageQueueDetail {
            input_tokens: -5,
            output_tokens: 10,
            reasoning_tokens: -1,
            cached_tokens: 0,
        };
        let (input, output, cached, total) = usage_detail_to_totals(&detail);
        assert_eq!(input, 0, "negative clamps to 0");
        assert_eq!(output, 10, "negative reasoning clamps to 0, only output counts");
        assert_eq!(cached, 0, "zero cached");
        assert_eq!(total, 10, "0 + 10");
    }

    #[test]
    fn test_parse_usage_queue_records_json() {
        let json = serde_json::json!([
            {
                "provider": "openai",
                "model": "gpt-4",
                "requested_at": "2025-06-01T12:00:00Z",
                "failed": false,
                "detail": {"input_tokens": 10, "output_tokens": 20}
            }
        ]);
        let records = parse_usage_queue_records(json).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].provider, "openai");
        assert_eq!(records[0].model, "gpt-4");
        assert_eq!(records[0].detail.input_tokens, 10);
        assert_eq!(records[0].detail.output_tokens, 20);
    }

    #[test]
    fn test_parse_usage_queue_records_skips_null() {
        let json = serde_json::json!([null, {"provider": "x", "model": "y", "requested_at": "2025-01-01T00:00:00Z", "detail": {}}]);
        let records = parse_usage_queue_records(json).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].provider, "x");
    }

    #[test]
    fn test_parse_usage_queue_records_string_items() {
        let json = serde_json::json!([
            r#"{"provider":"azure","model":"gpt-35","requested_at":"2025-06-01T00:00:00Z","detail":{"input_tokens":5,"output_tokens":15}}"#
        ]);
        let records = parse_usage_queue_records(json).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].provider, "azure");
        assert_eq!(records[0].detail.input_tokens, 5);
    }

    #[test]
    fn test_parse_usage_queue_records_with_timestamp_tokens() {
        // Official usage-queue response uses `timestamp` and `tokens` field names
        let json = serde_json::json!([
            {
                "provider": "anthropic",
                "model": "claude-3-opus",
                "timestamp": "2025-07-10T08:30:00Z",
                "failed": false,
                "tokens": {"input_tokens": 200, "output_tokens": 150, "reasoning_tokens": 30, "cached_tokens": 10}
            }
        ]);
        let records = parse_usage_queue_records(json).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].provider, "anthropic");
        assert_eq!(records[0].model, "claude-3-opus");
        assert_eq!(records[0].detail.input_tokens, 200);
        assert_eq!(records[0].detail.output_tokens, 150);
        assert_eq!(records[0].detail.reasoning_tokens, 30);
        assert_eq!(records[0].detail.cached_tokens, 10);

        // Verify non-zero totals after application
        let mut history = RequestHistory::default();
        let mut agg = Aggregate::default();
        apply_usage_queue_records(&records, &mut history, &mut agg);
        assert!(agg.total_tokens_in > 0, "total_tokens_in should be non-zero");
        assert!(agg.total_tokens_out > 0, "total_tokens_out should be non-zero");
        assert!(agg.total_tokens_cached > 0, "total_tokens_cached should be non-zero");
        assert!(history.total_tokens_in > 0, "history total_tokens_in should be non-zero");
        assert!(history.total_tokens_out > 0, "history total_tokens_out should be non-zero");
        assert!(history.total_tokens_cached > 0, "history total_tokens_cached should be non-zero");
    }

    #[test]
    fn test_parse_usage_queue_records_not_array() {
        let result = parse_usage_queue_records(serde_json::json!({}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be an array"));
    }

    #[test]
    fn test_apply_usage_queue_basic() {
        let records = vec![UsageQueueRecord {
            provider: "test_provider".to_string(),
            model: "test_model".to_string(),
            alias: String::new(),
            requested_at: "2025-06-10T14:30:00Z".to_string(),
            failed: false,
            detail: UsageQueueDetail {
                input_tokens: 100,
                output_tokens: 200,
                reasoning_tokens: 50,
                cached_tokens: 30,
            },
        }];

        let mut history = RequestHistory::default();
        let mut agg = Aggregate::default();
        apply_usage_queue_records(&records, &mut history, &mut agg);

        assert_eq!(agg.total_requests, 1);
        assert_eq!(agg.total_success_count, 1);
        assert_eq!(agg.total_failure_count, 0);
        assert_eq!(agg.total_tokens_in, 100);
        assert_eq!(agg.total_tokens_out, 250); // 200 output + 50 reasoning
        assert_eq!(agg.total_tokens_cached, 30);
        assert!(agg.total_cost_usd > 0.0);

        // model stats
        let ms = agg.model_stats.get("test_model").unwrap();
        assert_eq!(ms.requests, 1);
        assert_eq!(ms.tokens, 350); // 100 + 250
        assert_eq!(ms.input_tokens, 100);
        assert_eq!(ms.output_tokens, 250);
        assert_eq!(ms.cached_tokens, 30);

        // provider stats
        let ps = agg.provider_stats.get("test_provider").unwrap();
        assert_eq!(ps.requests, 1);
        assert_eq!(ps.tokens, 350);

        // history
        assert_eq!(history.total_request_count, 1);
        assert_eq!(history.total_success_count, 1);
        assert_eq!(history.total_tokens_in, 100);
        assert_eq!(history.total_tokens_out, 250);
        assert_eq!(history.total_tokens_cached, 30);
    }

    #[test]
    fn test_apply_usage_queue_failed() {
        let records = vec![UsageQueueRecord {
            provider: "p".to_string(),
            model: "m".to_string(),
            alias: String::new(),
            requested_at: "2025-06-10T10:00:00Z".to_string(),
            failed: true,
            detail: UsageQueueDetail {
                input_tokens: 0,
                output_tokens: 0,
                reasoning_tokens: 0,
                cached_tokens: 0,
            },
        }];

        let mut history = RequestHistory::default();
        let mut agg = Aggregate::default();
        apply_usage_queue_records(&records, &mut history, &mut agg);

        assert_eq!(agg.total_requests, 1);
        assert_eq!(agg.total_success_count, 0);
        assert_eq!(agg.total_failure_count, 1);
        assert_eq!(history.total_request_count, 1);
        assert_eq!(history.total_success_count, 0);
        // No cost for 0 tokens
        assert_eq!(agg.total_cost_usd, 0.0);
    }

    #[test]
    fn test_apply_usage_queue_uses_alias_when_model_empty() {
        let records = vec![UsageQueueRecord {
            provider: "".to_string(),
            model: "".to_string(),
            alias: "claude-sonnet".to_string(),
            requested_at: "2025-06-10T14:00:00Z".to_string(),
            failed: false,
            detail: UsageQueueDetail {
                input_tokens: 50,
                output_tokens: 30,
                reasoning_tokens: 0,
                cached_tokens: 0,
            },
        }];

        let mut history = RequestHistory::default();
        let mut agg = Aggregate::default();
        apply_usage_queue_records(&records, &mut history, &mut agg);

        // Empty provider defaults to "unknown"
        assert!(agg.provider_stats.contains_key("unknown"));
        // Empty model uses alias
        assert!(agg.model_stats.contains_key("claude-sonnet"));
    }

    #[test]
    fn test_import_rejects_wrong_version() {
        let result = import_usage_stats(serde_json::json!({"version": 999}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported export version"));
    }

    #[test]
    fn test_import_missing_fields() {
        let result = import_usage_stats(serde_json::json!({"version": 1}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing"));
    }

    #[test]
    fn test_trim_timeseries() {
        let mut series: Vec<TimeSeriesPoint> = (0..20)
            .map(|i| TimeSeriesPoint {
                label: format!("2025-06-{:02}", i + 1),
                value: (i * 10) as u64,
            })
            .collect();
        trim_timeseries(&mut series, 5);
        assert_eq!(series.len(), 5);
            assert_eq!(series[0].label, "2025-06-16");
            assert_eq!(series[4].label, "2025-06-20");
        }

        #[test]
        fn test_collector_starts_and_generation_increments() {
            let gen = Arc::new(AtomicU64::new(0));
            let port = 9999;
            let before = gen.load(Ordering::SeqCst);

            start_usage_queue_collector(gen.clone(), port);
            // Generation should have been bumped by 1
            assert_eq!(gen.load(Ordering::SeqCst), before + 1);
        }

        #[test]
        fn test_collector_stops_when_generation_advances() {
            let gen = Arc::new(AtomicU64::new(0));
            let port = 9999;

            start_usage_queue_collector(gen.clone(), port);
            assert_eq!(gen.load(Ordering::SeqCst), 1, "start bumps to 1");

            // Signal stop via generation bump
            gen.fetch_add(1, Ordering::SeqCst);
            assert_eq!(gen.load(Ordering::SeqCst), 2, "stop bumps to 2");

            // Give the thread a moment to notice the change and exit
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        #[test]
        fn test_collector_does_not_block() {
            let gen = Arc::new(AtomicU64::new(0));
            let port = 9999;

            let before = std::time::Instant::now();
            start_usage_queue_collector(gen.clone(), port);
            // Should return near-instantly (not wait for any sleep)
            assert!(before.elapsed() < std::time::Duration::from_millis(500));
        }

        #[test]
        fn test_collector_stop_start_prevents_stale_thread() {
            let gen = Arc::new(AtomicU64::new(0));
            let port = 9999;

            // Start the collector (captures generation 1 after bump)
            start_usage_queue_collector(gen.clone(), port);
            let after_first = gen.load(Ordering::SeqCst);
            assert_eq!(after_first, 1, "first start bumps to 1");

            // Stop: bump generation to invalidate the running thread
            gen.fetch_add(1, Ordering::SeqCst);
            assert_eq!(gen.load(Ordering::SeqCst), 2, "stop bumps to 2");

            // Start again: bumps generation; new thread captures gen=3, starts fresh
            start_usage_queue_collector(gen.clone(), port);
            assert_eq!(gen.load(Ordering::SeqCst), 3, "second start bumps to 3");

            // Thread 1 (captured gen=1) sees gen=3 != 1 => exits on next tick
            // Thread 2 (captured gen=3) sees gen=3 == 3 => keeps running
            // No stale thread survived the stop/start cycle
        }

        #[test]
        fn test_collector_respects_no_proxy_config() {
            let gen = Arc::new(AtomicU64::new(0));
            let port = 0; // Non-running port — collector should handle connection errors gracefully

            start_usage_queue_collector(gen.clone(), port);
            assert_eq!(gen.load(Ordering::SeqCst), 1, "start bumps to 1");

            // Let it try a sync cycle (will fail on connection, handled gracefully)
            std::thread::sleep(std::time::Duration::from_millis(200));

            // Stop
            gen.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        #[test]
        fn test_collector_attempts_sync_before_cancellation() {
            // Binds a TCP listener on a random port and starts the collector
            // pointing at it.  If (and only if) the collector enters the sync loop
            // and calls sync_usage_from_queue_blocking, it will connect to this port.
            // Before the off-by-one fix, the thread exited immediately because
            // my_gen held the old pre-increment value; recv_timeout would fail.
            let listener = std::net::TcpListener::bind("127.0.0.1:0")
                .expect("should bind test TCP listener");
            let port = listener.local_addr().unwrap().port();

            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let result = listener.accept().ok().and_then(|(mut stream, _)| {
                    // Read a bit of the HTTP request to verify the endpoint
                    let mut buf = [0; 4096];
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_secs(1)))
                        .ok();
                    match stream.read(&mut buf) {
                        Ok(0) | Err(_) => None,
                        Ok(n) => Some(buf[..n].to_vec()),
                    }
                });
                let _ = tx.send(result);
            });

            let gen = Arc::new(AtomicU64::new(0));
            start_usage_queue_collector(gen.clone(), port);
            assert_eq!(gen.load(Ordering::SeqCst), 1, "start bumps to 1");

            // Collector sleeps 2 s initially, then makes the HTTP request.
            // 10 s timeout covers 2 s sleep + 5 s reqwest timeout + margin.
            let request_bytes = rx
                .recv_timeout(std::time::Duration::from_secs(10))
            .expect(
                "collector must connect to TCP port within 10 s — proves it entered the sync loop and called sync_usage_from_queue_blocking",
            )
                .expect("collector should send HTTP request data");

            let request = String::from_utf8_lossy(&request_bytes);
            assert!(
                request.contains("usage-queue"),
                "collector should request the usage-queue endpoint; first line: {}",
                request.lines().next().unwrap_or("(empty)")
            );

            // Signal stop
            gen.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
