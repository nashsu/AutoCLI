/// Returns JavaScript that patches common browser automation detection vectors.
///
/// This script should be injected early into pages to hide signs of automation.
pub fn stealth_js() -> &'static str {
    r#"(() => {
  // Remove webdriver property
  Object.defineProperty(navigator, 'webdriver', {
    get: () => undefined,
  });

  // Override navigator.plugins to look non-empty
  Object.defineProperty(navigator, 'plugins', {
    get: () => [1, 2, 3, 4, 5],
  });

  // Override navigator.languages
  Object.defineProperty(navigator, 'languages', {
    get: () => ['en-US', 'en'],
  });

  // Prevent detection via permissions API
  if (navigator.permissions) {
    const origQuery = navigator.permissions.query;
    navigator.permissions.query = (params) => {
      if (params.name === 'notifications') {
        return Promise.resolve({ state: Notification.permission });
      }
      return origQuery.call(navigator.permissions, params);
    };
  }

  // Hide chrome.runtime if it's a sign of automation
  if (window.chrome && !window.chrome.runtime) {
    window.chrome.runtime = {};
  }

  // Override iframe contentWindow access detection
  const origDesc = Object.getOwnPropertyDescriptor(HTMLIFrameElement.prototype, 'contentWindow');
  if (origDesc) {
    Object.defineProperty(HTMLIFrameElement.prototype, 'contentWindow', {
      get: function() {
        return origDesc.get.call(this);
      },
    });
  }
})()"#
}
