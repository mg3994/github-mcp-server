use std::time::Duration;
use url::Url;
use crate::error::GitHubMcpError;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub github_api_url: String,
    pub request_timeout: Duration,
    pub log_level: String,
    pub max_retries: u32,
    pub rate_limit_buffer: u32,
    pub user_agent: String,
    pub max_concurrent_requests: u32,
    pub enable_request_logging: bool,
    pub github_enterprise: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            github_api_url: "https://api.github.com".to_string(),
            request_timeout: Duration::from_secs(30),
            log_level: "info".to_string(),
            max_retries: 3,
            rate_limit_buffer: 10,
            user_agent: format!("github-mcp-server/{}", env!("CARGO_PKG_VERSION")),
            max_concurrent_requests: 10,
            enable_request_logging: false,
            github_enterprise: false,
        }
    }
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, GitHubMcpError> {
        let mut config = Self::default();
        
        // GitHub API URL
        if let Ok(url) = std::env::var("GITHUB_API_URL") {
            config.github_api_url = url;
        }
        
        // Request timeout
        if let Ok(timeout_str) = std::env::var("REQUEST_TIMEOUT") {
            let timeout = timeout_str.parse::<u64>()
                .map_err(|_| GitHubMcpError::ConfigError("Invalid REQUEST_TIMEOUT: must be a positive integer".to_string()))?;
            config.request_timeout = Duration::from_secs(timeout);
        }
        
        // Log level
        if let Ok(level) = std::env::var("LOG_LEVEL") {
            config.log_level = level.to_lowercase();
        }
        
        // Max retries
        if let Ok(retries_str) = std::env::var("MAX_RETRIES") {
            config.max_retries = retries_str.parse::<u32>()
                .map_err(|_| GitHubMcpError::ConfigError("Invalid MAX_RETRIES: must be a positive integer".to_string()))?;
        }
        
        // Rate limit buffer
        if let Ok(buffer_str) = std::env::var("RATE_LIMIT_BUFFER") {
            config.rate_limit_buffer = buffer_str.parse::<u32>()
                .map_err(|_| GitHubMcpError::ConfigError("Invalid RATE_LIMIT_BUFFER: must be a positive integer".to_string()))?;
        }
        
        // User agent
        if let Ok(user_agent) = std::env::var("USER_AGENT") {
            config.user_agent = user_agent;
        }
        
        // Max concurrent requests
        if let Ok(max_concurrent_str) = std::env::var("MAX_CONCURRENT_REQUESTS") {
            config.max_concurrent_requests = max_concurrent_str.parse::<u32>()
                .map_err(|_| GitHubMcpError::ConfigError("Invalid MAX_CONCURRENT_REQUESTS: must be a positive integer".to_string()))?;
        }
        
        // Enable request logging
        if let Ok(enable_logging_str) = std::env::var("ENABLE_REQUEST_LOGGING") {
            config.enable_request_logging = enable_logging_str.parse::<bool>()
                .unwrap_or_else(|_| enable_logging_str.to_lowercase() == "true" || enable_logging_str == "1");
        }
        
        // Detect GitHub Enterprise
        config.github_enterprise = !config.github_api_url.starts_with("https://api.github.com");
        
        config.validate()?;
        Ok(config)
    }
    
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_github_api_url(mut self, url: String) -> Self {
        self.github_api_url = url;
        self
    }
    
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }
    
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }
    
    pub fn is_github_enterprise(&self) -> bool {
        self.github_enterprise
    }
    
    pub fn get_api_version(&self) -> &str {
        if self.github_enterprise {
            "v3"
        } else {
            "v3"
        }
    }
    
    fn validate(&self) -> Result<(), GitHubMcpError> {
        // Validate GitHub API URL
        if self.github_api_url.is_empty() {
            return Err(GitHubMcpError::ConfigError("GitHub API URL cannot be empty".to_string()));
        }
        
        // Validate URL format
        Url::parse(&self.github_api_url)
            .map_err(|e| GitHubMcpError::ConfigError(format!("Invalid GitHub API URL: {}", e)))?;
        
        // Validate request timeout
        if self.request_timeout.as_secs() == 0 {
            return Err(GitHubMcpError::ConfigError("Request timeout must be greater than 0".to_string()));
        }
        
        if self.request_timeout.as_secs() > 300 {
            return Err(GitHubMcpError::ConfigError("Request timeout cannot exceed 300 seconds".to_string()));
        }
        
        // Validate max retries
        if self.max_retries > 10 {
            return Err(GitHubMcpError::ConfigError("Max retries cannot exceed 10".to_string()));
        }
        
        // Validate rate limit buffer
        if self.rate_limit_buffer > 50 {
            return Err(GitHubMcpError::ConfigError("Rate limit buffer cannot exceed 50%".to_string()));
        }
        
        // Validate log level
        match self.log_level.as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {},
            _ => return Err(GitHubMcpError::ConfigError(
                "Invalid log level: must be one of trace, debug, info, warn, error".to_string()
            )),
        }
        
        // Validate user agent
        if self.user_agent.is_empty() {
            return Err(GitHubMcpError::ConfigError("User agent cannot be empty".to_string()));
        }
        
        // Validate max concurrent requests
        if self.max_concurrent_requests == 0 {
            return Err(GitHubMcpError::ConfigError("Max concurrent requests must be greater than 0".to_string()));
        }
        
        if self.max_concurrent_requests > 100 {
            return Err(GitHubMcpError::ConfigError("Max concurrent requests cannot exceed 100".to_string()));
        }
        
        Ok(())
    }
}