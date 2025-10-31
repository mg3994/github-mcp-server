use tracing::{info, warn};
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};
use crate::config::ServerConfig;
use crate::error::GitHubMcpError;

pub fn init_logging(config: &ServerConfig) -> Result<(), GitHubMcpError> {
    let log_level = parse_log_level(&config.log_level)?;
    
    // Create environment filter with default level
    let env_filter = EnvFilter::builder()
        .with_default_directive(log_level.into())
        .from_env_lossy()
        // Add specific filters for noisy crates
        .add_directive("reqwest=warn".parse().unwrap())
        .add_directive("hyper=warn".parse().unwrap())
        .add_directive("rustls=warn".parse().unwrap());
    
    // Configure the formatter
    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_span_events(if config.enable_request_logging {
            FmtSpan::NEW | FmtSpan::CLOSE
        } else {
            FmtSpan::NONE
        })
        .compact();
    
    // Initialize the subscriber
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
    
    info!(
        version = env!("CARGO_PKG_VERSION"),
        log_level = %config.log_level,
        github_api_url = %config.github_api_url,
        github_enterprise = config.github_enterprise,
        "GitHub MCP Server initialized"
    );
    
    if config.github_enterprise {
        info!("GitHub Enterprise mode detected");
    }
    
    if config.enable_request_logging {
        warn!("Request logging is enabled - this may log sensitive information");
    }
    
    Ok(())
}

fn parse_log_level(level: &str) -> Result<LevelFilter, GitHubMcpError> {
    match level.to_lowercase().as_str() {
        "trace" => Ok(LevelFilter::TRACE),
        "debug" => Ok(LevelFilter::DEBUG),
        "info" => Ok(LevelFilter::INFO),
        "warn" => Ok(LevelFilter::WARN),
        "error" => Ok(LevelFilter::ERROR),
        _ => Err(GitHubMcpError::ConfigError(
            format!("Invalid log level '{}': must be one of trace, debug, info, warn, error", level)
        )),
    }
}

// Structured logging macros for consistent formatting
#[macro_export]
macro_rules! log_request {
    ($method:expr, $url:expr, $status:expr) => {
        if $crate::config::ServerConfig::default().enable_request_logging {
            tracing::debug!(
                method = %$method,
                url = %$url,
                status = %$status,
                "HTTP request completed"
            );
        }
    };
}

#[macro_export]
macro_rules! log_github_api_call {
    ($endpoint:expr, $method:expr) => {
        tracing::debug!(
            endpoint = %$endpoint,
            method = %$method,
            "GitHub API call"
        );
    };
}

#[macro_export]
macro_rules! log_mcp_tool_call {
    ($tool_name:expr, $duration_ms:expr) => {
        tracing::info!(
            tool = %$tool_name,
            duration_ms = %$duration_ms,
            "MCP tool executed"
        );
    };
}

#[macro_export]
macro_rules! log_auth_event {
    ($event:expr, $user:expr) => {
        tracing::info!(
            event = %$event,
            user = %$user,
            "Authentication event"
        );
    };
}

#[macro_export]
macro_rules! log_rate_limit {
    ($remaining:expr, $reset_time:expr) => {
        tracing::warn!(
            remaining = %$remaining,
            reset_time = %$reset_time,
            "GitHub API rate limit status"
        );
    };
}

// Helper function to sanitize sensitive data in logs
pub fn sanitize_token(token: &str) -> String {
    if token.len() <= 8 {
        "*".repeat(token.len())
    } else {
        format!("{}***{}", &token[..4], &token[token.len()-4..])
    }
}

// Helper function to sanitize URLs (remove tokens from query params)
pub fn sanitize_url(url: &str) -> String {
    if let Ok(mut parsed_url) = url::Url::parse(url) {
        // Remove sensitive query parameters
        let query_pairs: Vec<(String, String)> = parsed_url
            .query_pairs()
            .filter(|(key, _)| !key.contains("token") && !key.contains("access_token"))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        
        parsed_url.query_pairs_mut().clear();
        for (key, value) in query_pairs {
            parsed_url.query_pairs_mut().append_pair(&key, &value);
        }
        
        parsed_url.to_string()
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sanitize_token() {
        assert_eq!(sanitize_token("ghp_1234567890abcdef"), "ghp_***cdef");
        assert_eq!(sanitize_token("short"), "*****");
        assert_eq!(sanitize_token(""), "");
    }
    
    #[test]
    fn test_sanitize_url() {
        let url_with_token = "https://api.github.com/user?access_token=secret123&other=value";
        let sanitized = sanitize_url(url_with_token);
        assert!(!sanitized.contains("secret123"));
        assert!(sanitized.contains("other=value"));
    }
    
    #[test]
    fn test_parse_log_level() {
        assert!(matches!(parse_log_level("info"), Ok(LevelFilter::INFO)));
        assert!(matches!(parse_log_level("DEBUG"), Ok(LevelFilter::DEBUG)));
        assert!(parse_log_level("invalid").is_err());
    }
}