use reqwest::{Client, Method, Response, header::{HeaderMap, HeaderValue}};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, warn, info, error};
use serde_json::Value;

use crate::config::ServerConfig;
use crate::error::GitHubMcpError;
use crate::models::*;
use crate::{log_github_api_call, log_rate_limit};

#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: u32,
    pub remaining: u32,
    pub reset_time: u64,
    pub used: u32,
}

pub struct GitHubClient {
    client: Client,
    base_url: String,
    max_retries: u32,
    user_agent: String,
    enable_request_logging: bool,
}

impl GitHubClient {
    pub fn new(config: &ServerConfig) -> Result<Self, GitHubMcpError> {
        let mut default_headers = HeaderMap::new();
        default_headers.insert("Accept", HeaderValue::from_static("application/vnd.github.v3+json"));
        default_headers.insert("X-GitHub-Api-Version", HeaderValue::from_static("2022-11-28"));
        
        let client = Client::builder()
            .timeout(config.request_timeout)
            .user_agent(&config.user_agent)
            .default_headers(default_headers)
            .build()
            .map_err(|e| GitHubMcpError::NetworkError(e.to_string()))?;
        
        Ok(Self {
            client,
            base_url: config.github_api_url.clone(),
            max_retries: config.max_retries,
            user_agent: config.user_agent.clone(),
            enable_request_logging: config.enable_request_logging,
        })
    }
    
    pub async fn authenticate(&self, token: &str) -> Result<User, GitHubMcpError> {
        log_github_api_call!("/user", "GET");
        let url = format!("{}/user", self.base_url);
        
        let response = self.make_request(Method::GET, &url, token, None).await?;
        let user: User = response.json().await?;
        
        info!("Successfully authenticated as user: {}", user.login);
        Ok(user)
    }
    
    pub async fn get_rate_limit(&self, token: &str) -> Result<RateLimitInfo, GitHubMcpError> {
        log_github_api_call!("/rate_limit", "GET");
        let url = format!("{}/rate_limit", self.base_url);
        
        let response = self.make_request(Method::GET, &url, token, None).await?;
        let rate_limit_data: Value = response.json().await?;
        
        let core = rate_limit_data["rate"].as_object()
            .ok_or_else(|| GitHubMcpError::SerializationError("Invalid rate limit response".to_string()))?;
        
        let rate_limit = RateLimitInfo {
            limit: core["limit"].as_u64().unwrap_or(0) as u32,
            remaining: core["remaining"].as_u64().unwrap_or(0) as u32,
            reset_time: core["reset"].as_u64().unwrap_or(0),
            used: core["used"].as_u64().unwrap_or(0) as u32,
        };
        
        log_rate_limit!(rate_limit.remaining, rate_limit.reset_time);
        Ok(rate_limit)
    }
    
    pub async fn get(&self, endpoint: &str, token: &str) -> Result<Response, GitHubMcpError> {
        let url = format!("{}{}", self.base_url, endpoint);
        self.make_request(Method::GET, &url, token, None).await
    }
    
    pub async fn post(&self, endpoint: &str, token: &str, body: Option<Value>) -> Result<Response, GitHubMcpError> {
        let url = format!("{}{}", self.base_url, endpoint);
        self.make_request(Method::POST, &url, token, body).await
    }
    
    pub async fn patch(&self, endpoint: &str, token: &str, body: Option<Value>) -> Result<Response, GitHubMcpError> {
        let url = format!("{}{}", self.base_url, endpoint);
        self.make_request(Method::PATCH, &url, token, body).await
    }
    
    pub async fn put(&self, endpoint: &str, token: &str, body: Option<Value>) -> Result<Response, GitHubMcpError> {
        let url = format!("{}{}", self.base_url, endpoint);
        self.make_request(Method::PUT, &url, token, body).await
    }
    
    pub async fn delete(&self, endpoint: &str, token: &str) -> Result<Response, GitHubMcpError> {
        let url = format!("{}{}", self.base_url, endpoint);
        self.make_request(Method::DELETE, &url, token, None).await
    }
    
