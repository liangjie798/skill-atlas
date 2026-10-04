use crate::models::{
    AiModelProfile, ClassificationRecord, OnlineSkillResult, SkillDetail, CATEGORIES,
};
use anyhow::{anyhow, Context, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::json;
use std::str::FromStr;

const PROMPT_VERSION: &str = "skill-atlas-classifier-v2";

#[derive(Debug, Deserialize)]
struct AiClassification {
    category: String,
    #[serde(default)]
    tags: Vec<String>,
    rationale: String,
}

pub async fn classify(
    profile: &AiModelProfile,
    api_key: &str,
    detail: &SkillDetail,
) -> Result<ClassificationRecord> {
    let manifest = detail
        .asset
        .manifest_body
        .as_deref()
        .context("无法读取该 Skill 的 SKILL.md")?;
    let categories = CATEGORIES.join("、");
    let text = request_text(profile, api_key, &format!("你是 Agent Skill 分类器。只能从这些主分类中选一个：{categories}。返回严格 JSON，不要 Markdown，格式为 {{\"category\":\"...\",\"tags\":[\"...\"],\"rationale\":\"一句简短依据\"}}。标签最多 5 个。"), &format!("以下是用户明确授权发送的完整 SKILL.md。不要请求或假设其他资源文件。\n\n{manifest}"), false).await?;
    let result: AiClassification = parse_json(&text, "AI 分类结果不符合 JSON 契约")?;
    if !CATEGORIES.contains(&result.category.as_str()) {
        return Err(anyhow!("AI 返回了未知主分类：{}", result.category));
    }
    let tags = result
        .tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty() && tag.chars().count() <= 30)
        .take(5)
        .collect();
    Ok(ClassificationRecord {
        category: result.category,
        tags,
        source: "ai".into(),
        rationale: Some(result.rationale),
        model: Some(profile.model.clone()),
        content_fingerprint: Some(detail.asset.content_fingerprint.clone()),
        is_stale: false,
    })
}

pub async fn search_skills(
    profile: &AiModelProfile,
    api_key: &str,
    query: &str,
) -> Result<Vec<OnlineSkillResult>> {
    let system = "你是 Agent Skill 发现助手。必须联网搜索公开网页和 GitHub，寻找真实存在且与查询匹配的 Agent Skill。不要虚构仓库、作者或 URL。返回严格 JSON 数组，不要 Markdown。每项格式为 {\"name\":\"...\",\"description\":\"中文简介\",\"sourceUrl\":\"直接来源 URL\",\"repositoryUrl\":\"可选仓库 URL\",\"author\":\"可选作者\",\"whyRelevant\":\"中文匹配依据\",\"tags\":[\"...\"]}。最多返回 12 项，只包含可核验来源。";
    let text = request_text(
        profile,
        api_key,
        system,
        &format!("搜索适合以下需求的 Skill：{query}"),
        true,
    )
    .await?;
    let mut results: Vec<OnlineSkillResult> = parse_json(&text, "模型搜索结果不符合 JSON 契约")?;
    results.retain(|item| {
        item.source_url.starts_with("https://") || item.source_url.starts_with("http://")
    });
    for item in &mut results {
        item.tags.truncate(5);
    }
    results.truncate(12);
    if results.is_empty() {
        return Err(anyhow!("模型没有返回可核验的 Skill 来源。"));
    }
    Ok(results)
}

async fn request_text(
    profile: &AiModelProfile,
    api_key: &str,
    system: &str,
    user: &str,
    web_search: bool,
) -> Result<String> {
    if profile.base_url.trim().is_empty() || profile.model.trim().is_empty() {
        return Err(anyhow!("请先配置 Base URL 和模型。"));
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(if web_search {
            90
        } else {
            45
        }))
        .default_headers(headers(profile, api_key)?)
        .build()?;
    let (endpoint, payload) = if profile.api_mode == "responses-web-search" {
        let mut value = json!({"model": profile.model, "instructions": system, "input": user});
        if web_search {
            value["tools"] = json!([{"type":"web_search"}]);
            value["tool_choice"] = json!("required");
        }
        (
            format!("{}/responses", profile.base_url.trim_end_matches('/')),
            value,
        )
    } else {
        let mut value = json!({"model": profile.model, "messages":[{"role":"system","content":system},{"role":"user","content":user}]});
        if web_search {
            value["web_search_options"] = json!({"search_context_size":"medium"});
        }
        (
            format!(
                "{}/chat/completions",
                profile.base_url.trim_end_matches('/')
            ),
            value,
        )
    };
    let response = client
        .post(endpoint)
        .json(&payload)
        .send()
        .await
        .context("无法连接 AI 服务")?;
    let status = response.status();
    let response_text = response.text().await?;
    if !status.is_success() {
        return Err(anyhow!("AI 服务返回 HTTP {}。", status.as_u16()));
    }
    let envelope: serde_json::Value =
        serde_json::from_str(&response_text).context("AI 服务返回了无效 JSON")?;
    if profile.api_mode == "responses-web-search" {
        let text = envelope
            .get("output")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter(|item| item.get("type").and_then(|value| value.as_str()) == Some("message"))
            .flat_map(|item| {
                item.get("content")
                    .and_then(|value| value.as_array())
                    .into_iter()
                    .flatten()
            })
            .filter_map(|item| item.get("text").and_then(|value| value.as_str()))
            .collect::<Vec<_>>()
            .join("\n");
        if text.is_empty() {
            Err(anyhow!("AI 响应中没有文本结果。"))
        } else {
            Ok(text)
        }
    } else {
        envelope
            .pointer("/choices/0/message/content")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .context("AI 响应缺少 choices[0].message.content")
    }
}

fn headers(profile: &AiModelProfile, api_key: &str) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {api_key}"))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    for (name, value) in &profile.custom_headers {
        if !name.eq_ignore_ascii_case("authorization") && !name.eq_ignore_ascii_case("content-type")
        {
            headers.insert(HeaderName::from_str(name)?, HeaderValue::from_str(value)?);
        }
    }
    Ok(headers)
}

fn parse_json<T: for<'de> Deserialize<'de>>(text: &str, message: &str) -> Result<T> {
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    serde_json::from_str(cleaned).with_context(|| message.to_string())
}

pub fn prompt_version() -> &'static str {
    PROMPT_VERSION
}
