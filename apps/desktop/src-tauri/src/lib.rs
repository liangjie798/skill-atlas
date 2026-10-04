mod ai;
mod models;
mod providers;
mod scanner;
mod store;

use crate::models::*;
use crate::providers::{default_roots, normalized};
use crate::store::Store;
use keyring::Entry;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

const KEYRING_SERVICE: &str = "io.github.liangjie798.skillatlas";
const KEYRING_USER: &str = "ai-api-key";

struct AppState { store: Arc<Store>, scans: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>> }
type CommandResult<T> = Result<T, String>;
fn err(error: impl std::fmt::Display) -> String { error.to_string() }

#[tauri::command]
fn scan_roots(app: tauri::AppHandle, state: State<'_, AppState>, request: ScanRequest) -> CommandResult<String> {
    let scan_id = uuid::Uuid::new_v4().to_string();
    let _ = app.emit("scan://progress", serde_json::json!({"scanId":scan_id,"phase":"discover","progress":0}));
    let mut roots = if request.include_default_roots { default_roots() } else { Vec::new() };
    roots.extend(state.store.custom_roots().map_err(err)?);
    let roots_scanned = roots.iter().filter(|root| root.root.exists).count();
    let cancelled = Arc::new(AtomicBool::new(false));
    state.scans.lock().map_err(|_| "扫描状态不可用。".to_string())?.insert(scan_id.clone(), cancelled.clone());
    let scans = state.scans.clone(); let store = state.store.clone(); let task_id = scan_id.clone(); let task_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let started = Instant::now();
        let result = scanner::scan_controlled(&roots, &cancelled, |current, total| {
            let progress = if total == 0 { 100 } else { current.saturating_mul(100) / total };
            let _ = task_app.emit("scan://progress", serde_json::json!({"scanId":task_id,"phase":"scan","progress":progress,"currentRoot":current,"totalRoots":total}));
        });
        match result {
            Ok(bundle) if !cancelled.load(Ordering::Relaxed) => {
                let summary = ScanSummary { scan_id: task_id.clone(), assets: bundle.assets.len(), installations: bundle.installations.len(), findings: bundle.findings.iter().filter(|finding| finding.severity != "info").count(), conflicts: bundle.assets.iter().filter(|asset| asset.has_conflict).count(), roots_scanned, duration_ms: started.elapsed().as_millis() };
                match store.replace_scan(bundle) { Ok(()) => { let _ = task_app.emit("scan://completed", &summary); }, Err(error) => { let _ = task_app.emit("scan://failed", serde_json::json!({"scanId":task_id,"error":error.to_string()})); } }
            }
            Ok(_) => { let _ = task_app.emit("scan://failed", serde_json::json!({"scanId":task_id,"error":"扫描已取消"})); }
            Err(error) => { let _ = task_app.emit("scan://failed", serde_json::json!({"scanId":task_id,"error":error.to_string()})); }
        }
        if let Ok(mut active) = scans.lock() { active.remove(&task_id); }
    });
    Ok(scan_id)
}

#[tauri::command]
fn cancel_scan(state: State<'_, AppState>, scan_id: String) -> bool {
    state.scans.lock().ok().and_then(|active| active.get(&scan_id).cloned()).map(|flag| { flag.store(true, Ordering::Relaxed); true }).unwrap_or(false)
}