    async fn make_request(&self, method: Method, url: &str, token: &str, body: Option<Value>) -> Result<Response, GitHubMcpError> {
        let mut attempts = 0;
        let mut delay = Duration::from_millis(100);
        
        loop {
            let mut request_builder = self.client
                .request(method.clone(), url)
                .header("Authorization", format!("Bearer {}", token));
            
            if let Some(ref body_data) = body {
                request_builder = request_builder
                    .header("Content-Type", "application/json")
                    .json(body_data);
            }
            
            let start_time = SystemTime::now();
            let response = request_builder.send().await?;
            let duration = start_time.elapsed().unwrap_or_default();
            
            // Log rate limit information from headers
            self.log_rate_limit_headers(&response);
            
            if self.enable_request_logging {
                debug!(
                    method = %method,
                    url = %crate::logging::sanitize_url(url),
                    status = %response.status(),
                    duration_ms = %duration.as_millis(),
                    "GitHub API request completed"
                );
            }
            
            match response.status().as_u16() {
                200..=299 => return Ok(response),
                401 => {
                    error!("GitHub authentication failed - invalid or expired token");
                    return Err(GitHubMcpError::AuthenticationError("Invalid or expired token".to_string()));
                },
                403 => {
                    // Check if this is a rate limit (GitHub returns 403 for rate limits)
                    if let Some(remaining) = response.headers().get("x-ratelimit-remaining") {
                        if let Ok(remaining_str) = remaining.to_str() {
                            if let Ok(remaining_count) = remaining_str.parse::<u32>() {
                                if remaining_count == 0 {
                                    let reset_time = response.headers()
                                        .get("x-ratelimit-reset")
                                        .and_then(|h| h.to_str().ok())
                                        .and_then(|s| s.parse::<u64>().ok())
                                        .unwrap_or_else(|| {
                                            SystemTime::now()
                                                .duration_since(UNIX_EPOCH)
                                                .unwrap_or_default()
                                                .as_secs() + 3600
                                        });
                                    
                                    let retry_after = reset_time.saturating_sub(
                                        SystemTime::now()
                                            .duration_since(UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_secs()
                                    );
                                    
                                    warn!("GitHub API rate limit exceeded, reset at {}", reset_time);
                                    return Err(GitHubMcpError::RateLimitError { retry_after });
                                }
                            }
                        }
                    }
                    
                    // Check for explicit retry-after header
                    if let Some(retry_after) = response.headers().get("retry-after") {
                        if let Ok(retry_after_str) = retry_after.to_str() {
                            if let Ok(retry_after_secs) = retry_after_str.parse::<u64>() {
                                return Err(GitHubMcpError::RateLimitError { retry_after: retry_after_secs });
                            }
                        }
                    }
                    
                    let error_text = response.text().await.unwrap_or_default();
                    error!("GitHub API access denied: {}", error_text);
                    return Err(GitHubMcpError::PermissionError(format!("Access denied: {}", error_text)));
                },
                429 => {
                    let retry_after = response.headers()
                        .get("retry-after")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(60);
                    
                    warn!("GitHub API rate limit (429), retry after {} seconds", retry_after);
                    return Err(GitHubMcpError::RateLimitError { retry_after });
                },
                500..=599 => {
                    attempts += 1;
                    if attempts >= self.max_retries {
                        let status = response.status().as_u16();
                        let error_text = response.text().await.unwrap_or_default();
                        error!("GitHub API server error after {} attempts: {} - {}", attempts, status, error_text);
                        return Err(GitHubMcpError::GitHubApiError {
                            status,
                            message: error_text,
                        });
                    }
                    
                    warn!("GitHub API server error {}, retrying in {:?} (attempt {}/{})", 
                          response.status(), delay, attempts, self.max_retries);
                    
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, Duration::from_secs(30)); // Cap at 30 seconds
                },
                status => {
                    let error_text = response.text().await.unwrap_or_default();
                    error!("GitHub API error {}: {}", status, error_text);
                    return Err(GitHubMcpError::GitHubApiError {
                        status,
                        message: error_text,
                    });
                }
            }
        }
    }
    
    fn log_rate_limit_headers(&self, response: &Response) {
        if let (Some(limit), Some(remaining), Some(reset)) = (
            response.headers().get("x-ratelimit-limit"),
            response.headers().get("x-ratelimit-remaining"),
            response.headers().get("x-ratelimit-reset")
        ) {
            if let (Ok(limit_str), Ok(remaining_str), Ok(reset_str)) = (
                limit.to_str(),
                remaining.to_str(),
                reset.to_str()
            ) {
                if let (Ok(remaining_count), Ok(reset_time)) = (
                    remaining_str.parse::<u32>(),
                    reset_str.parse::<u64>()
                ) {
                    // Log warning if rate limit is getting low
                    if remaining_count < 100 {
                        warn!(
                            remaining = remaining_count,
                            reset_time = reset_time,
                            limit = limit_str,
                            "GitHub API rate limit running low"
                        );
                    } else {
                        debug!(
                            remaining = remaining_count,
                            reset_time = reset_time,
                            limit = limit_str,
                            "GitHub API rate limit status"
                        );
                    }
                }
            }
        }
    }
    
    pub fn get_base_url(&self) -> &str {
        &self.base_url
    }
    
    pub fn get_user_agent(&self) -> &str {
        &self.user_agent
    }
    
