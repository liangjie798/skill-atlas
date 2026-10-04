use crate::models::{
    ClassificationRecord, FileEntry, HealthFinding, ScanBundle, SkillAsset, SkillInstallation,
};
use crate::providers::{normalized, RootSpec};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use regex::Regex;
use serde_yml::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use walkdir::WalkDir;

#[derive(Debug)]
struct Candidate {
    asset: SkillAsset,
    installation: SkillInstallation,
    findings: Vec<HealthFinding>,
    files: Vec<FileEntry>,
}

pub fn scan(roots: &[RootSpec]) -> Result<ScanBundle> {
    scan_controlled(roots, &AtomicBool::new(false), |_, _| {})
}

pub fn scan_controlled(
    roots: &[RootSpec],
    cancelled: &AtomicBool,
    mut progress: impl FnMut(usize, usize),
) -> Result<ScanBundle> {
    let mut seen_manifests = HashSet::new();
    let mut candidates = Vec::new();
    let available = roots
        .iter()
        .filter(|root| root.root.exists)
        .collect::<Vec<_>>();
    let total = available.len();
    for (root_index, spec) in available.into_iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            anyhow::bail!("扫描已取消");
        }
        progress(root_index, total);
        for entry in WalkDir::new(&spec.path)
            .max_depth(spec.max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            if cancelled.load(Ordering::Relaxed) {
                anyhow::bail!("扫描已取消");
            }
            if entry.file_type().is_file()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("skill.md")
            {
                let path = entry.path().to_path_buf();
                if seen_manifests.insert(normalized(&path)) {
                    candidates.push(read_candidate(spec, &path, None)?);
                }
            } else if entry.file_type().is_symlink() {
                if let Ok(target) = fs::canonicalize(entry.path()) {
                    let manifest = target.join("SKILL.md");
                    if manifest.is_file() {
                        let key =
                            format!("{}->{}", normalized(entry.path()), normalized(&manifest));
                        if seen_manifests.insert(key) {
                            candidates.push(read_candidate(
                                spec,
                                &manifest,
                                Some(entry.path().to_path_buf()),
                            )?);
                        }
                    }
                }
            }
        }
    }
    progress(total, total);
    Ok(resolve(candidates))
}

fn read_candidate(
    spec: &RootSpec,
    manifest: &Path,
    installation_override: Option<PathBuf>,
) -> Result<Candidate> {
    let skill_dir = manifest
        .parent()
        .context("SKILL.md has no parent directory")?;
    let install_dir = installation_override.as_deref().unwrap_or(skill_dir);
    let bytes =
        fs::read(manifest).with_context(|| format!("Cannot read {}", manifest.display()))?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let (frontmatter, body, frontmatter_error) = split_frontmatter(&text);
    let parsed: Option<Value> = frontmatter
        .as_deref()
        .and_then(|value| serde_yml::from_str(value).ok());
    let map = parsed.as_ref().and_then(Value::as_mapping);
    let folder_name = install_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unnamed-skill");
    let name = yaml_string(map, "name").unwrap_or_else(|| folder_name.to_string());
    let description = yaml_string(map, "description").unwrap_or_default();
    let compatibility = yaml_string(map, "compatibility");
    let license = yaml_string(map, "license");
    let author = map
        .and_then(|m| m.get(Value::String("metadata".into())))
        .and_then(Value::as_mapping)
        .and_then(|m| yaml_string(Some(m), "author"));
    let (fingerprint, files, has_scripts, limit_warning) = fingerprint_tree(skill_dir)?;
    let asset_id = format!("asset-{}", &fingerprint[..16]);
    let mut findings = validate(
        &asset_id,
        skill_dir,
        &name,
        &description,
        &text,
        frontmatter_error,
    );
    if let Some(detail) = limit_warning {
        findings.push(finding(
            &asset_id,
            "scan-limit",
            "warning",
            "扫描范围受限",
            &detail,
            Some(skill_dir),
        ));
    }
    if has_scripts {
        findings.push(finding(
            &asset_id,
            "executable-content",
            "info",
            "包含可执行脚本",
            "发现 scripts 目录。Skill Atlas 不会执行其中内容。",
            Some(skill_dir),
        ));
    }
    if installation_override.is_some()
        && !normalized(skill_dir).starts_with(&spec.root.normalized_path)
    {
        findings.push(finding(
            &asset_id,
            "link-outside-root",
            "warning",
            "链接目标位于扫描根目录外",
            "该链接指向批准根目录之外。内容仅以只读方式检查。",
            Some(skill_dir),
        ));
    }
    let (rule_category, rule_tags, rule_reason) = classify_rule(&name, &description, &body);
    let modified_at = fs::metadata(manifest)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .map(|time| DateTime::<Utc>::from(time).to_rfc3339());
    let link_type = if installation_override.is_some() {
        if cfg!(windows) {
            "junction"
        } else {
            "symlink"
        }
    } else {
        "directory"
    };
    let installation_id = hash_text(&format!("{}:{}", spec.root.id, normalized(install_dir)));
    let installation = SkillInstallation {
        id: format!("install-{}", &installation_id[..16]),
        asset_id: asset_id.clone(),
        provider: spec.root.provider.clone(),
        provider_label: spec.root.provider_label.clone(),
        scope: spec.root.scope.clone(),
        display_path: install_dir.to_string_lossy().into_owned(),
        normalized_path: normalized(install_dir),
        resolved_target: installation_override.map(|_| skill_dir.to_string_lossy().into_owned()),
        link_type: link_type.into(),
        read_only: true,
        modified_at: modified_at.clone(),
    };
    let health = health_from(&findings);
    let asset = SkillAsset {
        id: asset_id,
        logical_key: name.to_lowercase(),
        name,
        description,
        content_fingerprint: fingerprint.clone(),
        compatibility,
        license,
        author,
        body_preview: body.trim().chars().take(240).collect(),
        manifest_body: Some(text),
        category: rule_category.clone(),
        tags: rule_tags.clone(),
        classification: ClassificationRecord {
            category: rule_category,
            tags: rule_tags,
            source: "rule".into(),
            rationale: Some(rule_reason),
            model: None,
            content_fingerprint: Some(fingerprint),
            is_stale: false,
        },
        installation_count: 1,
        providers: vec![spec.root.provider.clone()],
        scopes: vec![spec.root.scope.clone()],
        health,
        finding_count: findings.len(),
        has_conflict: false,
        has_scripts,
        updated_at: modified_at,
    };
    Ok(Candidate {
        asset,
        installation,
        findings,
        files,
    })
}

