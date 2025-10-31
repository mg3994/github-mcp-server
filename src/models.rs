use serde::{Deserialize, Serialize};

// GitHub data models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: u64,
    pub node_id: String,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub private: bool,
    pub html_url: String,
    pub clone_url: String,
    pub git_url: String,
    pub ssh_url: String,
    pub default_branch: String,
    pub owner: User,
    pub created_at: String,
    pub updated_at: String,
    pub pushed_at: Option<String>,
    pub size: u64,
    pub stargazers_count: u32,
    pub watchers_count: u32,
    pub forks_count: u32,
    pub open_issues_count: u32,
    pub language: Option<String>,
    pub topics: Vec<String>,
    pub archived: bool,
    pub disabled: bool,
    pub visibility: String,
    pub permissions: Option<RepositoryPermissions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryPermissions {
    pub admin: bool,
    pub maintain: Option<bool>,
    pub push: bool,
    pub triage: Option<bool>,
    pub pull: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub node_id: String,
    pub login: String,
    pub avatar_url: String,
    pub gravatar_id: Option<String>,
    pub html_url: String,
    pub followers_url: String,
    pub following_url: String,
    pub gists_url: String,
    pub starred_url: String,
    pub subscriptions_url: String,
    pub organizations_url: String,
    pub repos_url: String,
    pub events_url: String,
    pub received_events_url: String,
    #[serde(rename = "type")]
    pub user_type: String,
    pub site_admin: bool,
    // Additional fields for authenticated user
    pub name: Option<String>,
    pub company: Option<String>,
    pub blog: Option<String>,
    pub location: Option<String>,
    pub email: Option<String>,
    pub hireable: Option<bool>,
    pub bio: Option<String>,
    pub twitter_username: Option<String>,
    pub public_repos: Option<u32>,
    pub public_gists: Option<u32>,
    pub followers: Option<u32>,
    pub following: Option<u32>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: u64,
    pub node_id: String,
    pub number: u32,
    pub title: String,
    pub body: Option<String>,
    pub state: IssueState,
    pub state_reason: Option<String>,
    pub labels: Vec<Label>,
    pub assignee: Option<User>,
    pub assignees: Vec<User>,
    pub milestone: Option<Milestone>,
    pub locked: bool,
    pub active_lock_reason: Option<String>,
    pub comments: u32,
    pub pull_request: Option<IssuePullRequest>,
    pub closed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub closed_by: Option<User>,
    pub author_association: String,
    pub draft: Option<bool>,
    pub html_url: String,
    pub comments_url: String,
    pub events_url: String,
    pub labels_url: String,
    pub repository_url: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueState {
    Open,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuePullRequest {
    pub url: String,
    pub html_url: String,
    pub diff_url: String,
    pub patch_url: String,
    pub merged_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: u64,
    pub node_id: String,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
    pub default: bool,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: u64,
    pub node_id: String,
    pub number: u32,
    pub title: String,
    pub description: Option<String>,
    pub creator: User,
    pub open_issues: u32,
    pub closed_issues: u32,
    pub state: MilestoneState,
    pub created_at: String,
    pub updated_at: String,
    pub due_on: Option<String>,
    pub closed_at: Option<String>,
    pub html_url: String,
    pub labels_url: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MilestoneState {
    Open,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub id: u64,
    pub node_id: String,
    pub number: u32,
    pub title: String,
    pub body: Option<String>,
    pub state: PullRequestState,
    pub locked: bool,
    pub user: User,
    pub assignee: Option<User>,
    pub assignees: Vec<User>,
    pub requested_reviewers: Vec<User>,
    pub requested_teams: Vec<Team>,
    pub labels: Vec<Label>,
    pub milestone: Option<Milestone>,
    pub draft: bool,
    pub commits_url: String,
    pub review_comments_url: String,
    pub review_comment_url: String,
    pub comments_url: String,
    pub statuses_url: String,
    pub head: PullRequestBranch,
    pub base: PullRequestBranch,
    pub author_association: String,
    pub auto_merge: Option<serde_json::Value>,
    pub active_lock_reason: Option<String>,
    pub merged: Option<bool>,
    pub mergeable: Option<bool>,
    pub rebaseable: Option<bool>,
    pub mergeable_state: Option<String>,
    pub merged_by: Option<User>,
    pub comments: u32,
    pub review_comments: u32,
    pub maintainer_can_modify: bool,
    pub commits: u32,
    pub additions: u32,
    pub deletions: u32,
    pub changed_files: u32,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub merged_at: Option<String>,
    pub merge_commit_sha: Option<String>,
    pub html_url: String,
    pub url: String,
    pub issue_url: String,
    pub patch_url: String,
    pub diff_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PullRequestState {
    Open,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestBranch {
    pub label: String,
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
    pub user: User,
    pub repo: Option<Repository>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: u64,
    pub node_id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub privacy: String,
    pub permission: String,
    pub url: String,
    pub html_url: String,
    pub members_url: String,
    pub repositories_url: String,
    pub parent: Option<Box<Team>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    pub name: String,
    pub path: String,
    pub sha: String,
    pub size: u64,
    pub url: String,
    pub html_url: String,
    pub git_url: String,
    pub download_url: Option<String>,
    #[serde(rename = "type")]
    pub file_type: String,
    pub content: Option<String>, // Base64 encoded content
    pub encoding: Option<String>, // "base64" or "utf-8"
    pub target: Option<String>, // For symlinks
    pub submodule_git_url: Option<String>, // For submodules
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryItem {
    pub name: String,
    pub path: String,
    pub sha: String,
    pub size: Option<u64>,
    pub url: String,
    pub html_url: String,
    pub git_url: String,
    pub download_url: Option<String>,
    #[serde(rename = "type")]
    pub item_type: String, // "file", "dir", "symlink", "submodule"
    pub target: Option<String>, // For symlinks
    pub submodule_git_url: Option<String>, // For submodules
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitReference {
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub node_id: String,
    pub url: String,
    pub object: GitObject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitObject {
    pub sha: String,
    #[serde(rename = "type")]
    pub object_type: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub commit: BranchCommit,
    pub protected: bool,
    pub protection: Option<BranchProtection>,
    pub protection_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchCommit {
    pub sha: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchProtection {
    pub enabled: bool,
    pub required_status_checks: Option<RequiredStatusChecks>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredStatusChecks {
    pub enforcement_level: String,
    pub contexts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub sha: String,
    pub node_id: String,
    pub commit: CommitDetails,
    pub url: String,
    pub html_url: String,
    pub comments_url: String,
    pub author: Option<User>,
    pub committer: Option<User>,
    pub parents: Vec<CommitParent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDetails {
    pub author: GitUser,
    pub committer: GitUser,
    pub message: String,
    pub tree: GitTree,
    pub url: String,
    pub comment_count: u32,
    pub verification: Option<CommitVerification>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitUser {
    pub name: String,
    pub email: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitTree {
    pub sha: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitParent {
    pub sha: String,
    pub url: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitVerification {
    pub verified: bool,
    pub reason: String,
    pub signature: Option<String>,
    pub payload: Option<String>,
}

// MCP protocol models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolParams {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResult {
    pub content: Vec<ToolContent>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "isError")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { 
        data: String, 
        #[serde(rename = "mimeType")]
        mime_type: String 
    },
    #[serde(rename = "resource")]
    Resource { 
        resource: ResourceReference 
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReference {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

// Legacy types for backward compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponse {
    pub content: Vec<ToolResponseContent>,
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponseContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

// Request/Response models for GitHub operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListReposParams {
    pub visibility: Option<String>, // "all", "public", "private"
    pub sort: Option<String>,       // "created", "updated", "pushed", "full_name"
    pub direction: Option<String>,  // "asc", "desc"
    pub per_page: Option<u32>,
    pub page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListIssuesParams {
    pub state: Option<String>,    // "open", "closed", "all"
    pub labels: Option<String>,   // comma-separated list
    pub assignee: Option<String>,
    pub sort: Option<String>,     // "created", "updated", "comments"
    pub direction: Option<String>, // "asc", "desc"
    pub per_page: Option<u32>,
    pub page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIssueRequest {
    pub title: String,
    pub body: Option<String>,
    pub labels: Option<Vec<String>>,
    pub assignees: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateIssueRequest {
    pub title: Option<String>,
    pub body: Option<String>,
    pub state: Option<IssueState>,
    pub labels: Option<Vec<String>>,
    pub assignees: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePullRequestRequest {
    pub title: String,
    pub body: Option<String>,
    pub head: String, // branch name
    pub base: String, // branch name
    pub draft: Option<bool>,
}

// Tool schema definitions
pub fn create_tool_schemas() -> Vec<Tool> {
    vec![
        Tool {
            name: "github_auth".to_string(),
            description: "Authenticate with GitHub using a personal access token".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "token": {
                        "type": "string",
                        "description": "GitHub personal access token"
                    }
                },
                "required": ["token"]
            }),
        },
        Tool {
            name: "github_list_repos".to_string(),
            description: "List repositories for the authenticated user".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "visibility": {
                        "type": "string",
                        "enum": ["all", "public", "private"],
                        "description": "Repository visibility filter",
                        "default": "all"
                    },
                    "sort": {
                        "type": "string",
                        "enum": ["created", "updated", "pushed", "full_name"],
                        "description": "Sort repositories by",
                        "default": "updated"
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["asc", "desc"],
                        "description": "Sort direction",
                        "default": "desc"
                    },
                    "per_page": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100,
                        "description": "Number of repositories per page",
                        "default": 30
                    },
                    "page": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Page number",
                        "default": 1
                    }
                }
            }),
        },
        Tool {    
        name: "github_search_repos".to_string(),
            description: "Search for repositories on GitHub".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "q": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "sort": {
                        "type": "string",
                        "enum": ["stars", "forks", "help-wanted-issues", "updated"],
                        "description": "Sort repositories by",
                        "default": "best-match"
                    },
                    "order": {
                        "type": "string",
                        "enum": ["asc", "desc"],
                        "description": "Sort order",
                        "default": "desc"
                    },
                    "per_page": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100,
                        "description": "Number of repositories per page",
                        "default": 30
                    },
                    "page": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Page number",
                        "default": 1
                    }
                },
                "required": ["q"]
            }),
        },
        Tool {
            name: "github_get_file".to_string(),
            description: "Get the contents of a file from a repository".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "owner": {
                        "type": "string",
                        "description": "Repository owner"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name"
                    },
                    "path": {
                        "type": "string",
                        "description": "File path"
                    },
                    "ref": {
                        "type": "string",
                        "description": "Branch, tag, or commit SHA",
                        "default": "main"
                    }
                },
                "required": ["owner", "repo", "path"]
            }),
        },
        Tool {
            name: "github_list_directory".to_string(),
            description: "List the contents of a directory in a repository".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "owner": {
                        "type": "string",
                        "description": "Repository owner"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory path",
                        "default": ""
                    },
                    "ref": {
                        "type": "string",
                        "description": "Branch, tag, or commit SHA",
                        "default": "main"
                    }
                },
                "required": ["owner", "repo"]
            }),
        },
        Tool {
            name: "github_list_issues".to_string(),
            description: "List issues for a repository".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "owner": {
                        "type": "string",
                        "description": "Repository owner"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name"
                    },
                    "state": {
                        "type": "string",
                        "enum": ["open", "closed", "all"],
                        "description": "Issue state filter",
                        "default": "open"
                    },
                    "labels": {
                        "type": "string",
                        "description": "Comma-separated list of label names"
                    },
                    "assignee": {
                        "type": "string",
                        "description": "Username of assignee"
                    },
                    "sort": {
                        "type": "string",
                        "enum": ["created", "updated", "comments"],
                        "description": "Sort issues by",
                        "default": "created"
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["asc", "desc"],
                        "description": "Sort direction",
                        "default": "desc"
                    },
                    "per_page": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100,
                        "description": "Number of issues per page",
                        "default": 30
                    },
                    "page": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Page number",
                        "default": 1
                    }
                },
                "required": ["owner", "repo"]
            }),
        },
        Tool {
            name: "github_create_issue".to_string(),
            description: "Create a new issue in a repository".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "owner": {
                        "type": "string",
                        "description": "Repository owner"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name"
                    },
                    "title": {
                        "type": "string",
                        "description": "Issue title"
                    },
                    "body": {
                        "type": "string",
                        "description": "Issue body"
                    },
                    "labels": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        },
                        "description": "Array of label names"
                    },
                    "assignees": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        },
                        "description": "Array of usernames to assign"
                    }
                },
                "required": ["owner", "repo", "title"]
            }),
        },
        Tool {
            name: "github_update_issue".to_string(),
            description: "Update an existing issue".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "owner": {
                        "type": "string",
                        "description": "Repository owner"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name"
                    },
                    "issue_number": {
                        "type": "integer",
                        "description": "Issue number"
                    },
                    "title": {
                        "type": "string",
                        "description": "Issue title"
                    },
                    "body": {
                        "type": "string",
                        "description": "Issue body"
                    },
                    "state": {
                        "type": "string",
                        "enum": ["open", "closed"],
                        "description": "Issue state"
                    },
                    "labels": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        },
                        "description": "Array of label names"
                    },
                    "assignees": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        },
                        "description": "Array of usernames to assign"
                    }
                },
                "required": ["owner", "repo", "issue_number"]
            }),
        },
        Tool {
            name: "github_list_prs".to_string(),
            description: "List pull requests for a repository".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "owner": {
                        "type": "string",
                        "description": "Repository owner"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name"
                    },
                    "state": {
                        "type": "string",
                        "enum": ["open", "closed", "all"],
                        "description": "Pull request state filter",
                        "default": "open"
                    },
                    "head": {
                        "type": "string",
                        "description": "Filter by head branch"
                    },
                    "base": {
                        "type": "string",
                        "description": "Filter by base branch"
                    },
                    "sort": {
                        "type": "string",
                        "enum": ["created", "updated", "popularity", "long-running"],
                        "description": "Sort pull requests by",
                        "default": "created"
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["asc", "desc"],
                        "description": "Sort direction",
                        "default": "desc"
                    },
                    "per_page": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100,
                        "description": "Number of pull requests per page",
                        "default": 30
                    },
                    "page": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Page number",
                        "default": 1
                    }
                },
                "required": ["owner", "repo"]
            }),
        },
        Tool {
            name: "github_create_pr".to_string(),
            description: "Create a new pull request".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "owner": {
                        "type": "string",
                        "description": "Repository owner"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name"
                    },
                    "title": {
                        "type": "string",
                        "description": "Pull request title"
                    },
                    "body": {
                        "type": "string",
                        "description": "Pull request body"
                    },
                    "head": {
                        "type": "string",
                        "description": "Head branch name"
                    },
                    "base": {
                        "type": "string",
                        "description": "Base branch name"
                    },
                    "draft": {
                        "type": "boolean",
                        "description": "Create as draft pull request",
                        "default": false
                    }
                },
                "required": ["owner", "repo", "title", "head", "base"]
            }),
        },
        Tool {
            name: "github_get_pr_details".to_string(),
            description: "Get details of a specific pull request".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "owner": {
                        "type": "string",
                        "description": "Repository owner"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name"
                    },
                    "pull_number": {
                        "type": "integer",
                        "description": "Pull request number"
                    }
                },
                "required": ["owner", "repo", "pull_number"]
            }),
        },
        Tool {
            name: "github_merge_pr".to_string(),
            description: "Merge a pull request".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "owner": {
                        "type": "string",
                        "description": "Repository owner"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name"
                    },
                    "pull_number": {
                        "type": "integer",
                        "description": "Pull request number"
                    },
                    "commit_title": {
                        "type": "string",
                        "description": "Commit title for merge"
                    },
                    "commit_message": {
                        "type": "string",
                        "description": "Commit message for merge"
                    },
                    "merge_method": {
                        "type": "string",
                        "enum": ["merge", "squash", "rebase"],
                        "description": "Merge method",
                        "default": "merge"
                    }
                },
                "required": ["owner", "repo", "pull_number"]
            }),
        },
    ]
}