    // Repository operations
    pub async fn list_repositories(&self, token: &str, params: &ListReposParams) -> Result<Vec<Repository>, GitHubMcpError> {
        log_github_api_call!("/user/repos", "GET");
        
        let mut query_params = Vec::new();
        
        if let Some(visibility) = &params.visibility {
            query_params.push(format!("visibility={}", visibility));
        }
        if let Some(sort) = &params.sort {
            query_params.push(format!("sort={}", sort));
        }
        if let Some(direction) = &params.direction {
            query_params.push(format!("direction={}", direction));
        }
        if let Some(per_page) = params.per_page {
            query_params.push(format!("per_page={}", per_page));
        }
        if let Some(page) = params.page {
            query_params.push(format!("page={}", page));
        }
        
        let query_string = if query_params.is_empty() {
            String::new()
        } else {
            format!("?{}", query_params.join("&"))
        };
        
        let endpoint = format!("/user/repos{}", query_string);
        let response = self.get(&endpoint, token).await?;
        let repositories: Vec<Repository> = response.json().await?;
        
        info!("Retrieved {} repositories", repositories.len());
        Ok(repositories)
    }
    
    pub async fn search_repositories(&self, token: &str, query: &str, sort: Option<&str>, order: Option<&str>, per_page: Option<u32>, page: Option<u32>) -> Result<Vec<Repository>, GitHubMcpError> {
        log_github_api_call!("/search/repositories", "GET");
        
        let mut query_params = vec![format!("q={}", urlencoding::encode(query))];
        
        if let Some(sort_param) = sort {
            query_params.push(format!("sort={}", sort_param));
        }
        if let Some(order_param) = order {
            query_params.push(format!("order={}", order_param));
        }
        if let Some(per_page) = per_page {
            query_params.push(format!("per_page={}", per_page));
        }
        if let Some(page) = page {
            query_params.push(format!("page={}", page));
        }
        
        let query_string = query_params.join("&");
        let endpoint = format!("/search/repositories?{}", query_string);
        
        let response = self.get(&endpoint, token).await?;
        let search_result: Value = response.json().await?;
        
        let repositories = search_result["items"]
            .as_array()
            .ok_or_else(|| GitHubMcpError::SerializationError("Invalid search response format".to_string()))?
            .iter()
            .map(|item| serde_json::from_value(item.clone()))
            .collect::<Result<Vec<Repository>, _>>()?;
        
        info!("Found {} repositories matching query: {}", repositories.len(), query);
        Ok(repositories)
    }
    
