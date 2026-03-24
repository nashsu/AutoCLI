/// Generate JS to click an element by CSS selector.
pub fn click_js(selector: &str) -> String {
    let escaped = selector.replace('\\', "\\\\").replace('\'', "\\'");
    format!(
        r#"(() => {{
  const el = document.querySelector('{escaped}');
  if (!el) throw new Error('Element not found: {escaped}');
  el.click();
  return true;
}})()"#
    )
}

/// Generate JS to type text into an element by CSS selector.
pub fn type_text_js(selector: &str, text: &str) -> String {
    let sel = selector.replace('\\', "\\\\").replace('\'', "\\'");
    let txt = text.replace('\\', "\\\\").replace('\'', "\\'");
    format!(
        r#"(() => {{
  const el = document.querySelector('{sel}');
  if (!el) throw new Error('Element not found: {sel}');
  el.focus();
  el.value = '{txt}';
  el.dispatchEvent(new Event('input', {{ bubbles: true }}));
  el.dispatchEvent(new Event('change', {{ bubbles: true }}));
  return true;
}})()"#
    )
}

/// Generate JS to simulate a key press.
pub fn press_key_js(key: &str) -> String {
    let k = key.replace('\\', "\\\\").replace('\'', "\\'");
    format!(
        r#"(() => {{
  const target = document.activeElement || document.body;
  const opts = {{ key: '{k}', code: '{k}', bubbles: true, cancelable: true }};
  target.dispatchEvent(new KeyboardEvent('keydown', opts));
  target.dispatchEvent(new KeyboardEvent('keypress', opts));
  target.dispatchEvent(new KeyboardEvent('keyup', opts));
  return true;
}})()"#
    )
}

/// Generate JS to scroll in a given direction by an amount of pixels.
pub fn scroll_js(direction: &str, amount: i32) -> String {
    let y = if direction == "up" { -amount } else { amount };
    format!(
        r#"(() => {{
  window.scrollBy({{ top: {y}, behavior: 'smooth' }});
  return {{ scrollY: window.scrollY, scrollHeight: document.body.scrollHeight }};
}})()"#
    )
}

/// Generate JS to auto-scroll the page repeatedly.
pub fn auto_scroll_js(max_scrolls: u32, delay_ms: u64) -> String {
    format!(
        r#"(async () => {{
  let prev = -1;
  let scrolls = 0;
  const max = {max_scrolls};
  const delay = {delay_ms};
  while (scrolls < max) {{
    window.scrollBy(0, window.innerHeight);
    await new Promise(r => setTimeout(r, delay));
    const cur = window.scrollY;
    if (cur === prev) break;
    prev = cur;
    scrolls++;
  }}
  return {{ scrolls, scrollY: window.scrollY, scrollHeight: document.body.scrollHeight }};
}})()"#
    )
}

/// Generate JS that waits for DOM stability (no mutations for a period).
pub fn wait_for_dom_stable_js() -> String {
    r#"(async () => {
  return new Promise((resolve) => {
    let timer;
    const observer = new MutationObserver(() => {
      clearTimeout(timer);
      timer = setTimeout(() => { observer.disconnect(); resolve(true); }, 500);
    });
    observer.observe(document.body, { childList: true, subtree: true, attributes: true });
    timer = setTimeout(() => { observer.disconnect(); resolve(true); }, 2000);
  });
})()"#
        .to_string()
}

/// Generate JS to capture network request information from Performance API.
pub fn network_requests_js() -> String {
    r#"(() => {
  const entries = performance.getEntriesByType('resource');
  return entries.map(e => ({
    url: e.name,
    method: 'GET',
    status: null,
    headers: {},
    body: null,
    response_body: null,
    duration: e.duration,
    type: e.initiatorType,
  }));
})()"#
        .to_string()
}

/// Generate JS to install a network request interceptor for a URL pattern.
pub fn install_interceptor_js(pattern: &str) -> String {
    let pat = pattern.replace('\\', "\\\\").replace('\'', "\\'");
    format!(
        r#"(() => {{
  if (!window.__opencli_intercepted) window.__opencli_intercepted = [];
  const regex = new RegExp('{pat}');
  const origFetch = window.fetch;
  window.fetch = async function(...args) {{
    const req = new Request(...args);
    const url = req.url;
    if (regex.test(url)) {{
      const body = await req.clone().text().catch(() => null);
      window.__opencli_intercepted.push({{
        url, method: req.method,
        headers: Object.fromEntries(req.headers.entries()),
        body,
      }});
    }}
    return origFetch.apply(this, args);
  }};
  const origXhr = XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open = function(method, url, ...rest) {{
    if (regex.test(url)) {{
      this.__opencli_url = url;
      this.__opencli_method = method;
    }}
    return origXhr.call(this, method, url, ...rest);
  }};
  const origSend = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.send = function(body) {{
    if (this.__opencli_url) {{
      window.__opencli_intercepted.push({{
        url: this.__opencli_url, method: this.__opencli_method,
        headers: {{}}, body: typeof body === 'string' ? body : null,
      }});
    }}
    return origSend.call(this, body);
  }};
  return true;
}})()"#
    )
}

