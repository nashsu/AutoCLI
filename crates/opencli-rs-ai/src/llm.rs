//! LLM client for AI-powered adapter generation.
//! Supports OpenAI-compatible API (works with OpenAI, Anthropic via proxy, local models, etc.)

use opencli_rs_core::CliError;
use serde_json::{json, Value};
use tracing::{debug, info};

use crate::config::LlmConfig;

/// The system prompt for adapter generation, embedded at compile time.
const SYSTEM_PROMPT: &str = include_str!("../prompts/generate-adapter.md");

/// Send captured page data to LLM and get back a YAML adapter.
pub async fn generate_with_llm(
    config: &LlmConfig,
    captured_data: &Value,
    goal: &str,
    site: &str,
) -> Result<String, CliError> {
    let endpoint = config.endpoint.as_deref()
        .ok_or_else(|| CliError::config("LLM endpoint not configured. Set it in ~/.opencli-rs/config.json"))?;
    let apikey = config.apikey.as_deref()
        .ok_or_else(|| CliError::config("LLM API key not configured. Set it in ~/.opencli-rs/config.json"))?;
    let model = config.modelname.as_deref()
        .ok_or_else(|| CliError::config("LLM model name not configured. Set it in ~/.opencli-rs/config.json"))?;

    let user_message = format!(
        "Generate an opencli-rs YAML adapter for site \"{}\" with goal \"{}\".\n\n\
        Here is the captured data from the web page:\n\n```json\n{}\n```\n\n\
        Return ONLY the YAML content, no explanation, no markdown fencing. Just the raw YAML.",
        site, goal,
        serde_json::to_string_pretty(captured_data)
            .unwrap_or_else(|_| captured_data.to_string())
    );

    info!(endpoint = endpoint, model = model, "Calling LLM for adapter generation");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| CliError::Http { message: format!("Failed to create HTTP client: {}", e), suggestions: vec![], source: None })?;

    // Detect API type from endpoint URL and build request accordingly
    let (request_body, auth_header) = if endpoint.contains("anthropic") {
        // Anthropic Messages API
        let body = json!({
            "model": model,
            "max_tokens": 4096,
            "system": SYSTEM_PROMPT,
            "messages": [
                { "role": "user", "content": user_message }
            ]
        });
        (body, ("x-api-key", apikey.to_string()))
    } else {
        // OpenAI-compatible API (OpenAI, Azure, local models, etc.)
        let body = json!({
            "model": model,
            "max_tokens": 4096,
            "messages": [
                { "role": "system", "content": SYSTEM_PROMPT },
                { "role": "user", "content": user_message }
            ]
        });
        (body, ("Authorization", format!("Bearer {}", apikey)))
    };

    debug!(body_size = request_body.to_string().len(), "Sending LLM request");

    let resp = client
        .post(endpoint)
        .header(auth_header.0, auth_header.1)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| CliError::Http { message: format!("LLM request failed: {}", e), suggestions: vec![], source: None })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(CliError::Http { message: format!("LLM API error {}: {}", status, body.chars().take(500).collect::<String>()), suggestions: vec![], source: None });
    }

    let resp_json: Value = resp.json().await
        .map_err(|e| CliError::Http { message: format!("Failed to parse LLM response: {}", e), suggestions: vec![], source: None })?;

    // Extract content from response (handle both Anthropic and OpenAI formats)
    let content = if let Some(choices) = resp_json.get("choices") {
        // OpenAI format
        choices.get(0)
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string()
    } else if let Some(content_arr) = resp_json.get("content") {
        // Anthropic format
        content_arr.get(0)
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string()
    } else {
        return Err(CliError::Http { message: "Unexpected LLM response format".into(), suggestions: vec![], source: None });
    };

    // Clean up: remove markdown fencing if present
    let yaml = content
        .trim()
        .strip_prefix("```yaml").or_else(|| content.trim().strip_prefix("```"))
        .unwrap_or(content.trim())
        .strip_suffix("```")
        .unwrap_or(content.trim())
        .trim()
        .to_string();

    if yaml.is_empty() {
        return Err(CliError::Http { message: "LLM returned empty content".into(), suggestions: vec![], source: None });
    }

    info!(yaml_len = yaml.len(), "LLM generated adapter YAML");
    Ok(yaml)
}
