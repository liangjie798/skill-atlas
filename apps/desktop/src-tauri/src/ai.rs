use crate::models::{AiSettings, ClassificationRecord, SkillDetail, CATEGORIES};
use anyhow::{anyhow, Context, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::json;
use std::str::FromStr;

const PROMPT_VERSION: &str = "skill-atlas-classifier-v1";

#[derive(Debug, Deserialize)]
struct AiClassification { category: String, #[serde(default)] tags: Vec<String>, rationale: String }

pub async fn classify(settings: &AiSettings, api_key: &str, detail: &SkillDetail) -> Result<ClassificationRecord> {
    if settings.base_url.trim().is_empty() || settings.model.trim().is_empty() { return Err(anyhow!("请先配置 Base URL 和模型。")); }
    let manifest = detail.asset.manifest_body.as_deref().context("无法读取该 Skill 的 SKILL.md")?;
    let endpoint = format!("{}/chat/completions", settings.base_url.trim_end_matches('/'));
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {api_key}"))?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    for (name, value) in &settings.custom_headers {
        if name.eq_ignore_ascii_case("authorization") || name.eq_ignore_ascii_case("content-type") { continue; }
        headers.insert(HeaderName::from_str(name)?, HeaderValue::from_str(value)?);
    }
    let categories = CATEGORIES.join("、");
    let response = reqwest::Client::builder().timeout(std::time::Duration::from_secs(45)).default_headers(headers).build()?
        .post(endpoint)
        .json(&json!({
            "model": settings.model,
            "temperature": 0.1,
            "messages": [
                {"role":"system","content":format!("你是 Agent Skill 分类器。只能从这些主分类中选一个：{categories}。返回严格 JSON，不要 Markdown，格式为 {{\"category\":\"...\",\"tags\":[\"...\"],\"rationale\":\"一句简短依据\"}}。标签最多 5 个。")},
                {"role":"user","content":format!("以下是用户明确授权发送的完整 SKILL.md。不要请求或假设其他资源文件。\n\n{manifest}")}
            ]
        })).send().await?;
    let status = response.status(); let response_text = response.text().await?;
    if !status.is_success() { return Err(anyhow!("AI 服务返回 HTTP {}", status.as_u16())); }
    let envelope: serde_json::Value = serde_json::from_str(&response_text).context("AI 服务返回了无效 JSON")?;
    let content = envelope.pointer("/choices/0/message/content").and_then(|value| value.as_str()).context("AI 响应缺少 choices[0].message.content")?;
    let cleaned = content.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
    let result: AiClassification = serde_json::from_str(cleaned).context("AI 分类结果不符合 JSON 契约")?;
    if !CATEGORIES.contains(&result.category.as_str()) { return Err(anyhow!("AI 返回了未知主分类：{}", result.category)); }
    let tags = result.tags.into_iter().map(|tag| tag.trim().to_string()).filter(|tag| !tag.is_empty() && tag.chars().count() <= 30).take(5).collect();
    Ok(ClassificationRecord {
        category: result.category, tags, source: "ai".into(), rationale: Some(result.rationale),
        model: Some(settings.model.clone()), content_fingerprint: Some(detail.asset.content_fingerprint.clone()), is_stale: false,
    })
}

pub fn prompt_version() -> &'static str { PROMPT_VERSION }
