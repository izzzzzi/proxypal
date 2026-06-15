//! API Keys Management - CRUD operations via Management API.

use tauri::State;
use crate::state::AppState;
use crate::config::save_config_to_file;
use crate::types::{GeminiApiKey, ClaudeApiKey, CodexApiKey, VertexApiKey, OpenAICompatibleProvider, ModelMapping};
use serde::{Deserialize, Serialize};

// Convert Management API kebab-case keys to camelCase for frontend
// The Management API returns data wrapped in an object like: { "gemini-api-key": [...] }
// It may also return null for empty lists: { "gemini-api-key": null }
fn convert_api_key_response<T: serde::de::DeserializeOwned>(json: serde_json::Value, wrapper_key: &str) -> Result<Vec<T>, String> {
    // Extract the array from the wrapper object
    let array_value = match &json {
        serde_json::Value::Object(obj) => {
            match obj.get(wrapper_key) {
                Some(serde_json::Value::Array(arr)) => serde_json::Value::Array(arr.clone()),
                Some(serde_json::Value::Null) | None => serde_json::Value::Array(vec![]), // null or missing = empty array
                Some(other) => return Err(format!("Expected array or null for key '{}', got: {:?}", wrapper_key, other)),
            }
        }
        serde_json::Value::Array(_) => json.clone(), // Already an array, use as-is
        serde_json::Value::Null => serde_json::Value::Array(vec![]), // Top-level null = empty array
        _ => return Err(format!("Unexpected response format: expected object with key '{}' or array", wrapper_key)),
    };
    
    // The Management API returns kebab-case, we need to convert
    let json_str = serde_json::to_string(&array_value).map_err(|e| e.to_string())?;
    // Replace kebab-case with camelCase for our structs
    let converted = json_str
        .replace("\"api-key\"", "\"apiKey\"")
        .replace("\"base-url\"", "\"baseUrl\"")
        .replace("\"proxy-url\"", "\"proxyUrl\"")
        .replace("\"excluded-models\"", "\"excludedModels\"")
        .replace("\"api-key-entries\"", "\"apiKeyEntries\"");
    serde_json::from_str(&converted).map_err(|e| e.to_string())
}

// Convert camelCase to kebab-case for Management API
fn convert_to_management_format<T: serde::Serialize>(data: &T) -> Result<serde_json::Value, String> {
    let json_str = serde_json::to_string(data).map_err(|e| e.to_string())?;
    let converted = json_str
        .replace("\"apiKey\"", "\"api-key\"")
        .replace("\"baseUrl\"", "\"base-url\"")
        .replace("\"proxyUrl\"", "\"proxy-url\"")
        .replace("\"excludedModels\"", "\"excluded-models\"")
        .replace("\"apiKeyEntries\"", "\"api-key-entries\"");
    serde_json::from_str(&converted).map_err(|e| e.to_string())
}