/// Generate JS to retrieve intercepted requests.
pub fn get_intercepted_requests_js() -> String {
    r#"(() => {
  const reqs = window.__opencli_intercepted || [];
  window.__opencli_intercepted = [];
  return reqs;
})()"#
        .to_string()
}

/// Generate JS to get a DOM snapshot as simplified accessibility tree.
pub fn snapshot_js(selector: Option<&str>, include_hidden: bool) -> String {
    let root = match selector {
        Some(s) => {
            let sel = s.replace('\\', "\\\\").replace('\'', "\\'");
            format!("document.querySelector('{sel}') || document.body")
        }
        None => "document.body".to_string(),
    };
    let hidden_check = if include_hidden {
        "false"
    } else {
        "getComputedStyle(el).display === 'none' || getComputedStyle(el).visibility === 'hidden'"
    };
    format!(
        r#"(() => {{
  function walk(el, depth) {{
    if ({hidden_check}) return null;
    const tag = el.tagName ? el.tagName.toLowerCase() : '';
    const role = el.getAttribute && el.getAttribute('role') || '';
    const text = el.childNodes.length === 1 && el.childNodes[0].nodeType === 3
      ? el.childNodes[0].textContent.trim().slice(0, 200) : '';
    const children = [];
    for (const child of el.children || []) {{
      const c = walk(child, depth + 1);
      if (c) children.push(c);
    }}
    if (!tag && !text && children.length === 0) return null;
    const node = {{ tag }};
    if (role) node.role = role;
    if (text) node.text = text;
    if (el.id) node.id = el.id;
    if (el.className && typeof el.className === 'string') node.class = el.className.slice(0, 100);
    if (el.href) node.href = el.href;
    if (el.src) node.src = el.src;
    if (children.length > 0) node.children = children;
    return node;
  }}
  const root = {root};
  return walk(root, 0);
}})()"#
    )
}

/// Generate JS to wait for a selector to appear.
pub fn wait_for_selector_js(selector: &str, timeout_ms: u64, visible: bool) -> String {
    let sel = selector.replace('\\', "\\\\").replace('\'', "\\'");
    let visible_check = if visible {
        " && el.offsetParent !== null"
    } else {
        ""
    };
    format!(
        r#"(async () => {{
  const deadline = Date.now() + {timeout_ms};
  while (Date.now() < deadline) {{
    const el = document.querySelector('{sel}');
    if (el{visible_check}) return true;
    await new Promise(r => setTimeout(r, 100));
  }}
  throw new Error('Timeout waiting for selector: {sel}');
}})()"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_click_js_contains_selector() {
        let js = click_js("#btn");
        assert!(js.contains("#btn"));
        assert!(js.contains("querySelector"));
        assert!(js.contains(".click()"));
    }

    #[test]
    fn test_type_text_js_contains_text() {
        let js = type_text_js("input.name", "hello world");
        assert!(js.contains("input.name"));
        assert!(js.contains("hello world"));
        assert!(js.contains(".value ="));
    }

    #[test]
    fn test_press_key_js() {
        let js = press_key_js("Enter");
        assert!(js.contains("Enter"));
        assert!(js.contains("keydown"));
    }

    #[test]
    fn test_scroll_js_up() {
        let js = scroll_js("up", 500);
        assert!(js.contains("-500"));
    }

    #[test]
    fn test_scroll_js_down() {
        let js = scroll_js("down", 300);
        assert!(js.contains("300"));
    }

    #[test]
    fn test_auto_scroll_js() {
        let js = auto_scroll_js(10, 200);
        assert!(js.contains("10"));
        assert!(js.contains("200"));
    }

    #[test]
    fn test_network_requests_js() {
        let js = network_requests_js();
        assert!(js.contains("getEntriesByType"));
    }

    #[test]
    fn test_install_interceptor_js() {
        let js = install_interceptor_js("api\\.example\\.com");
        assert!(js.contains("api\\\\.example\\\\.com"));
        assert!(js.contains("__opencli_intercepted"));
    }

    #[test]
    fn test_get_intercepted_requests_js() {
        let js = get_intercepted_requests_js();
        assert!(js.contains("__opencli_intercepted"));
    }

    #[test]
    fn test_snapshot_js() {
        let js = snapshot_js(None, false);
        assert!(js.contains("document.body"));
        assert!(js.contains("walk"));
    }

    #[test]
    fn test_wait_for_selector_js() {
        let js = wait_for_selector_js(".loading", 5000, true);
        assert!(js.contains(".loading"));
        assert!(js.contains("5000"));
        assert!(js.contains("offsetParent"));
    }
}