fn resolve(candidates: Vec<Candidate>) -> ScanBundle {
    let mut by_fingerprint: HashMap<String, Vec<Candidate>> = HashMap::new();
    for candidate in candidates {
        by_fingerprint
            .entry(candidate.asset.content_fingerprint.clone())
            .or_default()
            .push(candidate);
    }
    let mut assets = Vec::new();
    let mut installations = Vec::new();
    let mut findings = Vec::new();
    let mut files = Vec::new();
    for (_, mut group) in by_fingerprint {
        let mut first = group.remove(0);
        let mut providers: BTreeSet<String> = first.asset.providers.iter().cloned().collect();
        let mut scopes: BTreeSet<String> = first.asset.scopes.iter().cloned().collect();
        installations.push(first.installation.clone());
        for file in first.files.drain(..) {
            files.push((first.asset.id.clone(), file));
        }
        for candidate in group {
            providers.insert(candidate.installation.provider.clone());
            scopes.insert(candidate.installation.scope.clone());
            installations.push(candidate.installation);
            first.findings.extend(candidate.findings);
        }
        first.asset.installation_count = installations
            .iter()
            .filter(|item| item.asset_id == first.asset.id)
            .count();
        first.asset.providers = providers.into_iter().collect();
        first.asset.scopes = scopes.into_iter().collect();
        if first.asset.installation_count > 1 {
            first.findings.push(finding(
                &first.asset.id,
                "duplicate-copy",
                "info",
                "发现多个相同安装",
                "这些安装内容完全相同，已归并为一个逻辑资产。",
                None,
            ));
        }
        let mut finding_ids = HashSet::new();
        first
            .findings
            .retain(|finding| finding_ids.insert(finding.id.clone()));
        first.asset.finding_count = first.findings.len();
        first.asset.health = health_from(&first.findings);
        findings.extend(first.findings);
        assets.push(first.asset);
    }
    let mut logical: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, asset) in assets.iter().enumerate() {
        logical
            .entry(asset.logical_key.clone())
            .or_default()
            .push(index);
    }
    for indexes in logical.values().filter(|indexes| indexes.len() > 1) {
        for index in indexes {
            let asset = &mut assets[*index];
            asset.has_conflict = true;
            asset.health = "error".into();
            asset.finding_count += 1;
            findings.push(finding(
                &asset.id,
                "name-conflict",
                "error",
                "同名内容冲突",
                "存在名称相同但内容指纹不同的 Skill。",
                None,
            ));
        }
    }
    ScanBundle {
        assets,
        installations,
        findings,
        files,
    }
}

