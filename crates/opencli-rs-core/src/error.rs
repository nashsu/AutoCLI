use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("[browser] {message}")]
    BrowserConnect {
        message: String,
        suggestions: Vec<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("[adapter] {message}")]
    AdapterLoad {
        message: String,
        suggestions: Vec<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("[command] {message}")]
    CommandExecution {
        message: String,
        suggestions: Vec<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("[config] {message}")]
    Config {
        message: String,
        suggestions: Vec<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("[auth] {message}")]
    AuthRequired {
        message: String,
        suggestions: Vec<String>,
    },

    #[error("[timeout] {message}")]
    Timeout {
        message: String,
        suggestions: Vec<String>,
    },

    #[error("[argument] {message}")]
    Argument {
        message: String,
        suggestions: Vec<String>,
    },

    #[error("[empty] {message}")]
    EmptyResult {
        message: String,
        suggestions: Vec<String>,
    },

    #[error("[selector] {message}")]
    Selector {
        message: String,
        suggestions: Vec<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("[pipeline] {message}")]
    Pipeline {
        message: String,
        suggestions: Vec<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("[io] {0}")]
    Io(#[from] std::io::Error),

    #[error("[json] {0}")]
    Json(#[from] serde_json::Error),

    #[error("[yaml] {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("[http] {message}")]
    Http {
        message: String,
        suggestions: Vec<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl CliError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::BrowserConnect { .. } => "BROWSER_CONNECT",
            Self::AdapterLoad { .. } => "ADAPTER_LOAD",
            Self::CommandExecution { .. } => "COMMAND_EXECUTION",
            Self::Config { .. } => "CONFIG",
            Self::AuthRequired { .. } => "AUTH_REQUIRED",
            Self::Timeout { .. } => "TIMEOUT",
            Self::Argument { .. } => "ARGUMENT",
            Self::EmptyResult { .. } => "EMPTY_RESULT",
            Self::Selector { .. } => "SELECTOR",
            Self::Pipeline { .. } => "PIPELINE",
            Self::Io(_) => "IO",
            Self::Json(_) => "JSON",
            Self::Yaml(_) => "YAML",
            Self::Http { .. } => "HTTP",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::BrowserConnect { .. } => "🌐",
            Self::AdapterLoad { .. } => "🔌",
            Self::CommandExecution { .. } => "⚡",
            Self::Config { .. } => "⚙️",
            Self::AuthRequired { .. } => "🔒",
            Self::Timeout { .. } => "⏱️",
            Self::Argument { .. } => "📝",
            Self::EmptyResult { .. } => "📭",
            Self::Selector { .. } => "🎯",
            Self::Pipeline { .. } => "🔧",
            Self::Io(_) => "💾",
            Self::Json(_) => "📄",
            Self::Yaml(_) => "📄",
            Self::Http { .. } => "🌍",
        }
    }

    pub fn suggestions(&self) -> &[String] {
        match self {
            Self::BrowserConnect { suggestions, .. }
            | Self::AdapterLoad { suggestions, .. }
            | Self::CommandExecution { suggestions, .. }
            | Self::Config { suggestions, .. }
            | Self::AuthRequired { suggestions, .. }
            | Self::Timeout { suggestions, .. }
            | Self::Argument { suggestions, .. }
            | Self::EmptyResult { suggestions, .. }
            | Self::Selector { suggestions, .. }
            | Self::Pipeline { suggestions, .. }
            | Self::Http { suggestions, .. } => suggestions,
            Self::Io(_) | Self::Json(_) | Self::Yaml(_) => &[],
        }
    }

    // Convenience constructors

    pub fn browser_connect(msg: impl Into<String>) -> Self {
        Self::BrowserConnect {
            message: msg.into(),
            suggestions: vec![],
            source: None,
        }
    }

    pub fn argument(msg: impl Into<String>) -> Self {
        Self::Argument {
            message: msg.into(),
            suggestions: vec![],
        }
    }

    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::Timeout {
            message: msg.into(),
            suggestions: vec![],
        }
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config {
            message: msg.into(),
            suggestions: vec![],
            source: None,
        }
    }

    pub fn auth_required(msg: impl Into<String>) -> Self {
        Self::AuthRequired {
            message: msg.into(),
            suggestions: vec![],
        }
    }

    pub fn empty_result(msg: impl Into<String>) -> Self {
        Self::EmptyResult {
            message: msg.into(),
            suggestions: vec![],
        }
    }

    pub fn command_execution(msg: impl Into<String>) -> Self {
        Self::CommandExecution {
            message: msg.into(),
            suggestions: vec![],
            source: None,
        }
    }

    pub fn pipeline(msg: impl Into<String>) -> Self {
        Self::Pipeline {
            message: msg.into(),
            suggestions: vec![],
            source: None,
        }
    }
}