fn normalize_model_mappings(models: Option<&Vec<ModelMapping>>) -> Vec<ModelMapping> {
    models
        .map(|items| {
            items
                .iter()
                .map(|model| {
                    let alias = model
                        .alias
                        .as_deref()
                        .map(str::trim)
                        .filter(|alias| !alias.is_empty())
                        .unwrap_or(model.name.as_str())
                        .to_string();

                    ModelMapping {
                        name: model.name.clone(),
                        alias: Some(alias),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn normalize_openai_compatible_providers(providers: &[OpenAICompatibleProvider]) -> Vec<OpenAICompatibleProvider> {
    providers
        .iter()
        .map(|provider| OpenAICompatibleProvider {
            name: provider.name.clone(),
            base_url: provider.base_url.clone(),
            api_key_entries: provider.api_key_entries.clone(),
            models: Some(normalize_model_mappings(provider.models.as_ref())),
            headers: provider.headers.clone(),
            prefix: provider.prefix.clone(),
        })
        .collect()
}

// ============================================
// Gemini API Keys
// ============================================

#[tauri::command]
pub async fn get_gemini_api_keys(state: State<'_, AppState>) -> Result<Vec<GeminiApiKey>, String> {
    let port = state.config.lock().unwrap().port;
    let url = crate::get_management_url(port, "gemini-api-key");
    
    let client = crate::build_management_client();
    let response = client
        .get(&url)
        .header("X-Management-Key", &crate::get_management_key())
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Gemini API keys: {}", e))?;
    
    if !response.status().is_success() {
        return Ok(Vec::new());
    }
    
    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    convert_api_key_response(json, "gemini-api-key")
}

#[tauri::command]
pub async fn set_gemini_api_keys(state: State<'_, AppState>, keys: Vec<GeminiApiKey>) -> Result<(), String> {
    let port = state.config.lock().unwrap().port;
    let url = crate::get_management_url(port, "gemini-api-key");
    
    let client = crate::build_management_client();
    let body = convert_to_management_format(&keys)?;
    
    let response = client
        .put(&url)
        .header("X-Management-Key", &crate::get_management_key())
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to set Gemini API keys: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Failed to set Gemini API keys: {} - {}", status, text));
    }
    
    // Persist to ProxyPal config for restart persistence
    {
        let mut config = state.config.lock().unwrap();
        config.gemini_api_keys = keys;
        save_config_to_file(&config)?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn add_gemini_api_key(state: State<'_, AppState>, key: GeminiApiKey) -> Result<(), String> {
    let mut keys = get_gemini_api_keys(state.clone()).await?;
    keys.push(key);
    set_gemini_api_keys(state, keys).await
}

#[tauri::command]
pub async fn delete_gemini_api_key(state: State<'_, AppState>, index: usize) -> Result<(), String> {
    let mut keys = get_gemini_api_keys(state.clone()).await?;
    if index >= keys.len() {
        return Err("Index out of bounds".to_string());
    }
    keys.remove(index);
    set_gemini_api_keys(state, keys).await
}

// Validation result for a single Claude API key
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeKeyHealth {
    pub index: usize,
    pub key_prefix: String,
    pub status: String, // "valid" | "invalid" | "low_balance" | "error"
    pub message: String,
}

// Validate all Claude API keys against Anthropic API
#[tauri::command]
pub async fn validate_claude_api_keys(state: State<'_, AppState>) -> Result<Vec<ClaudeKeyHealth>, String> {
    let keys = {
        let config = state.config.lock().unwrap();
        config.claude_api_keys.clone()
    };

    if keys.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    for (i, key_entry) in keys.iter().enumerate() {
        let raw_key = &key_entry.api_key;
        let prefix = if raw_key.len() > 25 {
            format!("{}...{}", &raw_key[..20], &raw_key[raw_key.len()-10..])
        } else {
            raw_key.clone()
        };

        // Determine base URL (use custom if set, otherwise default Anthropic)
        let base_url = key_entry
            .base_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com")
            .trim_end_matches('/');
        let messages_url = format!("{}/v1/messages", base_url);

        match client
            .post(&messages_url)
            .header("Content-Type", "application/json")
            .header("x-api-key", raw_key)
            .header("anthropic-version", "2023-06-01")
            .json(&serde_json::json!({
                "model": "claude-sonnet-4-20250514",
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "ok"}]
            }))
            .send()
            .await
        {
            Ok(resp) => {
                let status_code = resp.status().as_u16();
                if resp.status().is_success() {
                    results.push(ClaudeKeyHealth {
                        index: i,
                        key_prefix: prefix,
                        status: "valid".to_string(),
                        message: "API key is valid and responding".to_string(),
                    });
                } else if status_code == 401 {
                    results.push(ClaudeKeyHealth {
                        index: i,
                        key_prefix: prefix,
                        status: "invalid".to_string(),
                        message: "Invalid API key (HTTP 401)".to_string(),
                    });
                } else if status_code == 402 || status_code == 429 {
                    let body = resp.text().await.unwrap_or_default();
                    let msg = if body.contains("credit balance") || body.contains("low") {
                        "Insufficient credit balance".to_string()
                    } else {
                        format!("Quota exceeded (HTTP {}): {:.100}", status_code, body)
                    };
                    results.push(ClaudeKeyHealth {
                        index: i,
                        key_prefix: prefix,
                        status: "low_balance".to_string(),
                        message: msg,
                    });
                } else if status_code == 404 {
                    // 404 model not found means the key IS authenticated
                    results.push(ClaudeKeyHealth {
                        index: i,
                        key_prefix: prefix,
                        status: "valid".to_string(),
                        message: "API key is valid (model unavailable, check account tier)".to_string(),
                    });
                } else {
                    let body = resp.text().await.unwrap_or_default();
                    if body.contains("credit balance") || body.contains("low") {
                        results.push(ClaudeKeyHealth {
                            index: i,
                            key_prefix: prefix,
                            status: "low_balance".to_string(),
                            message: "Insufficient credit balance".to_string(),
                        });
                    } else {
                        results.push(ClaudeKeyHealth {
                            index: i,
                            key_prefix: prefix,
                            status: "error".to_string(),
                            message: format!("HTTP {}: {:.150}", status_code, body),
                        });
                    }
                }
            }
            Err(e) => {
                results.push(ClaudeKeyHealth {
                    index: i,
                    key_prefix: prefix,
                    status: "error".to_string(),
                    message: format!("Request failed: {}", e),
                });
            }
        }
    }

    Ok(results)
}

// Remove Claude API keys that are invalid or have low balance
#[tauri::command]
pub async fn cleanup_claude_api_keys(state: State<'_, AppState>) -> Result<Vec<ClaudeKeyHealth>, String> {
    let results = validate_claude_api_keys(state.clone()).await?;

    let bad_indices: Vec<usize> = results
        .iter()
        .filter(|r| r.status == "invalid" || r.status == "low_balance")
        .map(|r| r.index)
        .collect();

    if bad_indices.is_empty() {
        return Ok(results);
    }

    let mut config = state.config.lock().unwrap();
    let original_len = config.claude_api_keys.len();
    // Remove in reverse order to keep indices stable
    let mut idx = 0;
    config.claude_api_keys.retain(|_| {
        let keep = !bad_indices.contains(&idx);
        idx += 1;
        keep
    });
    let removed_count = original_len - config.claude_api_keys.len();
    save_config_to_file(&config).map_err(|e| format!("Failed to save config: {}", e))?;

    // Also update via Management API
    let port = config.port;
    let url = crate::get_management_url(port, "claude-api-key");
    let client = crate::build_management_client();
    let body = serde_json::to_value(&config.claude_api_keys).map_err(|e| e.to_string())?;
    // Convert camelCase to kebab-case for Management API
    let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    let converted = body_str
        .replace("\"apiKey\"", "\"api-key\"")
        .replace("\"baseUrl\"", "\"base-url\"")
        .replace("\"proxyUrl\"", "\"proxy-url\"")
        .replace("\"excludedModels\"", "\"excluded-models\"");
    let management_body: serde_json::Value =
        serde_json::from_str(&converted).map_err(|e| e.to_string())?;

    let _ = client
        .put(&url)
        .header("X-Management-Key", &crate::get_management_key())
        .json(&management_body)
        .send()
        .await;

    eprintln!("[ProxyPal] Cleaned up {} invalid/low-balance Claude API key(s)", removed_count);

    Ok(results)
}

// ============================================
// Claude API Keys
// ============================================

#[tauri::command]
pub async fn get_claude_api_keys(state: State<'_, AppState>) -> Result<Vec<ClaudeApiKey>, String> {
    let port = state.config.lock().unwrap().port;
    let url = crate::get_management_url(port, "claude-api-key");
    
    let client = crate::build_management_client();
    let response = client
        .get(&url)
        .header("X-Management-Key", &crate::get_management_key())
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Claude API keys: {}", e))?;
    
    if !response.status().is_success() {
        return Ok(Vec::new());
    }
    
    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    convert_api_key_response(json, "claude-api-key")
}

#[tauri::command]
pub async fn set_claude_api_keys(state: State<'_, AppState>, keys: Vec<ClaudeApiKey>) -> Result<(), String> {
    let port = state.config.lock().unwrap().port;
    let url = crate::get_management_url(port, "claude-api-key");
    
    let client = crate::build_management_client();
    let body = convert_to_management_format(&keys)?;
    
    let response = client
        .put(&url)
        .header("X-Management-Key", &crate::get_management_key())
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to set Claude API keys: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Failed to set Claude API keys: {} - {}", status, text));
    }
    
    // Persist to ProxyPal config for restart persistence
    {
        let mut config = state.config.lock().unwrap();
        config.claude_api_keys = keys;
        save_config_to_file(&config)?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn add_claude_api_key(state: State<'_, AppState>, key: ClaudeApiKey) -> Result<(), String> {
    let mut keys = get_claude_api_keys(state.clone()).await?;
    keys.push(key);
    set_claude_api_keys(state, keys).await
}

#[tauri::command]
pub async fn delete_claude_api_key(state: State<'_, AppState>, index: usize) -> Result<(), String> {
    let mut keys = get_claude_api_keys(state.clone()).await?;
    if index >= keys.len() {
        return Err("Index out of bounds".to_string());
    }
    keys.remove(index);
    set_claude_api_keys(state, keys).await
}

// ============================================
// Codex API Keys
// ============================================

#[tauri::command]
pub async fn get_codex_api_keys(state: State<'_, AppState>) -> Result<Vec<CodexApiKey>, String> {
    let port = state.config.lock().unwrap().port;
    let url = crate::get_management_url(port, "codex-api-key");
    
    let client = crate::build_management_client();
    let response = client
        .get(&url)
        .header("X-Management-Key", &crate::get_management_key())
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Codex API keys: {}", e))?;
    
    if !response.status().is_success() {
        return Ok(Vec::new());
    }
    
    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    convert_api_key_response(json, "codex-api-key")
}

#[tauri::command]
pub async fn set_codex_api_keys(state: State<'_, AppState>, keys: Vec<CodexApiKey>) -> Result<(), String> {
    let port = state.config.lock().unwrap().port;
    let url = crate::get_management_url(port, "codex-api-key");
    
    let client = crate::build_management_client();
    let body = convert_to_management_format(&keys)?;
    
    let response = client
        .put(&url)
        .header("X-Management-Key", &crate::get_management_key())
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to set Codex API keys: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Failed to set Codex API keys: {} - {}", status, text));
    }
    
    // Persist to ProxyPal config for restart persistence
    {
        let mut config = state.config.lock().unwrap();
        config.codex_api_keys = keys;
        save_config_to_file(&config)?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn add_codex_api_key(state: State<'_, AppState>, key: CodexApiKey) -> Result<(), String> {
    let mut keys = get_codex_api_keys(state.clone()).await?;
    keys.push(key);
    set_codex_api_keys(state, keys).await
}

#[tauri::command]
pub async fn delete_codex_api_key(state: State<'_, AppState>, index: usize) -> Result<(), String> {
    let mut keys = get_codex_api_keys(state.clone()).await?;
    if index >= keys.len() {
        return Err("Index out of bounds".to_string());
    }
    keys.remove(index);
    set_codex_api_keys(state, keys).await
}

// ============================================
// Vertex API Keys
// ============================================

#[tauri::command]
pub async fn get_vertex_api_keys(state: State<'_, AppState>) -> Result<Vec<VertexApiKey>, String> {
    let port = state.config.lock().unwrap().port;
    let url = crate::get_management_url(port, "vertex-api-key");
    
    let client = crate::build_management_client();
    let response = client
        .get(&url)
        .header("X-Management-Key", &crate::get_management_key())
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Vertex API keys: {}", e))?;
    
    if !response.status().is_success() {
        return Ok(Vec::new());
    }
    
    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    convert_api_key_response(json, "vertex-api-key")
}

#[tauri::command]
pub async fn set_vertex_api_keys(state: State<'_, AppState>, keys: Vec<VertexApiKey>) -> Result<(), String> {
    let port = state.config.lock().unwrap().port;
    let url = crate::get_management_url(port, "vertex-api-key");
    
    let client = crate::build_management_client();
    let body = convert_to_management_format(&keys)?;
    
    let response = client
        .put(&url)
        .header("X-Management-Key", &crate::get_management_key())
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to set Vertex API keys: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Failed to set Vertex API keys: {} - {}", status, text));
    }
    
    // Persist to ProxyPal config for restart persistence
    {
        let mut config = state.config.lock().unwrap();
        config.vertex_api_keys = keys;
        save_config_to_file(&config)?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn add_vertex_api_key(state: State<'_, AppState>, key: VertexApiKey) -> Result<(), String> {
    let mut keys = get_vertex_api_keys(state.clone()).await?;
    keys.push(key);
    set_vertex_api_keys(state, keys).await
}

#[tauri::command]
pub async fn delete_vertex_api_key(state: State<'_, AppState>, index: usize) -> Result<(), String> {
    let mut keys = get_vertex_api_keys(state.clone()).await?;
    if index >= keys.len() {
        return Err("Index out of bounds".to_string());
    }
    keys.remove(index);
    set_vertex_api_keys(state, keys).await
}

// ============================================
// OpenAI-Compatible Providers
// ============================================

#[tauri::command]
pub async fn get_openai_compatible_providers(state: State<'_, AppState>) -> Result<Vec<OpenAICompatibleProvider>, String> {
    let port = state.config.lock().unwrap().port;
    let url = crate::get_management_url(port, "openai-compatibility");
    
    let client = crate::build_management_client();
    let response = client
        .get(&url)
        .header("X-Management-Key", &crate::get_management_key())
        .send()
        .await
        .map_err(|e| format!("Failed to fetch OpenAI-compatible providers: {}", e))?;
    
    if !response.status().is_success() {
        return Ok(Vec::new());
    }
    
    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    convert_api_key_response(json, "openai-compatibility")
}

#[tauri::command]
pub async fn set_openai_compatible_providers(state: State<'_, AppState>, providers: Vec<OpenAICompatibleProvider>) -> Result<(), String> {
    let port = state.config.lock().unwrap().port;
    let url = crate::get_management_url(port, "openai-compatibility");
    let normalized_providers = normalize_openai_compatible_providers(&providers);
    
    let client = crate::build_management_client();
    let body = convert_to_management_format(&normalized_providers)?;
    
    let response = client
        .put(&url)
        .header("X-Management-Key", &crate::get_management_key())
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to set OpenAI-compatible providers: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Failed to set OpenAI-compatible providers: {} - {}", status, text));
    }
    
    // Persist to local config for restart persistence
    {
        let mut config = state.config.lock().unwrap();
        config.amp_openai_providers = normalized_providers.iter().map(|p| {
            crate::types::amp::AmpOpenAIProvider {
                id: uuid::Uuid::new_v4().to_string(),
                name: p.name.clone(),
                base_url: p.base_url.clone(),
                api_key: p.api_key_entries.first().map(|e| e.api_key.clone()).unwrap_or_default(),
                models: normalize_model_mappings(p.models.as_ref()).into_iter().map(|model| crate::types::amp::AmpOpenAIModel {
                    name: model.name,
                    alias: model.alias.unwrap_or_default(),
                }).collect(),
            }
        }).collect();
    }
    let config_to_save = state.config.lock().unwrap().clone();
    crate::config::save_config_to_file(&config_to_save)?;
    
    Ok(())
}

#[tauri::command]
pub async fn add_openai_compatible_provider(state: State<'_, AppState>, provider: OpenAICompatibleProvider) -> Result<(), String> {
    let mut providers = get_openai_compatible_providers(state.clone()).await?;
    providers.push(provider);
    set_openai_compatible_providers(state, providers).await
}

#[tauri::command]
pub async fn delete_openai_compatible_provider(state: State<'_, AppState>, index: usize) -> Result<(), String> {
    let mut providers = get_openai_compatible_providers(state.clone()).await?;
    if index >= providers.len() {
        return Err("Index out of bounds".to_string());
    }
    providers.remove(index);
    set_openai_compatible_providers(state, providers).await
}
