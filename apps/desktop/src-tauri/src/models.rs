use serde::{Deserialize, Serialize};

pub const CATEGORIES: [&str; 11] = [
    "编程开发",
    "测试与质量",
    "DevOps 与云",
    "数据与 AI",
    "设计与媒体",
    "文档与办公",
    "研究与知识",
    "业务与营销",
    "安全与合规",
    "Agent 与工具",
    "其他",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthFinding {
    pub id: String,
    pub asset_id: String,
    pub code: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInstallation {
    pub id: String,
    pub asset_id: String,
    pub provider: String,
    pub provider_label: String,
    pub scope: String,
    pub display_path: String,
    pub normalized_path: String,
    pub resolved_target: Option<String>,
    pub link_type: String,
    pub read_only: bool,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationRecord {
    pub category: String,
    pub tags: Vec<String>,
    pub source: String,
    pub rationale: Option<String>,
    pub model: Option<String>,
    pub content_fingerprint: Option<String>,
    pub is_stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillAsset {
    pub id: String,
    pub logical_key: String,
    pub name: String,
    pub description: String,
    pub content_fingerprint: String,
    pub compatibility: Option<String>,
    pub license: Option<String>,
    pub author: Option<String>,
    pub body_preview: String,
    pub manifest_body: Option<String>,
    pub category: String,
    pub tags: Vec<String>,
    pub classification: ClassificationRecord,
    pub installation_count: usize,
    pub providers: Vec<String>,
    pub scopes: Vec<String>,
    pub health: String,
    pub finding_count: usize,
    pub has_conflict: bool,
    pub has_scripts: bool,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDetail {
    #[serde(flatten)]
    pub asset: SkillAsset,
    pub installations: Vec<SkillInstallation>,
    pub findings: Vec<HealthFinding>,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillQuery {
    pub search: Option<String>,
    pub provider: Option<String>,
    pub scope: Option<String>,
    pub category: Option<String>,
    pub health: Option<String>,
    pub conflicts_only: Option<bool>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedSkills {
    pub items: Vec<SkillAsset>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRoot {
    pub id: String,
    pub provider: String,
    pub provider_label: String,
    pub scope: String,
    pub display_path: String,
    pub normalized_path: String,
    pub exists: bool,
    pub custom: bool,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub scan_id: String,
    pub assets: usize,
    pub installations: usize,
    pub findings: usize,
    pub conflicts: usize,
    pub roots_scanned: usize,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    pub assets: usize,
    pub installations: usize,
    pub providers: usize,
    pub findings: usize,
    pub conflicts: usize,
    pub categories: Vec<CategoryCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryCount {
    pub category: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRequest {
    #[serde(default = "default_true")]
    pub include_default_roots: bool,
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationConsent {
    pub confirmed: bool,
    pub include_manifest_body: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelProfile {
    pub id: String,
    #[serde(default = "default_ai_provider")]
    pub provider: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub api_mode: String,
    #[serde(default)]
    pub custom_headers: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub has_api_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub profiles: Vec<AiModelProfile>,
    pub active_profile_id: String,
}

fn default_ai_provider() -> String {
    "custom".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnlineSkillResult {
    pub name: String,
    pub description: String,
    pub source_url: String,
    pub repository_url: Option<String>,
    pub author: Option<String>,
    pub why_relevant: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnlineSearchConsent {
    pub confirmed: bool,
    pub query: String,
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScanBundle {
    pub assets: Vec<SkillAsset>,
    pub installations: Vec<SkillInstallation>,
    pub findings: Vec<HealthFinding>,
    pub files: Vec<(String, FileEntry)>,
}