fn split_frontmatter(text: &str) -> (Option<String>, String, Option<&'static str>) {
    let normalized = text.trim_start_matches('\u{feff}');
    if !normalized.starts_with("---") {
        return (
            None,
            normalized.into(),
            Some("SKILL.md 必须以 YAML frontmatter 开始。"),
        );
    }
    let mut lines = normalized.lines();
    lines.next();
    let mut yaml = Vec::new();
    let mut body = Vec::new();
    let mut ended = false;
    for line in lines {
        if !ended && line.trim() == "---" {
            ended = true;
            continue;
        }
        if ended {
            body.push(line);
        } else {
            yaml.push(line);
        }
    }
    if !ended {
        return (None, normalized.into(), Some("YAML frontmatter 未闭合。"));
    }
    let yaml_text = yaml.join("\n");
    if serde_yml::from_str::<Value>(&yaml_text).is_err() {
        return (
            Some(yaml_text),
            body.join("\n"),
            Some("YAML frontmatter 无法解析。"),
        );
    }
    (Some(yaml_text), body.join("\n"), None)
}

fn yaml_string(map: Option<&serde_yml::Mapping>, key: &str) -> Option<String> {
    map.and_then(|m| m.get(Value::String(key.into())))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn validate(
    asset_id: &str,
    skill_dir: &Path,
    name: &str,
    description: &str,
    text: &str,
    frontmatter_error: Option<&str>,
) -> Vec<HealthFinding> {
    let mut result = Vec::new();
    if let Some(error) = frontmatter_error {
        result.push(finding(
            asset_id,
            "invalid-frontmatter",
            "error",
            "Frontmatter 无效",
            error,
            Some(skill_dir),
        ));
    }
    let name_re = Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").unwrap();
    if !name_re.is_match(name) || name.len() > 64 {
        result.push(finding(
            asset_id,
            "invalid-name",
            "error",
            "名称不符合规范",
            "名称应为 1-64 个小写字母、数字或单连字符。",
            Some(skill_dir),
        ));
    }
    let folder = skill_dir
        .file_name()
        .and_then(|part| part.to_str())
        .unwrap_or_default();
    if !folder.eq_ignore_ascii_case(name) {
        result.push(finding(
            asset_id,
            "name-directory-mismatch",
            "warning",
            "名称与目录不一致",
            &format!("frontmatter 名称为 {name}，目录名为 {folder}。"),
            Some(skill_dir),
        ));
    }
    if description.trim().is_empty() || description.len() > 1024 {
        result.push(finding(
            asset_id,
            "invalid-description",
            "error",
            "描述缺失或过长",
            "description 必须为 1-1024 个字符。",
            Some(skill_dir),
        ));
    }
    let reference_re =
        Regex::new(r#"(?:\(|\s|`)((?:references|scripts|assets)/[^\s)`\]]+)"#).unwrap();
    for capture in reference_re.captures_iter(text) {
        let relative =
            capture.get(1).unwrap().as_str().trim_matches(|character| {
                character == '`' || character == '"' || character == '\''
            });
        if relative.contains("..") || !skill_dir.join(relative).exists() {
            result.push(finding(
                asset_id,
                "missing-reference",
                "warning",
                "引用文件缺失",
                &format!("未找到引用：{relative}"),
                Some(skill_dir),
            ));
        }
    }
    result
}

fn fingerprint_tree(dir: &Path) -> Result<(String, Vec<FileEntry>, bool, Option<String>)> {
    let mut paths = Vec::new();
    for entry in WalkDir::new(dir)
        .max_depth(12)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.path() == dir {
            continue;
        }
        paths.push(entry.path().to_path_buf());
        if paths.len() >= 500 {
            break;
        }
    }
    paths.sort_by_key(|path| normalized(path));
    let mut hasher = Sha256::new();
    let mut files = Vec::new();
    let mut total = 0_u64;
    let mut limited = None;
    for path in paths {
        let relative = path
            .strip_prefix(dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_dir() {
            files.push(FileEntry {
                path: relative,
                size: 0,
                kind: "directory".into(),
            });
            continue;
        }
        if !metadata.is_file() {
            continue;
        }
        if total.saturating_add(metadata.len()) > 25 * 1024 * 1024 {
            limited = Some("Skill 文件总量超过 25 MB，指纹只覆盖前 25 MB。".into());
            break;
        }
        let content = fs::read(&path)?;
        total += content.len() as u64;
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update(&content);
        files.push(FileEntry {
            path: relative,
            size: metadata.len(),
            kind: "file".into(),
        });
    }
    if files.len() >= 500 {
        limited = Some("Skill 超过 500 个文件，指纹只覆盖前 500 个。".into());
    }
    let has_scripts = files
        .iter()
        .any(|file| file.path == "scripts" || file.path.starts_with("scripts/"));
    Ok((
        format!("{:x}", hasher.finalize()),
        files,
        has_scripts,
        limited,
    ))
}

fn classify_rule(name: &str, description: &str, body: &str) -> (String, Vec<String>, String) {
    let text = format!(
        "{name} {description} {}",
        body.chars().take(1200).collect::<String>()
    )
    .to_lowercase();
    let rules = [
        (
            "安全与合规",
            ["security", "安全", "audit", "vulnerability"].as_slice(),
        ),
        ("测试与质量", ["test", "测试", "review", "lint"].as_slice()),
        (
            "DevOps 与云",
            ["deploy", "docker", "kubernetes", "cloud", "ci/cd"].as_slice(),
        ),
        (
            "数据与 AI",
            ["data", "数据", "model", "llm", "machine learning"].as_slice(),
        ),
        (
            "设计与媒体",
            ["design", "figma", "image", "video", "设计"].as_slice(),
        ),
        (
            "文档与办公",
            ["document", "pdf", "spreadsheet", "slides", "文档"].as_slice(),
        ),
        (
            "研究与知识",
            ["research", "search", "研究", "knowledge"].as_slice(),
        ),
        (
            "业务与营销",
            ["marketing", "sales", "seo", "营销", "business"].as_slice(),
        ),
        (
            "Agent 与工具",
            ["agent", "skill", "mcp", "plugin"].as_slice(),
        ),
        (
            "编程开发",
            ["code", "react", "typescript", "rust", "开发"].as_slice(),
        ),
    ];
    for (category, terms) in rules {
        if terms.iter().any(|term| text.contains(term)) {
            return (
                category.into(),
                terms
                    .iter()
                    .filter(|term| text.contains(**term))
                    .take(3)
                    .map(|term| (*term).to_string())
                    .collect(),
                format!("名称、描述或正文命中 {category} 的本地规则。"),
            );
        }
    }
    ("其他".into(), Vec::new(), "没有命中已知本地规则。".into())
}

fn finding(
    asset_id: &str,
    code: &str,
    severity: &str,
    title: &str,
    detail: &str,
    path: Option<&Path>,
) -> HealthFinding {
    let finding_path = path.map(normalized).unwrap_or_default();
    let id_hash = hash_text(&format!("{asset_id}:{code}:{detail}:{finding_path}"));
    HealthFinding {
        id: format!("finding-{}", &id_hash[..16]),
        asset_id: asset_id.into(),
        code: code.into(),
        severity: severity.into(),
        title: title.into(),
        detail: detail.into(),
        path: path.map(|p| p.to_string_lossy().into_owned()),
    }
}

fn health_from(findings: &[HealthFinding]) -> String {
    if findings.iter().any(|finding| finding.severity == "error") {
        "error".into()
    } else if findings.iter().any(|finding| finding.severity == "warning") {
        "attention".into()
    } else {
        "healthy".into()
    }
}

fn hash_text(value: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(value.as_bytes());
    format!("{:x}", hash.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parses_valid_frontmatter() {
        let (yaml, body, error) =
            split_frontmatter("---\nname: demo\ndescription: Demo\n---\nHello");
        assert!(yaml.is_some());
        assert_eq!(body, "Hello");
        assert!(error.is_none());
    }

    #[test]
    fn groups_identical_installs() {
        let temp = tempfile::tempdir().unwrap();
        let root_path = temp.path().join("skills");
        fs::create_dir_all(root_path.join("demo")).unwrap();
        let mut file = fs::File::create(root_path.join("demo/SKILL.md")).unwrap();
        writeln!(file, "---\nname: demo\ndescription: Demo skill\n---\nBody").unwrap();
        let spec = crate::providers::custom_root(
            "test".into(),
            root_path,
            "custom".into(),
            "custom".into(),
        );
        let result = scan(&[spec]).unwrap();
        assert_eq!(result.assets.len(), 1);
        assert_eq!(result.installations.len(), 1);
    }

    #[test]
    fn identical_installs_produce_unique_finding_ids() {
        let temp = tempfile::tempdir().unwrap();
        let mut specs = Vec::new();
        for index in 1..=2 {
            let root_path = temp.path().join(format!("skills-{index}"));
            let skill_path = root_path.join("demo");
            fs::create_dir_all(skill_path.join("scripts")).unwrap();
            fs::write(
                skill_path.join("SKILL.md"),
                "---\nname: demo\ndescription: Demo skill\n---\nBody",
            )
            .unwrap();
            fs::write(skill_path.join("scripts/run.js"), "console.log('demo')").unwrap();
            specs.push(crate::providers::custom_root(
                format!("test-{index}"),
                root_path,
                "custom".into(),
                "custom".into(),
            ));
        }

        let result = scan(&specs).unwrap();
        let unique = result
            .findings
            .iter()
            .map(|finding| finding.id.as_str())
            .collect::<HashSet<_>>();
        assert_eq!(unique.len(), result.findings.len());
        let store = crate::store::Store::open(temp.path().join("index.sqlite3")).unwrap();
        store.replace_scan(result).unwrap();
    }
}
