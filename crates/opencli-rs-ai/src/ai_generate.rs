//! AI-powered adapter generation.
//! Captures full page data (network requests + responses, metadata, framework)
//! and sends it to an LLM to generate a precise YAML adapter.

use opencli_rs_core::{CliError, IPage};
use serde_json::{json, Value};
use tracing::{debug, info};

use crate::config::LlmConfig;
use crate::explore::detect_site_name;
use crate::llm::generate_with_llm;

/// Capture all API data from a page for AI analysis.
/// Installs fetch/XHR interceptors, navigates, scrolls, then collects everything.
pub async fn capture_page_data(
    page: &dyn IPage,
    url: &str,
) -> Result<Value, CliError> {
    info!(url = url, "Capturing page data for AI analysis");

    // Step 1: Navigate to page
    page.goto(url, None).await?;
    page.wait_for_timeout(5000).await?;

    // Step 2: Scroll to trigger lazy loading
    let _ = page.auto_scroll(Some(opencli_rs_core::AutoScrollOptions {
        max_scrolls: Some(3),
        delay_ms: Some(1500),
        ..Default::default()
    })).await;

    page.wait_for_timeout(2000).await?;

    // Step 3: Collect all data in one evaluate call
    let js = r#"(async () => {
        // Get all API URLs from Performance entries
        const perfEntries = performance.getEntriesByType('resource')
            .map(e => e.name)
            .filter(u => {
                const l = u.toLowerCase();
                return (l.includes('/api/') || l.includes('/v1/') || l.includes('/v2/')
                    || l.includes('/v3/') || l.includes('/x/') || l.includes('.json')
                    || l.includes('graphql') || l.includes('search') || l.includes('feed')
                    || l.includes('hot') || l.includes('trending') || l.includes('list')
                    || l.includes('recommend') || l.includes('query'))
                    && !l.endsWith('.js') && !l.endsWith('.css') && !l.endsWith('.png')
                    && !l.endsWith('.jpg') && !l.endsWith('.svg') && !l.endsWith('.woff2');
            });

        // Deduplicate by pathname
        const seen = new Set();
        const uniqueUrls = perfEntries.filter(url => {
            try {
                const key = new URL(url).pathname;
                if (seen.has(key)) return false;
                seen.add(key);
                return true;
            } catch { return false; }
        });

        // Re-fetch each API to get response body
        const apiResponses = [];
        for (const url of uniqueUrls.slice(0, 20)) {
            try {
                const resp = await fetch(url, { credentials: 'include' });
                if (!resp.ok) continue;
                const ct = resp.headers.get('content-type') || '';
                if (!ct.includes('json')) continue;
                const body = await resp.json();
                apiResponses.push({
                    url: url,
                    method: 'GET',
                    status: resp.status,
                    body: JSON.stringify(body).slice(0, 50000),
                });
            } catch {}
        }

        // Page metadata
        const meta = {
            url: location.href,
            title: document.title,
        };

        // Framework detection
        const app = document.querySelector('#app');
        const framework = {};
        try { framework.vue3 = !!app?.__vue_app__; } catch {}
        try { framework.pinia = !!(app?.__vue_app__?.config?.globalProperties?.$pinia); } catch {}
        try { framework.react = !!document.querySelector('[data-reactroot]') || !!window.__REACT_DEVTOOLS_GLOBAL_HOOK__; } catch {}
        try { framework.nextjs = !!window.__NEXT_DATA__; } catch {}
        try { framework.nuxt = !!window.__NUXT__; } catch {}

        // Global state variables
        const globals = {};
        try { if (window.__INITIAL_STATE__) globals.__INITIAL_STATE__ = JSON.stringify(window.__INITIAL_STATE__).slice(0, 30000); } catch {}
        try { if (window.__NEXT_DATA__) globals.__NEXT_DATA__ = JSON.stringify(window.__NEXT_DATA__).slice(0, 30000); } catch {}
        try { if (window.__NUXT__) globals.__NUXT__ = JSON.stringify(window.__NUXT__).slice(0, 30000); } catch {}

        return {
            meta,
            framework,
            globals,
            intercepted: apiResponses,
            perf_urls: uniqueUrls,
        };
    })()"#;

    let data = page.evaluate(js).await?;

    if data.is_null() {
        return Err(CliError::empty_result("Failed to capture page data"));
    }

    debug!(
        apis = data.get("intercepted").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
        perf_urls = data.get("perf_urls").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
        "Page data captured"
    );

    Ok(data)
}

/// AI-powered generate: capture page data → send to LLM → save YAML adapter.
pub async fn generate_with_ai(
    page: &dyn IPage,
    url: &str,
    goal: &str,
    llm_config: &LlmConfig,
) -> Result<(String, String, String), CliError> {
    // Step 1: Capture page data
    eprintln!("📡 Capturing page data...");
    let captured = capture_page_data(page, url).await?;

    // Step 2: Detect site name
    let site = detect_site_name(url);

    // Step 3: Send to LLM
    eprintln!("🤖 Sending to AI for analysis...");
    let yaml = generate_with_llm(llm_config, &captured, goal, &site).await?;

    // Step 4: Ensure site field matches detected site name (LLM may use localized names)
    let yaml = if let Some(line) = yaml.lines().find(|l| l.starts_with("site:")) {
        yaml.replacen(line, &format!("site: {}", site), 1)
    } else {
        yaml.clone()
    };

    // Step 5: Extract command name from YAML
    let name = yaml.lines()
        .find(|l| l.starts_with("name:"))
        .and_then(|l| l.strip_prefix("name:"))
        .map(|s| s.trim().trim_matches('"').to_string())
        .unwrap_or_else(|| goal.to_string());

    Ok((site, name, yaml))
}
