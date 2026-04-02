use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// What to test: a URL, a file path, a service name, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestTarget {
    /// Human-readable name for this target (e.g. "login-flow").
    pub name: String,
    /// The kind of target: web-url, api-endpoint, mobile-app, service.
    pub kind: TargetKind,
    /// The locator: a URL, file path, or service identifier.
    pub locator: String,
    /// Optional environment variables passed to runners.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// What kind of system is being tested.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TargetKind {
    WebUrl,
    ApiEndpoint,
    MobileApp,
    Service,
}

impl std::fmt::Display for TargetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WebUrl => write!(f, "web-url"),
            Self::ApiEndpoint => write!(f, "api-endpoint"),
            Self::MobileApp => write!(f, "mobile-app"),
            Self::Service => write!(f, "service"),
        }
    }
}

impl std::str::FromStr for TargetKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "web-url" | "weburl" | "web_url" | "url" => Ok(Self::WebUrl),
            "api-endpoint" | "api" | "api_endpoint" => Ok(Self::ApiEndpoint),
            "mobile-app" | "mobile" | "mobile_app" => Ok(Self::MobileApp),
            "service" | "svc" => Ok(Self::Service),
            other => Err(format!("unknown target kind: {other}")),
        }
    }
}