    pub async fn get_repository(&self, token: &str, owner: &str, repo: &str) -> Result<Repository, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}", owner, repo), "GET");
        
        let endpoint = format!("/repos/{}/{}", owner, repo);
        let response = self.get(&endpoint, token).await?;
        let repository: Repository = response.json().await?;
        
        debug!("Retrieved repository: {}/{}", owner, repo);
        Ok(repository)
    }
    
    pub async fn get_file_content(&self, token: &str, owner: &str, repo: &str, path: &str, ref_name: Option<&str>) -> Result<FileContent, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/contents/{}", owner, repo, path), "GET");
        
        let mut endpoint = format!("/repos/{}/{}/contents/{}", owner, repo, urlencoding::encode(path));
        
        if let Some(ref_val) = ref_name {
            endpoint.push_str(&format!("?ref={}", urlencoding::encode(ref_val)));
        }
        
        let response = self.get(&endpoint, token).await?;
        let file_content: FileContent = response.json().await?;
        
        debug!("Retrieved file content: {}/{}/{}", owner, repo, path);
        Ok(file_content)
    }
    
    pub async fn list_directory(&self, token: &str, owner: &str, repo: &str, path: &str, ref_name: Option<&str>) -> Result<Vec<DirectoryItem>, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/contents/{}", owner, repo, path), "GET");
        
        let encoded_path = if path.is_empty() { 
            String::new() 
        } else { 
            urlencoding::encode(path).to_string()
        };
        
        let mut endpoint = format!("/repos/{}/{}/contents/{}", owner, repo, encoded_path);
        
        if let Some(ref_val) = ref_name {
            endpoint.push_str(&format!("?ref={}", urlencoding::encode(ref_val)));
        }
        
        let response = self.get(&endpoint, token).await?;
        let directory_items: Vec<DirectoryItem> = response.json().await?;
        
        debug!("Listed {} items in directory: {}/{}/{}", directory_items.len(), owner, repo, path);
        Ok(directory_items)
    }
    
    pub async fn get_repository_branches(&self, token: &str, owner: &str, repo: &str, per_page: Option<u32>, page: Option<u32>) -> Result<Vec<Branch>, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/branches", owner, repo), "GET");
        
        let mut query_params = Vec::new();
        
        if let Some(per_page) = per_page {
            query_params.push(format!("per_page={}", per_page));
        }
        if let Some(page) = page {
            query_params.push(format!("page={}", page));
        }
        
        let query_string = if query_params.is_empty() {
            String::new()
        } else {
            format!("?{}", query_params.join("&"))
        };
        
        let endpoint = format!("/repos/{}/{}/branches{}", owner, repo, query_string);
        let response = self.get(&endpoint, token).await?;
        let branches: Vec<Branch> = response.json().await?;
        
        debug!("Retrieved {} branches for repository: {}/{}", branches.len(), owner, repo);
        Ok(branches)
    }
    
    pub async fn get_repository_commits(&self, token: &str, owner: &str, repo: &str, sha: Option<&str>, path: Option<&str>, per_page: Option<u32>, page: Option<u32>) -> Result<Vec<Commit>, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/commits", owner, repo), "GET");
        
        let mut query_params = Vec::new();
        
        if let Some(sha_val) = sha {
            query_params.push(format!("sha={}", urlencoding::encode(sha_val)));
        }
        if let Some(path_val) = path {
            query_params.push(format!("path={}", urlencoding::encode(path_val)));
        }
        if let Some(per_page) = per_page {
            query_params.push(format!("per_page={}", per_page));
        }
        if let Some(page) = page {
            query_params.push(format!("page={}", page));
        }
        
        let query_string = if query_params.is_empty() {
            String::new()
        } else {
            format!("?{}", query_params.join("&"))
        };
        
        let endpoint = format!("/repos/{}/{}/commits{}", owner, repo, query_string);
        let response = self.get(&endpoint, token).await?;
        let commits: Vec<Commit> = response.json().await?;
        
        debug!("Retrieved {} commits for repository: {}/{}", commits.len(), owner, repo);
        Ok(commits)
    }
    
    pub async fn get_repository_tags(&self, token: &str, owner: &str, repo: &str, per_page: Option<u32>, page: Option<u32>) -> Result<Vec<GitReference>, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/tags", owner, repo), "GET");
        
        let mut query_params = Vec::new();
        
        if let Some(per_page) = per_page {
            query_params.push(format!("per_page={}", per_page));
        }
        if let Some(page) = page {
            query_params.push(format!("page={}", page));
        }
        
        let query_string = if query_params.is_empty() {
            String::new()
        } else {
            format!("?{}", query_params.join("&"))
        };
        
        let endpoint = format!("/repos/{}/{}/tags{}", owner, repo, query_string);
        let response = self.get(&endpoint, token).await?;
        let tags: Vec<GitReference> = response.json().await?;
        
        debug!("Retrieved {} tags for repository: {}/{}", tags.len(), owner, repo);
        Ok(tags)
    }
    
    // Issue management operations
    pub async fn list_issues(&self, token: &str, owner: &str, repo: &str, params: &ListIssuesParams) -> Result<Vec<Issue>, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/issues", owner, repo), "GET");
        
        let mut query_params = Vec::new();
        
        if let Some(state) = &params.state {
            query_params.push(format!("state={}", state));
        }
        if let Some(labels) = &params.labels {
            query_params.push(format!("labels={}", urlencoding::encode(labels)));
        }
        if let Some(assignee) = &params.assignee {
            query_params.push(format!("assignee={}", urlencoding::encode(assignee)));
        }
        if let Some(sort) = &params.sort {
            query_params.push(format!("sort={}", sort));
        }
        if let Some(direction) = &params.direction {
            query_params.push(format!("direction={}", direction));
        }
        if let Some(per_page) = params.per_page {
            query_params.push(format!("per_page={}", per_page));
        }
        if let Some(page) = params.page {
            query_params.push(format!("page={}", page));
        }
        
        let query_string = if query_params.is_empty() {
            String::new()
        } else {
            format!("?{}", query_params.join("&"))
        };
        
        let endpoint = format!("/repos/{}/{}/issues{}", owner, repo, query_string);
        let response = self.get(&endpoint, token).await?;
        let issues: Vec<Issue> = response.json().await?;
        
        info!("Retrieved {} issues for repository: {}/{}", issues.len(), owner, repo);
        Ok(issues)
    }
    
    pub async fn get_issue(&self, token: &str, owner: &str, repo: &str, issue_number: u32) -> Result<Issue, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/issues/{}", owner, repo, issue_number), "GET");
        
        let endpoint = format!("/repos/{}/{}/issues/{}", owner, repo, issue_number);
        let response = self.get(&endpoint, token).await?;
        let issue: Issue = response.json().await?;
        
        debug!("Retrieved issue #{} from repository: {}/{}", issue_number, owner, repo);
        Ok(issue)
    }
    
    pub async fn create_issue(&self, token: &str, owner: &str, repo: &str, request: &CreateIssueRequest) -> Result<Issue, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/issues", owner, repo), "POST");
        
        let endpoint = format!("/repos/{}/{}/issues", owner, repo);
        let body = serde_json::to_value(request)?;
        let response = self.post(&endpoint, token, Some(body)).await?;
        let issue: Issue = response.json().await?;
        
        info!("Created issue #{} in repository: {}/{}", issue.number, owner, repo);
        Ok(issue)
    }
    
    pub async fn update_issue(&self, token: &str, owner: &str, repo: &str, issue_number: u32, request: &UpdateIssueRequest) -> Result<Issue, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/issues/{}", owner, repo, issue_number), "PATCH");
        
        let endpoint = format!("/repos/{}/{}/issues/{}", owner, repo, issue_number);
        let body = serde_json::to_value(request)?;
        let response = self.patch(&endpoint, token, Some(body)).await?;
        let issue: Issue = response.json().await?;
        
        info!("Updated issue #{} in repository: {}/{}", issue_number, owner, repo);
        Ok(issue)
    }
    
    pub async fn close_issue(&self, token: &str, owner: &str, repo: &str, issue_number: u32) -> Result<Issue, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/issues/{}", owner, repo, issue_number), "PATCH");
        
        let update_request = UpdateIssueRequest {
            title: None,
            body: None,
            state: Some(IssueState::Closed),
            labels: None,
            assignees: None,
        };
        
        self.update_issue(token, owner, repo, issue_number, &update_request).await
    }
    
    pub async fn reopen_issue(&self, token: &str, owner: &str, repo: &str, issue_number: u32) -> Result<Issue, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/issues/{}", owner, repo, issue_number), "PATCH");
        
        let update_request = UpdateIssueRequest {
            title: None,
            body: None,
            state: Some(IssueState::Open),
            labels: None,
            assignees: None,
        };
        
        self.update_issue(token, owner, repo, issue_number, &update_request).await
    }
    
    pub async fn add_labels_to_issue(&self, token: &str, owner: &str, repo: &str, issue_number: u32, labels: Vec<String>) -> Result<Vec<Label>, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/issues/{}/labels", owner, repo, issue_number), "POST");
        
        let endpoint = format!("/repos/{}/{}/issues/{}/labels", owner, repo, issue_number);
        let body = serde_json::json!({ "labels": labels });
        let response = self.post(&endpoint, token, Some(body)).await?;
        let updated_labels: Vec<Label> = response.json().await?;
        
        debug!("Added {} labels to issue #{} in repository: {}/{}", labels.len(), issue_number, owner, repo);
        Ok(updated_labels)
    }
    
    pub async fn remove_label_from_issue(&self, token: &str, owner: &str, repo: &str, issue_number: u32, label: &str) -> Result<(), GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/issues/{}/labels/{}", owner, repo, issue_number, label), "DELETE");
        
        let endpoint = format!("/repos/{}/{}/issues/{}/labels/{}", owner, repo, issue_number, urlencoding::encode(label));
        let _response = self.delete(&endpoint, token).await?;
        
        debug!("Removed label '{}' from issue #{} in repository: {}/{}", label, issue_number, owner, repo);
        Ok(())
    }
    
    pub async fn assign_issue(&self, token: &str, owner: &str, repo: &str, issue_number: u32, assignees: Vec<String>) -> Result<Issue, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/issues/{}/assignees", owner, repo, issue_number), "POST");
        
        let endpoint = format!("/repos/{}/{}/issues/{}/assignees", owner, repo, issue_number);
        let body = serde_json::json!({ "assignees": assignees });
        let response = self.post(&endpoint, token, Some(body)).await?;
        let issue: Issue = response.json().await?;
        
        debug!("Assigned {} users to issue #{} in repository: {}/{}", assignees.len(), issue_number, owner, repo);
        Ok(issue)
    }
    
    pub async fn unassign_issue(&self, token: &str, owner: &str, repo: &str, issue_number: u32, assignees: Vec<String>) -> Result<Issue, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/issues/{}/assignees", owner, repo, issue_number), "DELETE");
        
        let endpoint = format!("/repos/{}/{}/issues/{}/assignees", owner, repo, issue_number);
        let body = serde_json::json!({ "assignees": assignees });
        let response = self.make_request(Method::DELETE, &format!("{}{}", self.base_url, endpoint), token, Some(body)).await?;
        let issue: Issue = response.json().await?;
        
        debug!("Unassigned {} users from issue #{} in repository: {}/{}", assignees.len(), issue_number, owner, repo);
        Ok(issue)
    }
    
    pub async fn list_issue_comments(&self, token: &str, owner: &str, repo: &str, issue_number: u32, per_page: Option<u32>, page: Option<u32>) -> Result<Vec<Value>, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/issues/{}/comments", owner, repo, issue_number), "GET");
        
        let mut query_params = Vec::new();
        
        if let Some(per_page) = per_page {
            query_params.push(format!("per_page={}", per_page));
        }
        if let Some(page) = page {
            query_params.push(format!("page={}", page));
        }
        
        let query_string = if query_params.is_empty() {
            String::new()
        } else {
            format!("?{}", query_params.join("&"))
        };
        
        let endpoint = format!("/repos/{}/{}/issues/{}/comments{}", owner, repo, issue_number, query_string);
        let response = self.get(&endpoint, token).await?;
        let comments: Vec<Value> = response.json().await?;
        
        debug!("Retrieved {} comments for issue #{} in repository: {}/{}", comments.len(), issue_number, owner, repo);
        Ok(comments)
    }
    
    pub async fn create_issue_comment(&self, token: &str, owner: &str, repo: &str, issue_number: u32, body: &str) -> Result<Value, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/issues/{}/comments", owner, repo, issue_number), "POST");
        
        let endpoint = format!("/repos/{}/{}/issues/{}/comments", owner, repo, issue_number);
        let request_body = serde_json::json!({ "body": body });
        let response = self.post(&endpoint, token, Some(request_body)).await?;
        let comment: Value = response.json().await?;
        
        debug!("Created comment on issue #{} in repository: {}/{}", issue_number, owner, repo);
        Ok(comment)
    }
    
    pub async fn search_issues(&self, token: &str, query: &str, sort: Option<&str>, order: Option<&str>, per_page: Option<u32>, page: Option<u32>) -> Result<Vec<Issue>, GitHubMcpError> {
        log_github_api_call!("/search/issues", "GET");
        
        let mut query_params = vec![format!("q={}", urlencoding::encode(query))];
        
        if let Some(sort_param) = sort {
            query_params.push(format!("sort={}", sort_param));
        }
        if let Some(order_param) = order {
            query_params.push(format!("order={}", order_param));
        }
        if let Some(per_page) = per_page {
            query_params.push(format!("per_page={}", per_page));
        }
        if let Some(page) = page {
            query_params.push(format!("page={}", page));
        }
        
        let query_string = query_params.join("&");
        let endpoint = format!("/search/issues?{}", query_string);
        
        let response = self.get(&endpoint, token).await?;
        let search_result: Value = response.json().await?;
        
        let issues = search_result["items"]
            .as_array()
            .ok_or_else(|| GitHubMcpError::SerializationError("Invalid search response format".to_string()))?
            .iter()
            .map(|item| serde_json::from_value(item.clone()))
            .collect::<Result<Vec<Issue>, _>>()?;
        
        info!("Found {} issues matching query: {}", issues.len(), query);
        Ok(issues)
    }
    
    // Pull request operations
    pub async fn list_pull_requests(&self, token: &str, owner: &str, repo: &str, state: Option<&str>, head: Option<&str>, base: Option<&str>, sort: Option<&str>, direction: Option<&str>, per_page: Option<u32>, page: Option<u32>) -> Result<Vec<PullRequest>, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/pulls", owner, repo), "GET");
        
        let mut query_params = Vec::new();
        
        if let Some(state_val) = state {
            query_params.push(format!("state={}", state_val));
        }
        if let Some(head_val) = head {
            query_params.push(format!("head={}", urlencoding::encode(head_val)));
        }
        if let Some(base_val) = base {
            query_params.push(format!("base={}", urlencoding::encode(base_val)));
        }
        if let Some(sort_val) = sort {
            query_params.push(format!("sort={}", sort_val));
        }
        if let Some(direction_val) = direction {
            query_params.push(format!("direction={}", direction_val));
        }
        if let Some(per_page) = per_page {
            query_params.push(format!("per_page={}", per_page));
        }
        if let Some(page) = page {
            query_params.push(format!("page={}", page));
        }
        
        let query_string = if query_params.is_empty() {
            String::new()
        } else {
            format!("?{}", query_params.join("&"))
        };
        
        let endpoint = format!("/repos/{}/{}/pulls{}", owner, repo, query_string);
        let response = self.get(&endpoint, token).await?;
        let pull_requests: Vec<PullRequest> = response.json().await?;
        
        info!("Retrieved {} pull requests for repository: {}/{}", pull_requests.len(), owner, repo);
        Ok(pull_requests)
    }
    
    pub async fn get_pull_request(&self, token: &str, owner: &str, repo: &str, pull_number: u32) -> Result<PullRequest, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/pulls/{}", owner, repo, pull_number), "GET");
        
        let endpoint = format!("/repos/{}/{}/pulls/{}", owner, repo, pull_number);
        let response = self.get(&endpoint, token).await?;
        let pull_request: PullRequest = response.json().await?;
        
        debug!("Retrieved pull request #{} from repository: {}/{}", pull_number, owner, repo);
        Ok(pull_request)
    }
    
    pub async fn create_pull_request(&self, token: &str, owner: &str, repo: &str, request: &CreatePullRequestRequest) -> Result<PullRequest, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/pulls", owner, repo), "POST");
        
        let endpoint = format!("/repos/{}/{}/pulls", owner, repo);
        let body = serde_json::to_value(request)?;
        let response = self.post(&endpoint, token, Some(body)).await?;
        let pull_request: PullRequest = response.json().await?;
        
        info!("Created pull request #{} in repository: {}/{}", pull_request.number, owner, repo);
        Ok(pull_request)
    }
    
    pub async fn update_pull_request(&self, token: &str, owner: &str, repo: &str, pull_number: u32, title: Option<&str>, body: Option<&str>, state: Option<&str>, base: Option<&str>) -> Result<PullRequest, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/pulls/{}", owner, repo, pull_number), "PATCH");
        
        let mut update_data = serde_json::Map::new();
        
        if let Some(title_val) = title {
            update_data.insert("title".to_string(), serde_json::Value::String(title_val.to_string()));
        }
        if let Some(body_val) = body {
            update_data.insert("body".to_string(), serde_json::Value::String(body_val.to_string()));
        }
        if let Some(state_val) = state {
            update_data.insert("state".to_string(), serde_json::Value::String(state_val.to_string()));
        }
        if let Some(base_val) = base {
            update_data.insert("base".to_string(), serde_json::Value::String(base_val.to_string()));
        }
        
        let endpoint = format!("/repos/{}/{}/pulls/{}", owner, repo, pull_number);
        let body = serde_json::Value::Object(update_data);
        let response = self.patch(&endpoint, token, Some(body)).await?;
        let pull_request: PullRequest = response.json().await?;
        
        info!("Updated pull request #{} in repository: {}/{}", pull_number, owner, repo);
        Ok(pull_request)
    }
    
    pub async fn close_pull_request(&self, token: &str, owner: &str, repo: &str, pull_number: u32) -> Result<PullRequest, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/pulls/{}", owner, repo, pull_number), "PATCH");
        
        self.update_pull_request(token, owner, repo, pull_number, None, None, Some("closed"), None).await
    }
    
    pub async fn reopen_pull_request(&self, token: &str, owner: &str, repo: &str, pull_number: u32) -> Result<PullRequest, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/pulls/{}", owner, repo, pull_number), "PATCH");
        
        self.update_pull_request(token, owner, repo, pull_number, None, None, Some("open"), None).await
    }
    
    pub async fn merge_pull_request(&self, token: &str, owner: &str, repo: &str, pull_number: u32, commit_title: Option<&str>, commit_message: Option<&str>, merge_method: Option<&str>) -> Result<Value, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/pulls/{}/merge", owner, repo, pull_number), "PUT");
        
        let mut merge_data = serde_json::Map::new();
        
        if let Some(title) = commit_title {
            merge_data.insert("commit_title".to_string(), serde_json::Value::String(title.to_string()));
        }
        if let Some(message) = commit_message {
            merge_data.insert("commit_message".to_string(), serde_json::Value::String(message.to_string()));
        }
        if let Some(method) = merge_method {
            merge_data.insert("merge_method".to_string(), serde_json::Value::String(method.to_string()));
        }
        
        let endpoint = format!("/repos/{}/{}/pulls/{}/merge", owner, repo, pull_number);
        let body = serde_json::Value::Object(merge_data);
        let response = self.put(&endpoint, token, Some(body)).await?;
        let merge_result: Value = response.json().await?;
        
        info!("Merged pull request #{} in repository: {}/{}", pull_number, owner, repo);
        Ok(merge_result)
    }
    
    pub async fn get_pull_request_files(&self, token: &str, owner: &str, repo: &str, pull_number: u32, per_page: Option<u32>, page: Option<u32>) -> Result<Vec<Value>, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/pulls/{}/files", owner, repo, pull_number), "GET");
        
        let mut query_params = Vec::new();
        
        if let Some(per_page) = per_page {
            query_params.push(format!("per_page={}", per_page));
        }
        if let Some(page) = page {
            query_params.push(format!("page={}", page));
        }
        
        let query_string = if query_params.is_empty() {
            String::new()
        } else {
            format!("?{}", query_params.join("&"))
        };
        
        let endpoint = format!("/repos/{}/{}/pulls/{}/files{}", owner, repo, pull_number, query_string);
        let response = self.get(&endpoint, token).await?;
        let files: Vec<Value> = response.json().await?;
        
        debug!("Retrieved {} files for pull request #{} in repository: {}/{}", files.len(), pull_number, owner, repo);
        Ok(files)
    }
    
    pub async fn get_pull_request_commits(&self, token: &str, owner: &str, repo: &str, pull_number: u32, per_page: Option<u32>, page: Option<u32>) -> Result<Vec<Commit>, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/pulls/{}/commits", owner, repo, pull_number), "GET");
        
        let mut query_params = Vec::new();
        
        if let Some(per_page) = per_page {
            query_params.push(format!("per_page={}", per_page));
        }
        if let Some(page) = page {
            query_params.push(format!("page={}", page));
        }
        
        let query_string = if query_params.is_empty() {
            String::new()
        } else {
            format!("?{}", query_params.join("&"))
        };
        
        let endpoint = format!("/repos/{}/{}/pulls/{}/commits{}", owner, repo, pull_number, query_string);
        let response = self.get(&endpoint, token).await?;
        let commits: Vec<Commit> = response.json().await?;
        
        debug!("Retrieved {} commits for pull request #{} in repository: {}/{}", commits.len(), pull_number, owner, repo);
        Ok(commits)
    }
    
    pub async fn list_pull_request_reviews(&self, token: &str, owner: &str, repo: &str, pull_number: u32, per_page: Option<u32>, page: Option<u32>) -> Result<Vec<Value>, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/pulls/{}/reviews", owner, repo, pull_number), "GET");
        
        let mut query_params = Vec::new();
        
        if let Some(per_page) = per_page {
            query_params.push(format!("per_page={}", per_page));
        }
        if let Some(page) = page {
            query_params.push(format!("page={}", page));
        }
        
        let query_string = if query_params.is_empty() {
            String::new()
        } else {
            format!("?{}", query_params.join("&"))
        };
        
        let endpoint = format!("/repos/{}/{}/pulls/{}/reviews{}", owner, repo, pull_number, query_string);
        let response = self.get(&endpoint, token).await?;
        let reviews: Vec<Value> = response.json().await?;
        
        debug!("Retrieved {} reviews for pull request #{} in repository: {}/{}", reviews.len(), pull_number, owner, repo);
        Ok(reviews)
    }
    
    pub async fn create_pull_request_review(&self, token: &str, owner: &str, repo: &str, pull_number: u32, body: Option<&str>, event: &str, comments: Option<Vec<Value>>) -> Result<Value, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/pulls/{}/reviews", owner, repo, pull_number), "POST");
        
        let mut review_data = serde_json::Map::new();
        
        if let Some(body_val) = body {
            review_data.insert("body".to_string(), serde_json::Value::String(body_val.to_string()));
        }
        review_data.insert("event".to_string(), serde_json::Value::String(event.to_string()));
        
        if let Some(comments_val) = comments {
            review_data.insert("comments".to_string(), serde_json::Value::Array(comments_val));
        }
        
        let endpoint = format!("/repos/{}/{}/pulls/{}/reviews", owner, repo, pull_number);
        let body = serde_json::Value::Object(review_data);
        let response = self.post(&endpoint, token, Some(body)).await?;
        let review: Value = response.json().await?;
        
        info!("Created review for pull request #{} in repository: {}/{}", pull_number, owner, repo);
        Ok(review)
    }
    
    pub async fn request_pull_request_reviewers(&self, token: &str, owner: &str, repo: &str, pull_number: u32, reviewers: Vec<String>, team_reviewers: Option<Vec<String>>) -> Result<PullRequest, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/pulls/{}/requested_reviewers", owner, repo, pull_number), "POST");
        
        let mut request_data = serde_json::Map::new();
        request_data.insert("reviewers".to_string(), serde_json::Value::Array(
            reviewers.into_iter().map(serde_json::Value::String).collect()
        ));
        
        if let Some(teams) = team_reviewers {
            request_data.insert("team_reviewers".to_string(), serde_json::Value::Array(
                teams.into_iter().map(serde_json::Value::String).collect()
            ));
        }
        
        let endpoint = format!("/repos/{}/{}/pulls/{}/requested_reviewers", owner, repo, pull_number);
        let body = serde_json::Value::Object(request_data);
        let response = self.post(&endpoint, token, Some(body)).await?;
        let pull_request: PullRequest = response.json().await?;
        
        debug!("Requested reviewers for pull request #{} in repository: {}/{}", pull_number, owner, repo);
        Ok(pull_request)
    }
    
    pub async fn remove_pull_request_reviewers(&self, token: &str, owner: &str, repo: &str, pull_number: u32, reviewers: Vec<String>, team_reviewers: Option<Vec<String>>) -> Result<PullRequest, GitHubMcpError> {
        log_github_api_call!(&format!("/repos/{}/{}/pulls/{}/requested_reviewers", owner, repo, pull_number), "DELETE");
        
        let mut request_data = serde_json::Map::new();
        request_data.insert("reviewers".to_string(), serde_json::Value::Array(
            reviewers.into_iter().map(serde_json::Value::String).collect()
        ));
        
        if let Some(teams) = team_reviewers {
            request_data.insert("team_reviewers".to_string(), serde_json::Value::Array(
                teams.into_iter().map(serde_json::Value::String).collect()
            ));
        }
        
        let endpoint = format!("/repos/{}/{}/pulls/{}/requested_reviewers", owner, repo, pull_number);
        let body = serde_json::Value::Object(request_data);
        let response = self.make_request(Method::DELETE, &format!("{}{}", self.base_url, endpoint), token, Some(body)).await?;
        let pull_request: PullRequest = response.json().await?;
        
        debug!("Removed reviewers from pull request #{} in repository: {}/{}", pull_number, owner, repo);
        Ok(pull_request)
    }
    
    pub async fn check_pull_request_mergeable(&self, token: &str, owner: &str, repo: &str, pull_number: u32) -> Result<bool, GitHubMcpError> {
        let pull_request = self.get_pull_request(token, owner, repo, pull_number).await?;
        
        // GitHub API may return null for mergeable initially, so we might need to retry
        match pull_request.mergeable {
            Some(mergeable) => Ok(mergeable),
            None => {
                // Wait a moment and try again as GitHub might still be calculating
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                let updated_pr = self.get_pull_request(token, owner, repo, pull_number).await?;
                Ok(updated_pr.mergeable.unwrap_or(false))
            }
        }
    }}