#[tauri::command]
fn list_skills(state: State<'_, AppState>, query: SkillQuery) -> CommandResult<PagedSkills> { state.store.list_skills(&query).map_err(err) }

#[tauri::command]
fn get_skill_detail(state: State<'_, AppState>, asset_id: String) -> CommandResult<SkillDetail> { state.store.detail(&asset_id).map_err(err)?.ok_or_else(|| "Skill 不存在或已被移除。".into()) }

#[tauri::command]
fn get_installations(state: State<'_, AppState>, asset_id: String) -> CommandResult<Vec<SkillInstallation>> { Ok(state.store.detail(&asset_id).map_err(err)?.map(|detail| detail.installations).unwrap_or_default()) }

#[tauri::command]
fn list_health_findings(state: State<'_, AppState>, _query: SkillQuery) -> CommandResult<Vec<HealthFinding>> { state.store.findings().map_err(err) }

#[tauri::command]
fn dashboard_summary(state: State<'_, AppState>) -> CommandResult<DashboardSummary> { state.store.dashboard().map_err(err) }

#[tauri::command]
fn list_provider_roots(state: State<'_, AppState>) -> CommandResult<Vec<ProviderRoot>> {
    let mut roots = default_roots(); roots.extend(state.store.custom_roots().map_err(err)?); Ok(roots.into_iter().map(|root| root.root).collect())
}

#[tauri::command]
fn add_custom_root(state: State<'_, AppState>, path: String, provider: String, scope: String) -> CommandResult<()> {
    let path = PathBuf::from(path); if !path.is_dir() { return Err("所选路径不是可读取目录。".into()); }
    let canonical = std::fs::canonicalize(&path).map_err(err)?; let mut hash = Sha256::new(); hash.update(normalized(&canonical));
    state.store.add_custom_root(&format!("custom-{:x}", hash.finalize())[..23], &canonical, &provider, &scope).map_err(err)
}

#[tauri::command]
fn remove_custom_root(state: State<'_, AppState>, root_id: String) -> CommandResult<()> { state.store.remove_custom_root(&root_id).map_err(err) }

#[tauri::command]
fn set_user_classification(state: State<'_, AppState>, asset_id: String, category: String, tags: Vec<String>) -> CommandResult<()> {
    if !CATEGORIES.contains(&category.as_str()) { return Err("未知分类。".into()); }
    let detail = state.store.detail(&asset_id).map_err(err)?.ok_or_else(|| "Skill 不存在。".to_string())?;
    let record = ClassificationRecord { category, tags: tags.into_iter().take(20).collect(), source: "user".into(), rationale: Some("用户本地修正".into()), model: None, content_fingerprint: Some(detail.asset.content_fingerprint), is_stale: false };
    state.store.set_classification(&detail.asset.logical_key, &record).map_err(err)
}

#[tauri::command]
async fn classify_skills(app: tauri::AppHandle, state: State<'_, AppState>, asset_ids: Vec<String>, consent: ClassificationConsent) -> CommandResult<String> {
    if !consent.confirmed || !consent.include_manifest_body { return Err("必须确认发送完整 SKILL.md 后才能分类。".into()); }
    let settings = load_ai_settings(&state.store)?; let api_key = Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(err)?.get_password().map_err(|_| "尚未保存 API Key。".to_string())?;
    let job_id = uuid::Uuid::new_v4().to_string();
    for (index, asset_id) in asset_ids.iter().enumerate() {
        let detail = state.store.detail(asset_id).map_err(err)?.ok_or_else(|| format!("Skill {asset_id} 不存在。"))?;
        let _ = app.emit("classification://progress", serde_json::json!({"jobId":job_id,"current":index,"total":asset_ids.len(),"assetId":asset_id}));
        let record = match state.store.cached_ai_classification(&detail.asset.logical_key, &detail.asset.content_fingerprint, &settings.model, ai::prompt_version()).map_err(err)? {
            Some(cached) => cached,
            None => ai::classify(&settings, &api_key, &detail).await.map_err(err)?,
        };
        state.store.set_classification(&detail.asset.logical_key, &record).map_err(err)?;
    }
    let _ = app.emit("classification://completed", serde_json::json!({"jobId":job_id,"count":asset_ids.len()})); Ok(job_id)
}

#[tauri::command]
fn get_ai_settings(state: State<'_, AppState>) -> CommandResult<AiSettings> { load_ai_settings(&state.store) }

fn load_ai_settings(store: &Store) -> CommandResult<AiSettings> {
    let mut settings: AiSettings = store.setting("ai_settings").map_err(err)?.and_then(|value| serde_json::from_str(&value).ok()).unwrap_or(AiSettings { base_url: "https://api.openai.com/v1".into(), model: "gpt-4.1-mini".into(), custom_headers: Default::default(), has_api_key: false });
    settings.has_api_key = Entry::new(KEYRING_SERVICE, KEYRING_USER).ok().and_then(|entry| entry.get_password().ok()).is_some(); Ok(settings)
}

#[tauri::command]
fn save_ai_settings(state: State<'_, AppState>, mut settings: AiSettings, api_key: Option<String>) -> CommandResult<()> {
    if !(settings.base_url.starts_with("https://") || settings.base_url.starts_with("http://localhost") || settings.base_url.starts_with("http://127.0.0.1")) { return Err("Base URL 必须使用 HTTPS，本机服务除外。".into()); }
    if let Some(key) = api_key.filter(|key| !key.trim().is_empty()) { Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(err)?.set_password(&key).map_err(err)?; }
    settings.has_api_key = false; state.store.set_setting("ai_settings", &serde_json::to_string(&settings).map_err(err)?).map_err(err)
}

#[tauri::command]
fn reveal_in_file_manager(path: String) -> CommandResult<()> {
    let path = Path::new(&path); if !path.exists() { return Err("路径已不存在。".into()); }
    #[cfg(target_os = "windows")]
    let mut command = { let mut value = std::process::Command::new("explorer.exe"); value.arg(path); value };
    #[cfg(target_os = "macos")]
    let mut command = { let mut value = std::process::Command::new("open"); value.arg(path); value };
    #[cfg(target_os = "linux")]
    let mut command = { let mut value = std::process::Command::new("xdg-open"); value.arg(path); value };
    command.spawn().map_err(err)?; Ok(())
}

#[tauri::command]
fn copy_path(app: tauri::AppHandle, path: String) -> CommandResult<()> { app.clipboard().write_text(path).map_err(err) }

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?; let store = Arc::new(Store::open(data_dir.join("skill-atlas.sqlite3"))?);
            app.manage(AppState { store: store.clone(), scans: Arc::new(Mutex::new(HashMap::new())) });
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut roots = default_roots(); if let Ok(custom) = store.custom_roots() { roots.extend(custom); }
                if let Ok(bundle) = scanner::scan(&roots) { let _ = store.replace_scan(bundle); let _ = app_handle.emit("scan://completed", serde_json::json!({"background":true})); }
                let (sender, receiver) = std::sync::mpsc::channel();
                let Ok(mut watcher) = RecommendedWatcher::new(move |event| { let _ = sender.send(event); }, notify::Config::default()) else { return };
                for root in &roots { if root.path.is_dir() { let _ = watcher.watch(&root.path, RecursiveMode::Recursive); } }
                while receiver.recv().is_ok() {
                    while receiver.recv_timeout(std::time::Duration::from_millis(450)).is_ok() {}
                    if let Ok(bundle) = scanner::scan(&roots) { let _ = store.replace_scan(bundle); let _ = app_handle.emit("scan://completed", serde_json::json!({"background":true})); }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![scan_roots,cancel_scan,list_skills,get_skill_detail,get_installations,list_health_findings,dashboard_summary,list_provider_roots,add_custom_root,remove_custom_root,set_user_classification,classify_skills,get_ai_settings,save_ai_settings,reveal_in_file_manager,copy_path])
        .run(tauri::generate_context!()).expect("failed to run Skill Atlas");
}
