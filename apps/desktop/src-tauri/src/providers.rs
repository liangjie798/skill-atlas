use crate::models::ProviderRoot;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct RootSpec {
    pub root: ProviderRoot,
    pub path: PathBuf,
    pub max_depth: usize,
}

fn normalize(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        value.to_lowercase()
    } else {
        value
    }
}

fn root(
    id: &str,
    provider: &str,
    label: &str,
    scope: &str,
    path: PathBuf,
    max_depth: usize,
) -> RootSpec {
    RootSpec {
        root: ProviderRoot {
            id: id.into(),
            provider: provider.into(),
            provider_label: label.into(),
            scope: scope.into(),
            display_path: path.to_string_lossy().into_owned(),
            normalized_path: normalize(&path),
            exists: path.is_dir(),
            custom: false,
            read_only: true,
        },
        path,
        max_depth,
    }
}

pub fn default_roots() -> Vec<RootSpec> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    vec![
        root(
            "codex-user",
            "codex",
            "Codex",
            "user",
            home.join(".codex/skills"),
            5,
        ),
        root(
            "codex-plugins",
            "codex",
            "Codex Plugins",
            "plugin",
            home.join(".codex/plugins/cache"),
            12,
        ),
        root(
            "claude-user",
            "claude-code",
            "Claude Code",
            "user",
            home.join(".claude/skills"),
            6,
        ),
        root(
            "cursor-user",
            "cursor",
            "Cursor",
            "user",
            home.join(".cursor/skills"),
            6,
        ),
        root(
            "gemini-user",
            "gemini-cli",
            "Gemini CLI",
            "user",
            home.join(".gemini/skills"),
            6,
        ),
        root(
            "shared-user",
            "shared",
            "Shared",
            "user",
            home.join(".agents/skills"),
            6,
        ),
    ]
}

pub fn custom_root(id: String, path: PathBuf, provider: String, scope: String) -> RootSpec {
    RootSpec {
        root: ProviderRoot {
            id,
            provider,
            provider_label: "Custom".into(),
            scope,
            display_path: path.to_string_lossy().into_owned(),
            normalized_path: normalize(&path),
            exists: path.is_dir(),
            custom: true,
            read_only: true,
        },
        path,
        max_depth: 12,
    }
}

pub fn normalized(path: &Path) -> String {
    normalize(path)
}
