use crate::models::*;
use crate::providers::{custom_root, RootSpec};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
pub struct Store {
    path: PathBuf,
    write_lock: Arc<Mutex<()>>,
}

impl Store {
    pub fn open(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let store = Self {
            path,
            write_lock: Arc::new(Mutex::new(())),
        };
        let connection = store.connect()?;
        store.migrate(&connection)?;
        Ok(store)
    }

    fn connect(&self) -> Result<Connection> {
        let connection = Connection::open(&self.path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.busy_timeout(Duration::from_secs(10))?;
        Ok(connection)
    }

    fn migrate(&self, connection: &Connection) -> Result<()> {
        connection.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS assets (
              id TEXT PRIMARY KEY, logical_key TEXT NOT NULL, name TEXT NOT NULL, description TEXT NOT NULL,
              fingerprint TEXT NOT NULL, compatibility TEXT, license TEXT, author TEXT, body_preview TEXT NOT NULL,
              manifest_body TEXT, category TEXT NOT NULL, tags_json TEXT NOT NULL, classification_json TEXT NOT NULL,
              installation_count INTEGER NOT NULL, providers_json TEXT NOT NULL, scopes_json TEXT NOT NULL,
              health TEXT NOT NULL, finding_count INTEGER NOT NULL, has_conflict INTEGER NOT NULL,
              has_scripts INTEGER NOT NULL, updated_at TEXT
            );
            CREATE TABLE IF NOT EXISTS installations (
              id TEXT PRIMARY KEY, asset_id TEXT NOT NULL, provider TEXT NOT NULL, provider_label TEXT NOT NULL,
              scope TEXT NOT NULL, display_path TEXT NOT NULL, normalized_path TEXT NOT NULL, resolved_target TEXT,
              link_type TEXT NOT NULL, read_only INTEGER NOT NULL, modified_at TEXT,
              FOREIGN KEY(asset_id) REFERENCES assets(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS findings (
              id TEXT PRIMARY KEY, asset_id TEXT NOT NULL, code TEXT NOT NULL, severity TEXT NOT NULL,
              title TEXT NOT NULL, detail TEXT NOT NULL, path TEXT,
              FOREIGN KEY(asset_id) REFERENCES assets(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS files (
              asset_id TEXT NOT NULL, path TEXT NOT NULL, size INTEGER NOT NULL, kind TEXT NOT NULL,
              PRIMARY KEY(asset_id, path), FOREIGN KEY(asset_id) REFERENCES assets(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS classifications (
              logical_key TEXT PRIMARY KEY, category TEXT NOT NULL, tags_json TEXT NOT NULL, source TEXT NOT NULL,
              rationale TEXT, model TEXT, content_fingerprint TEXT, updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS custom_roots (
              id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE, provider TEXT NOT NULL, scope TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE VIRTUAL TABLE IF NOT EXISTS assets_fts USING fts5(asset_id UNINDEXED, name, description, body, tags, paths);
        "#)?;
        let _ = connection.execute(
            "ALTER TABLE classifications ADD COLUMN prompt_version TEXT",
            [],
        );
        Ok(())
    }

    pub fn replace_scan(&self, mut bundle: ScanBundle) -> Result<()> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| anyhow::anyhow!("索引写入锁不可用"))?;
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let classifications = load_classifications(&transaction)?;
        for asset in &mut bundle.assets {
            if let Some(saved) = classifications.get(&asset.logical_key) {
                let stale = saved
                    .content_fingerprint
                    .as_deref()
                    .map(|fingerprint| fingerprint != asset.content_fingerprint)
                    .unwrap_or(false);
                if saved.source == "user" || !stale {
                    asset.category = saved.category.clone();
                    asset.tags = saved.tags.clone();
                    asset.classification = ClassificationRecord {
                        is_stale: stale,
                        ..saved.clone()
                    };
                } else {
                    asset.classification.is_stale = true;
                }
            }
        }
        transaction.execute("DELETE FROM assets_fts", [])?;
        transaction.execute("DELETE FROM assets", [])?;
        for asset in &bundle.assets {
            insert_asset(&transaction, asset)?;
        }
        for installation in &bundle.installations {
            insert_installation(&transaction, installation)?;
        }
        for finding in &bundle.findings {
            insert_finding(&transaction, finding)?;
        }
        for (asset_id, file) in &bundle.files {
            transaction.execute(
                "INSERT INTO files(asset_id,path,size,kind) VALUES(?1,?2,?3,?4)",
                params![asset_id, file.path, file.size, file.kind],
            )?;
        }
        for asset in &bundle.assets {
            let paths = bundle
                .installations
                .iter()
                .filter(|item| item.asset_id == asset.id)
                .map(|item| item.display_path.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            transaction.execute("INSERT INTO assets_fts(asset_id,name,description,body,tags,paths) VALUES(?1,?2,?3,?4,?5,?6)", params![asset.id, asset.name, asset.description, asset.manifest_body.as_deref().unwrap_or_default(), asset.tags.join(" "), paths])?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn list_skills(&self, query: &SkillQuery) -> Result<PagedSkills> {
        let connection = self.connect()?;
        let mut statement =
            connection.prepare("SELECT id FROM assets ORDER BY name COLLATE NOCASE")?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut assets = Vec::new();
        for id in ids {
            if let Some(asset) = self.asset_with(&connection, &id)? {
                if matches_query(&connection, &asset, query)? {
                    assets.push(asset);
                }
            }
        }
        let total = assets.len();
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(100).clamp(1, 500);
        let start = (page - 1) * page_size;
        let items = assets.into_iter().skip(start).take(page_size).collect();
        Ok(PagedSkills {
            items,
            total,
            page,
            page_size,
        })
    }

    pub fn detail(&self, asset_id: &str) -> Result<Option<SkillDetail>> {
        let connection = self.connect()?;
        let Some(asset) = self.asset_with(&connection, asset_id)? else {
            return Ok(None);
        };
        let installations = load_installations(&connection, asset_id)?;
        let findings = load_findings(&connection, asset_id)?;
        let mut statement = connection.prepare(
            "SELECT path,size,kind FROM files WHERE asset_id=?1 ORDER BY kind DESC,path",
        )?;
        let files = statement
            .query_map([asset_id], |row| {
                Ok(FileEntry {
                    path: row.get(0)?,
                    size: row.get(1)?,
                    kind: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(Some(SkillDetail {
            asset,
            installations,
            findings,
            files,
        }))
    }

    fn asset_with(&self, connection: &Connection, asset_id: &str) -> Result<Option<SkillAsset>> {
        connection.query_row("SELECT id,logical_key,name,description,fingerprint,compatibility,license,author,body_preview,manifest_body,category,tags_json,classification_json,installation_count,providers_json,scopes_json,health,finding_count,has_conflict,has_scripts,updated_at FROM assets WHERE id=?1", [asset_id], asset_from_row).optional().map_err(Into::into)
    }

    pub fn dashboard(&self) -> Result<DashboardSummary> {
        let connection = self.connect()?;
        let count = |sql: &str| -> Result<usize> {
            Ok(connection.query_row(sql, [], |row| row.get::<_, i64>(0))? as usize)
        };
        let mut categories = Vec::new();
        for category in CATEGORIES {
            let value = connection.query_row(
                "SELECT COUNT(*) FROM assets WHERE category=?1",
                [category],
                |row| row.get::<_, i64>(0),
            )? as usize;
            categories.push(CategoryCount {
                category: category.into(),
                count: value,
            });
        }
        Ok(DashboardSummary {
            assets: count("SELECT COUNT(*) FROM assets")?,
            installations: count("SELECT COUNT(*) FROM installations")?,
            providers: count("SELECT COUNT(DISTINCT provider) FROM installations")?,
            findings: count("SELECT COUNT(*) FROM findings WHERE severity != 'info'")?,
            conflicts: count("SELECT COUNT(*) FROM assets WHERE has_conflict=1")?,
            categories,
        })
    }

    pub fn findings(&self) -> Result<Vec<HealthFinding>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare("SELECT id,asset_id,code,severity,title,detail,path FROM findings ORDER BY CASE severity WHEN 'error' THEN 0 WHEN 'warning' THEN 1 ELSE 2 END,title")?;
        let findings = statement
            .query_map([], |row| {
                Ok(HealthFinding {
                    id: row.get(0)?,
                    asset_id: row.get(1)?,
                    code: row.get(2)?,
                    severity: row.get(3)?,
                    title: row.get(4)?,
                    detail: row.get(5)?,
                    path: row.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(findings)
    }

    pub fn set_classification(
        &self,
        logical_key: &str,
        record: &ClassificationRecord,
    ) -> Result<()> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| anyhow::anyhow!("索引写入锁不可用"))?;
        let connection = self.connect()?;
        let prompt_version = (record.source == "ai").then_some("skill-atlas-classifier-v2");
        connection.execute("INSERT INTO classifications(logical_key,category,tags_json,source,rationale,model,content_fingerprint,updated_at,prompt_version) VALUES(?1,?2,?3,?4,?5,?6,?7,datetime('now'),?8) ON CONFLICT(logical_key) DO UPDATE SET category=excluded.category,tags_json=excluded.tags_json,source=excluded.source,rationale=excluded.rationale,model=excluded.model,content_fingerprint=excluded.content_fingerprint,updated_at=excluded.updated_at,prompt_version=excluded.prompt_version", params![logical_key, record.category, serde_json::to_string(&record.tags)?, record.source, record.rationale, record.model, record.content_fingerprint, prompt_version])?;
        connection.execute("UPDATE assets SET category=?1,tags_json=?2,classification_json=?3 WHERE logical_key=?4", params![record.category, serde_json::to_string(&record.tags)?, serde_json::to_string(record)?, logical_key])?;
        Ok(())
    }

    pub fn cached_ai_classification(
        &self,
        logical_key: &str,
        fingerprint: &str,
        model: &str,
        prompt_version: &str,
    ) -> Result<Option<ClassificationRecord>> {
        let connection = self.connect()?;
        connection.query_row(
            "SELECT category,tags_json,source,rationale,model,content_fingerprint FROM classifications WHERE logical_key=?1 AND source='ai' AND content_fingerprint=?2 AND model=?3 AND prompt_version=?4",
            params![logical_key, fingerprint, model, prompt_version],
            |row| Ok(ClassificationRecord { category: row.get(0)?, tags: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or_default(), source: row.get(2)?, rationale: row.get(3)?, model: row.get(4)?, content_fingerprint: row.get(5)?, is_stale: false }),
        ).optional().map_err(Into::into)
    }

    pub fn custom_roots(&self) -> Result<Vec<RootSpec>> {
        let connection = self.connect()?;
        let mut statement =
            connection.prepare("SELECT id,path,provider,scope FROM custom_roots ORDER BY path")?;
        let roots = statement
            .query_map([], |row| {
                Ok(custom_root(
                    row.get(0)?,
                    PathBuf::from(row.get::<_, String>(1)?),
                    row.get(2)?,
                    row.get(3)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(roots)
    }

    pub fn add_custom_root(
        &self,
        id: &str,
        path: &Path,
        provider: &str,
        scope: &str,
    ) -> Result<()> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| anyhow::anyhow!("索引写入锁不可用"))?;
        self.connect()?.execute(
            "INSERT OR IGNORE INTO custom_roots(id,path,provider,scope) VALUES(?1,?2,?3,?4)",
            params![id, path.to_string_lossy(), provider, scope],
        )?;
        Ok(())
    }

    pub fn remove_custom_root(&self, id: &str) -> Result<()> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| anyhow::anyhow!("索引写入锁不可用"))?;
        self.connect()?
            .execute("DELETE FROM custom_roots WHERE id=?1", [id])?;
        Ok(())
    }
    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .connect()?
            .query_row("SELECT value FROM settings WHERE key=?1", [key], |row| {
                row.get(0)
            })
            .optional()?)
    }
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| anyhow::anyhow!("索引写入锁不可用"))?;
        self.connect()?.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key,value])?;
        Ok(())
    }
}

fn insert_asset(tx: &Transaction<'_>, asset: &SkillAsset) -> Result<()> {
    tx.execute("INSERT INTO assets VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21)", params![asset.id,asset.logical_key,asset.name,asset.description,asset.content_fingerprint,asset.compatibility,asset.license,asset.author,asset.body_preview,asset.manifest_body,asset.category,serde_json::to_string(&asset.tags)?,serde_json::to_string(&asset.classification)?,asset.installation_count,serde_json::to_string(&asset.providers)?,serde_json::to_string(&asset.scopes)?,asset.health,asset.finding_count,asset.has_conflict,asset.has_scripts,asset.updated_at])?;
    Ok(())
}
fn insert_installation(tx: &Transaction<'_>, item: &SkillInstallation) -> Result<()> {
    tx.execute(
        "INSERT INTO installations VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            item.id,
            item.asset_id,
            item.provider,
            item.provider_label,
            item.scope,
            item.display_path,
            item.normalized_path,
            item.resolved_target,
            item.link_type,
            item.read_only,
            item.modified_at
        ],
    )?;
    Ok(())
}
fn insert_finding(tx: &Transaction<'_>, item: &HealthFinding) -> Result<()> {
    tx.execute(
        "INSERT INTO findings VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![
            item.id,
            item.asset_id,
            item.code,
            item.severity,
            item.title,
            item.detail,
            item.path
        ],
    )?;
    Ok(())
}

fn load_classifications(connection: &Connection) -> Result<HashMap<String, ClassificationRecord>> {
    let mut statement = connection.prepare("SELECT logical_key,category,tags_json,source,rationale,model,content_fingerprint FROM classifications")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            ClassificationRecord {
                category: row.get(1)?,
                tags: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or_default(),
                source: row.get(3)?,
                rationale: row.get(4)?,
                model: row.get(5)?,
                content_fingerprint: row.get(6)?,
                is_stale: false,
            },
        ))
    })?;
    Ok(rows.collect::<rusqlite::Result<HashMap<_, _>>>()?)
}

fn asset_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SkillAsset> {
    Ok(SkillAsset {
        id: row.get(0)?,
        logical_key: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        content_fingerprint: row.get(4)?,
        compatibility: row.get(5)?,
        license: row.get(6)?,
        author: row.get(7)?,
        body_preview: row.get(8)?,
        manifest_body: row.get(9)?,
        category: row.get(10)?,
        tags: serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or_default(),
        classification: serde_json::from_str(&row.get::<_, String>(12)?).unwrap_or(
            ClassificationRecord {
                category: "其他".into(),
                tags: vec![],
                source: "none".into(),
                rationale: None,
                model: None,
                content_fingerprint: None,
                is_stale: false,
            },
        ),
        installation_count: row.get(13)?,
        providers: serde_json::from_str(&row.get::<_, String>(14)?).unwrap_or_default(),
        scopes: serde_json::from_str(&row.get::<_, String>(15)?).unwrap_or_default(),
        health: row.get(16)?,
        finding_count: row.get(17)?,
        has_conflict: row.get(18)?,
        has_scripts: row.get(19)?,
        updated_at: row.get(20)?,
    })
}

fn load_installations(connection: &Connection, asset_id: &str) -> Result<Vec<SkillInstallation>> {
    let mut statement = connection.prepare("SELECT id,asset_id,provider,provider_label,scope,display_path,normalized_path,resolved_target,link_type,read_only,modified_at FROM installations WHERE asset_id=?1 ORDER BY provider,display_path")?;
    let installations = statement
        .query_map([asset_id], |row| {
            Ok(SkillInstallation {
                id: row.get(0)?,
                asset_id: row.get(1)?,
                provider: row.get(2)?,
                provider_label: row.get(3)?,
                scope: row.get(4)?,
                display_path: row.get(5)?,
                normalized_path: row.get(6)?,
                resolved_target: row.get(7)?,
                link_type: row.get(8)?,
                read_only: row.get(9)?,
                modified_at: row.get(10)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(installations)
}
fn load_findings(connection: &Connection, asset_id: &str) -> Result<Vec<HealthFinding>> {
    let mut statement = connection.prepare(
        "SELECT id,asset_id,code,severity,title,detail,path FROM findings WHERE asset_id=?1",
    )?;
    let findings = statement
        .query_map([asset_id], |row| {
            Ok(HealthFinding {
                id: row.get(0)?,
                asset_id: row.get(1)?,
                code: row.get(2)?,
                severity: row.get(3)?,
                title: row.get(4)?,
                detail: row.get(5)?,
                path: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(findings)
}

fn matches_query(connection: &Connection, asset: &SkillAsset, query: &SkillQuery) -> Result<bool> {
    if let Some(category) = &query.category {
        if &asset.category != category {
            return Ok(false);
        }
    }
    if let Some(health) = &query.health {
        if &asset.health != health {
            return Ok(false);
        }
    }
    if query.conflicts_only.unwrap_or(false) && !asset.has_conflict {
        return Ok(false);
    }
    if let Some(provider) = &query.provider {
        if !asset.providers.contains(provider) {
            return Ok(false);
        }
    }
    if let Some(scope) = &query.scope {
        if !asset.scopes.contains(scope) {
            return Ok(false);
        }
    }
    if let Some(search) = query
        .search
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        let needle = search.to_lowercase();
        let installations = load_installations(connection, &asset.id)?;
        let haystack = format!(
            "{} {} {} {} {}",
            asset.name,
            asset.description,
            asset.manifest_body.as_deref().unwrap_or_default(),
            asset.tags.join(" "),
            installations
                .iter()
                .map(|item| item.display_path.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        )
        .to_lowercase();
        if !haystack.contains(&needle) {
            return Ok(false);
        }
    }
    Ok(true)
}
