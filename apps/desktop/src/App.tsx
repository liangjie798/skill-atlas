import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowsClockwise, Brain, CheckCircle, Clipboard, Code, FolderOpen, Funnel,
  Gear, GitDiff, Globe, GridFour, HardDrives, Heartbeat, MagnifyingGlass, Path,
  Plus, Robot, ShieldWarning, SidebarSimple, Sparkle, Stack, Tag, Warning,
  ArrowSquareOut,
  X,
} from "@phosphor-icons/react";
import type { AiModelProfile, AiSettings, DashboardSummary, OnlineSkillResult, SkillAsset, SkillCategory, SkillDetail, SkillQuery } from "@skill-atlas/contracts";
import { SKILL_CATEGORIES } from "@skill-atlas/contracts";
import { api } from "./api";

type NavView = "library" | "agents" | "conflicts" | "health" | "categories" | "discover";
type DetailTab = "overview" | "manifest" | "files";

const categoryShort: Record<SkillCategory, string> = {
  "编程开发": "开发", "测试与质量": "测试", "DevOps 与云": "运维", "数据与 AI": "数据",
  "设计与媒体": "设计", "文档与办公": "文档", "研究与知识": "研究", "业务与营销": "业务",
  "安全与合规": "安全", "Agent 与工具": "Agent", "其他": "其他",
};

const providerLabels: Record<string, string> = {
  codex: "Codex", "claude-code": "Claude Code", cursor: "Cursor", "gemini-cli": "Gemini CLI", shared: "Shared", custom: "Custom",
};

const aiProviders = [
  { id: "openai", name: "OpenAI", baseUrl: "https://api.openai.com/v1", model: "gpt-5.5", apiMode: "responses-web-search" as const },
  { id: "deepseek", name: "DeepSeek", baseUrl: "https://api.deepseek.com/v1", model: "deepseek-chat", apiMode: "chat-completions" as const },
  { id: "siliconflow", name: "硅基流动", baseUrl: "https://api.siliconflow.cn/v1", model: "", apiMode: "chat-completions" as const },
  { id: "moonshot", name: "Moonshot / Kimi", baseUrl: "https://api.moonshot.cn/v1", model: "", apiMode: "chat-completions" as const },
  { id: "openrouter", name: "OpenRouter", baseUrl: "https://openrouter.ai/api/v1", model: "", apiMode: "chat-completions" as const },
  { id: "ollama", name: "Ollama（本机）", baseUrl: "http://127.0.0.1:11434/v1", model: "", apiMode: "chat-completions" as const },
  { id: "custom", name: "自定义 OpenAI-compatible", baseUrl: "https://", model: "", apiMode: "chat-completions" as const },
];

function healthLabel(health: SkillAsset["health"]) {
  return health === "healthy" ? "健康" : health === "attention" ? "需关注" : "有错误";
}

function chineseError(reason: unknown, fallback = "操作失败") {
  const message = reason instanceof Error ? reason.message : String(reason ?? "");
  if (/database is (locked|busy)/i.test(message)) return "本地索引正在更新，请稍后重试。";
  if (/UNIQUE constraint failed: findings\.id/i.test(message)) return "健康检查记录重复，请升级到最新版本后重新扫描。";
  if (/network|fetch|connect|timeout/i.test(message)) return "网络连接失败，请检查模型服务与网络设置。";
  return message || fallback;
}

export default function App() {
  const [view, setView] = useState<NavView>("library");
  const [query, setQuery] = useState<SkillQuery>({ pageSize: 100 });
  const [assets, setAssets] = useState<SkillAsset[]>([]);
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [selectedId, setSelectedId] = useState<string>();
  const [detail, setDetail] = useState<SkillDetail>();
  const [detailTab, setDetailTab] = useState<DetailTab>("overview");
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [scanProgress, setScanProgress] = useState({ value: 0, currentRoot: 0, totalRoots: 0 });
  const activeScan = useRef<string | undefined>(undefined);
  const [notice, setNotice] = useState<string>();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [classifyOpen, setClassifyOpen] = useState(false);

  const effectiveQuery = useMemo<SkillQuery>(() => ({
    ...query,
    conflictsOnly: view === "conflicts" || query.conflictsOnly,
    health: view === "health" ? query.health ?? "attention" : query.health,
  }), [query, view]);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [result, nextSummary] = await Promise.all([api.listSkills(effectiveQuery), api.dashboardSummary()]);
      setAssets(result.items); setSummary(nextSummary);
      setSelectedId(current => result.items.some(item => item.id === current) ? current : result.items[0]?.id);
    } catch (error) { setNotice(chineseError(error, "加载本地索引失败。")); }
    finally { setLoading(false); }
  }, [effectiveQuery]);

  useEffect(() => {
    void load();
    const retry = window.setTimeout(() => void load(), 1200);
    let unlisten: (() => void) | undefined;
    let unlistenFailed: (() => void) | undefined;
    let unlistenProgress: (() => void) | undefined;
    if (api.isTauri()) void import("@tauri-apps/api/event").then(async ({ listen }) => {
      unlisten = await listen<{ scanId?: string; assets?: number; installations?: number; durationMs?: number }>("scan://completed", event => {
        void load();
        if (event.payload.scanId === activeScan.current) {
          setScanning(false); activeScan.current = undefined;
          setScanProgress({ value: 100, currentRoot: 0, totalRoots: 0 });
          setNotice(`扫描完成：${event.payload.assets ?? 0} 个资产，${event.payload.installations ?? 0} 个安装实例，用时 ${event.payload.durationMs ?? 0} ms`);
        }
      });
      unlistenFailed = await listen<{ scanId: string; error: string }>("scan://failed", event => {
        if (event.payload.scanId === activeScan.current) { setScanning(false); activeScan.current = undefined; setNotice(`扫描失败：${chineseError(event.payload.error, "请稍后重试。")}`); }
      });
      unlistenProgress = await listen<{ scanId: string; progress: number; currentRoot?: number; totalRoots?: number }>("scan://progress", event => {
        if (event.payload.scanId === activeScan.current) setScanProgress({ value: event.payload.progress, currentRoot: event.payload.currentRoot ?? 0, totalRoots: event.payload.totalRoots ?? 0 });
      });
    });
    return () => { window.clearTimeout(retry); unlisten?.(); unlistenFailed?.(); unlistenProgress?.(); };
  }, [load]);
  useEffect(() => {
    if (!selectedId) { setDetail(undefined); return; }
    void api.getSkillDetail(selectedId).then(setDetail).catch(error => setNotice(chineseError(error, "读取 Skill 详情失败。")));
  }, [selectedId]);

  async function scan() {
    setScanning(true); setScanProgress({ value: 0, currentRoot: 0, totalRoots: 0 }); setNotice(undefined);
    try {
      activeScan.current = await api.scanRoots();
      setNotice("扫描任务已启动");
    } catch (error) { setScanning(false); setNotice(chineseError(error, "扫描失败。")); }
    if (!api.isTauri()) {
      setScanProgress({ value: 46, currentRoot: 1, totalRoots: 3 });
      await new Promise(resolve => window.setTimeout(resolve, 180));
      setScanProgress({ value: 82, currentRoot: 2, totalRoots: 3 });
      await new Promise(resolve => window.setTimeout(resolve, 180));
      setScanProgress({ value: 100, currentRoot: 3, totalRoots: 3 });
      await load();
      activeScan.current = undefined; setScanning(false);
      setNotice("扫描完成：浏览器预览数据已刷新");
    }
  }

  async function cancelScan() {
    if (activeScan.current) { await api.cancelScan(activeScan.current); setNotice("正在取消扫描"); }
  }

  async function addRoot() {
    if (!api.isTauri()) { setNotice("浏览器预览模式不会访问本地文件系统"); return; }
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({ directory: true, multiple: false, title: "选择 Skill 根目录" });
    if (typeof selected === "string") { await api.addCustomRoot(selected); await scan(); }
  }

  return (
    <div className={scanning ? "app-shell scanning" : "app-shell"}>
      <header className="topbar" data-tauri-drag-region>
        <div className="brand"><img src="/app-icon-source.png" alt="" /><strong>Skill Atlas</strong><span className="read-only">只读模式</span></div>
        <div className="top-actions">
          <button className="button quiet" onClick={() => void addRoot()}><Plus size={16} />添加目录</button>
          <button className={scanning ? "button quiet" : "button primary"} onClick={() => void (scanning ? cancelScan() : scan())}><ArrowsClockwise size={16} className={scanning ? "spin" : ""} />{scanning ? "取消扫描" : "重新扫描"}</button>
          <button className="icon-button" aria-label="设置" onClick={() => setSettingsOpen(true)}><Gear size={19} /></button>
        </div>
      </header>
      {scanning && <div className="scan-progress" role="progressbar" aria-label="Skill 扫描进度" aria-valuemin={0} aria-valuemax={100} aria-valuenow={scanProgress.value}><div className="scan-progress-copy"><span>正在扫描本地 Skill</span><b>{scanProgress.totalRoots > 0 ? `目录 ${Math.min(scanProgress.currentRoot + 1, scanProgress.totalRoots)}/${scanProgress.totalRoots}` : "正在准备"}</b><strong>{scanProgress.value}%</strong></div><div className="scan-progress-track"><i style={{ transform: `scaleX(${scanProgress.value / 100})` }} /></div></div>}

      <div className="workspace">
        <aside className="sidebar">
          <nav aria-label="资产视图">
            <NavButton active={view === "library"} icon={<GridFour />} label="资产库" count={summary?.assets} onClick={() => { setView("library"); setQuery({ pageSize: 100 }); }} />
            <NavButton active={view === "agents"} icon={<Robot />} label="按 Agent" count={summary?.providers} onClick={() => setView("agents")} />
            <NavButton active={view === "categories"} icon={<Tag />} label="分类" onClick={() => setView("categories")} />
            <NavButton active={view === "conflicts"} icon={<GitDiff />} label="冲突" count={summary?.conflicts} tone="danger" onClick={() => { setView("conflicts"); setQuery(q => ({ ...q, conflictsOnly: true })); }} />
            <NavButton active={view === "health"} icon={<Heartbeat />} label="健康检查" count={summary?.findings} tone="warning" onClick={() => { setView("health"); setQuery(q => ({ ...q, health: "attention" })); }} />
            <NavButton active={view === "discover"} icon={<Globe />} label="联网发现" onClick={() => setView("discover")} />
          </nav>

          <div className="sidebar-section">
            <div className="sidebar-heading"><span>分类</span><button aria-label="管理分类"><Gear size={14} /></button></div>
            <div className="category-list">
              {SKILL_CATEGORIES.map(category => {
                const count = summary?.categories.find(item => item.category === category)?.count ?? 0;
                return <button key={category} className={query.category === category ? "category active" : "category"} onClick={() => setQuery(q => ({ ...q, category: q.category === category ? undefined : category }))}><span>{categoryShort[category]}</span><b>{count}</b></button>;
              })}
            </div>
          </div>

          <div className="sidebar-footer"><ShieldWarning size={16} /><span>不会修改 Skill 文件</span></div>
        </aside>

        {view === "discover" ? <OnlineDiscovery onNotice={setNotice} /> : <section className="library-panel">
          <div className="panel-toolbar">
            <div className="search"><MagnifyingGlass size={17} /><input value={query.search ?? ""} onChange={event => setQuery(q => ({ ...q, search: event.target.value }))} placeholder="搜索名称、用途、标签或路径" aria-label="搜索 Skill" /></div>
            <button className="filter-button" aria-label="过滤器"><Funnel size={17} /><span>{assets.length}</span></button>
          </div>

          <div className="list-heading"><div><h1>{view === "conflicts" ? "内容冲突" : view === "health" ? "健康检查" : "Skill 资产"}</h1><p>{summary ? `${summary.assets} 个逻辑资产，${summary.installations} 个安装实例` : "正在读取本地索引"}</p></div><SidebarSimple size={18} /></div>

          <div className="skill-list" aria-live="polite">
            {loading ? <SkeletonList /> : assets.length === 0 ? <EmptyState onReset={() => { setView("library"); setQuery({ pageSize: 100 }); }} /> : assets.map(asset => (
              <button key={asset.id} className={selectedId === asset.id ? "skill-row selected" : "skill-row"} onClick={() => { setSelectedId(asset.id); setDetailTab("overview"); }}>
                <div className="skill-glyph"><Code size={18} weight="bold" /></div>
                <div className="skill-copy"><div className="skill-title"><strong>{asset.name}</strong>{asset.hasConflict && <span className="badge danger">冲突</span>}{asset.hasScripts && <span className="badge">脚本</span>}</div><p>{asset.description || "未提供描述"}</p><div className="skill-meta"><span>{asset.category}</span><span>{asset.providers.map(p => providerLabels[p]).join(" + ")}</span><span>{asset.installationCount} 处安装</span></div></div>
                <div className={`health ${asset.health}`} title={healthLabel(asset.health)}>{asset.health === "healthy" ? <CheckCircle weight="fill" /> : asset.health === "attention" ? <Warning weight="fill" /> : <ShieldWarning weight="fill" />}</div>
              </button>
            ))}
          </div>
        </section>}

        <aside className="inspector">
          {view === "discover" ? <div className="no-selection"><Globe size={30} /><p>搜索结果来自模型联网检索，不会自动安装或写入本地索引。</p></div> : detail ? <>
            <div className="inspector-head"><div className="large-glyph"><Code size={24} weight="bold" /></div><div><div className="eyeline">{detail.category}</div><h2>{detail.name}</h2></div></div>
            <p className="description">{detail.description || "未提供描述"}</p>
            <div className="inspector-actions"><button className="button primary compact" onClick={() => setClassifyOpen(true)}><Sparkle size={15} />AI 分类</button><button className="button quiet compact" onClick={() => detail.installations[0] && void api.reveal(detail.installations[0].displayPath)}><FolderOpen size={15} />打开目录</button></div>
            <div className="tabs" role="tablist">
              {(["overview", "manifest", "files"] as const).map(tab => <button key={tab} className={detailTab === tab ? "active" : ""} onClick={() => setDetailTab(tab)}>{tab === "overview" ? "概览" : tab === "manifest" ? "SKILL.md" : "文件"}</button>)}
            </div>
            <div className="inspector-body">
              {detailTab === "overview" && <Overview detail={detail} onCopy={path => void api.copyPath(path)} />}
              {detailTab === "manifest" && <pre className="manifest">{detail.manifestBody || "无法读取 SKILL.md"}</pre>}
              {detailTab === "files" && <div className="file-list">{detail.files.map(file => <div key={file.path}><Path size={15} /><span>{file.path}</span><b>{file.kind === "file" ? formatBytes(file.size) : "目录"}</b></div>)}</div>}
            </div>
          </> : <div className="no-selection"><Stack size={30} /><p>选择一个 Skill 查看详情</p></div>}
        </aside>
      </div>

      {settingsOpen && <SettingsModal onClose={() => setSettingsOpen(false)} />}
      {classifyOpen && detail && <ClassificationModal detail={detail} onClose={() => setClassifyOpen(false)} onSaved={async () => { setClassifyOpen(false); await load(); if (selectedId) setDetail(await api.getSkillDetail(selectedId)); }} />}
      {notice && <div className={scanning ? "global-notice below-progress" : "global-notice"} role="status"><CheckCircle size={16} /><span>{notice}</span><button aria-label="关闭通知" onClick={() => setNotice(undefined)}><X size={14} /></button></div>}
    </div>
  );
}

function NavButton({ active, icon, label, count, tone, onClick }: { active: boolean; icon: React.ReactElement; label: string; count?: number; tone?: string; onClick: () => void }) {
  return <button className={active ? "nav-button active" : "nav-button"} onClick={onClick}>{icon}<span>{label}</span>{count !== undefined && <b className={tone}>{count}</b>}</button>;
}

function Overview({ detail, onCopy }: { detail: SkillDetail; onCopy: (path: string) => void }) {
  return <>
    <section className="detail-section"><h3>分类</h3><div className="classification"><div><b>{detail.category}</b><span>{detail.classification.source === "user" ? "用户修正" : detail.classification.source === "ai" ? "AI 分类" : detail.classification.source === "rule" ? "规则推断" : "未分类"}</span></div>{detail.classification.rationale && <p>{detail.classification.rationale}</p>}<div className="tags">{detail.tags.map(tag => <span key={tag}>{tag}</span>)}</div></div></section>
    <section className="detail-section"><h3>安装位置</h3>{detail.installations.map(item => <div className="installation" key={item.id}><div><b>{item.providerLabel}</b><span>{item.scope} / {item.linkType}</span></div><button title="复制路径" onClick={() => onCopy(item.displayPath)}><Clipboard size={15} /></button><code>{item.displayPath}</code>{item.resolvedTarget && <small>目标：{item.resolvedTarget}</small>}</div>)}</section>
    <section className="detail-section"><h3>健康发现</h3>{detail.findings.length === 0 ? <div className="healthy-message"><CheckCircle size={18} weight="fill" />未发现结构或路径问题</div> : detail.findings.map(finding => <div className={`finding ${finding.severity}`} key={finding.id}><Warning size={17} weight="fill" /><div><b>{finding.title}</b><p>{finding.detail}</p></div></div>)}</section>
    <section className="detail-section facts"><h3>资产信息</h3><dl><div><dt>内容指纹</dt><dd>{detail.contentFingerprint.slice(0, 12)}</dd></div><div><dt>安装实例</dt><dd>{detail.installationCount}</dd></div><div><dt>许可证</dt><dd>{detail.license || "未声明"}</dd></div><div><dt>兼容性</dt><dd>{detail.compatibility || "未声明"}</dd></div></dl></section>
  </>;
}

function OnlineDiscovery({ onNotice }: { onNotice: (message: string) => void }) {
  const [query, setQuery] = useState("");
  const [settings, setSettings] = useState<AiSettings>();
  const [profileId, setProfileId] = useState("");
  const [results, setResults] = useState<OnlineSkillResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => { void api.getAiSettings().then(value => { setSettings(value); setProfileId(value.activeProfileId); }).catch(reason => setError(chineseError(reason, "读取模型配置失败。"))); }, []);
  async function search() {
    if (!query.trim()) { setError("请输入想查找的 Skill 用途或关键词。"); return; }
    setSearching(true); setError(""); setResults([]);
    try { const items = await api.searchOnline(query.trim(), profileId); setResults(items); onNotice(`联网搜索完成：找到 ${items.length} 个可核验结果。`); }
    catch (reason) { setError(chineseError(reason, "联网搜索失败。")); }
    finally { setSearching(false); }
  }
  return <section className="library-panel discovery-panel"><div className="discovery-head"><div><span>模型联网搜索</span><h1>发现公开 Skill</h1><p>搜索词会发送给所选模型。结果不会自动安装，也不会写入本地资产库。</p></div><Globe size={28} /></div><div className="discovery-controls"><div className="search large"><MagnifyingGlass size={17} /><input value={query} onChange={event => setQuery(event.target.value)} onKeyDown={event => { if (event.key === "Enter" && !searching) void search(); }} placeholder="例如：适合 React 可访问性审查的 Skill" aria-label="联网搜索 Skill" /></div><select value={profileId} onChange={event => setProfileId(event.target.value)} aria-label="搜索模型">{settings?.profiles.map(profile => <option key={profile.id} value={profile.id}>{profile.name} · {profile.model || "未配置"}</option>)}</select><button className="button primary" disabled={searching || !profileId} onClick={() => void search()}><Globe size={16} />{searching ? "模型正在搜索" : "确认联网搜索"}</button></div>{error && <div className="discovery-error" role="alert"><Warning size={17} /><span>{error}</span></div>}<div className="online-results" aria-live="polite">{searching ? <SkeletonList /> : results.length > 0 ? results.map(result => <article className="online-result" key={`${result.name}-${result.sourceUrl}`}><div className="online-result-head"><div><h2>{result.name}</h2>{result.author && <span>{result.author}</span>}</div><button className="icon-button" aria-label={`打开 ${result.name} 来源`} onClick={() => void api.openUrl(result.sourceUrl)}><ArrowSquareOut size={17} /></button></div><p>{result.description}</p><small>{result.whyRelevant}</small><div className="tags">{result.tags.map(tag => <span key={tag}>{tag}</span>)}</div><button className="source-link" onClick={() => void api.openUrl(result.sourceUrl)}>{result.sourceUrl}</button></article>) : <div className="discovery-empty"><Globe size={36} /><h2>通过模型搜索公开 Skill</h2><p>选择已配置的搜索模型，输入用途、技术栈或工作场景。每条结果都必须包含可点击来源。</p></div>}</div></section>;
}

function SettingsModal({ onClose }: { onClose: () => void }) {
  const [settings, setSettings] = useState<AiSettings>({ enabled: true, activeProfileId: "openai-default", profiles: [{ id: "openai-default", provider: "openai", name: "OpenAI", baseUrl: "https://api.openai.com/v1", model: "gpt-5.5", apiMode: "responses-web-search", customHeaders: {}, hasApiKey: false }] });
  const [apiKey, setApiKey] = useState("");
  const [headersJson, setHeadersJson] = useState("{}");
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [fetchingModels, setFetchingModels] = useState(false);
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);
  const active = settings.profiles.find(profile => profile.id === settings.activeProfileId) ?? settings.profiles[0];
  useEffect(() => { void api.getAiSettings().then(value => { setSettings(value); const profile = value.profiles.find(item => item.id === value.activeProfileId) ?? value.profiles[0]; setHeadersJson(JSON.stringify(profile?.customHeaders ?? {}, null, 2)); }); }, []);
  function updateProfile(patch: Partial<AiModelProfile>) { setSettings(value => ({ ...value, profiles: value.profiles.map(profile => profile.id === value.activeProfileId ? { ...profile, ...patch } : profile) })); }
  function selectProvider(providerId: string) {
    const existing = settings.profiles.find(profile => profile.provider === providerId);
    if (existing) {
      setSettings(value => ({ ...value, activeProfileId: existing.id }));
      setHeadersJson(JSON.stringify(existing.customHeaders ?? {}, null, 2));
    } else {
      const preset = aiProviders.find(provider => provider.id === providerId) ?? aiProviders.at(-1)!;
      const profile: AiModelProfile = { id: `${providerId}-${Date.now()}`, provider: preset.id, name: preset.name, baseUrl: preset.baseUrl, model: preset.model, apiMode: preset.apiMode, customHeaders: {}, hasApiKey: false };
      setSettings(value => ({ ...value, profiles: [...value.profiles, profile], activeProfileId: profile.id }));
      setHeadersJson("{}");
    }
    setApiKey(""); setAvailableModels([]); setError(""); setSaved(false);
  }
  function parsedHeaders() {
    const parsed = JSON.parse(headersJson) as unknown;
    if (!parsed || Array.isArray(parsed) || typeof parsed !== "object" || Object.values(parsed).some(value => typeof value !== "string")) throw new Error("自定义请求头必须是字符串键值的 JSON 对象。");
    return parsed as Record<string, string>;
  }
  async function fetchModels() {
    setError(""); setFetchingModels(true); setAvailableModels([]);
    try {
      if (!active) throw new Error("未找到当前模型配置。");
      const models = await api.listAiModels({ ...active, customHeaders: parsedHeaders() }, apiKey || undefined);
      setAvailableModels(models);
      if (!active.model && models[0]) updateProfile({ model: models[0] });
    } catch (reason) { setError(chineseError(reason, "获取模型失败。")); }
    finally { setFetchingModels(false); }
  }
  async function save() {
    setError("");
    try {
      if (!active) throw new Error("未找到当前模型配置。");
      const next = { ...settings, profiles: settings.profiles.map(profile => profile.id === active.id ? { ...profile, customHeaders: parsedHeaders() } : profile) };
      await api.saveAiSettings(next, active.id, apiKey || undefined); setSettings(next);
      setSaved(true); window.setTimeout(() => setSaved(false), 1800);
    } catch (reason) { setError(reason instanceof Error ? reason.message : "设置保存失败。"); }
  }
  return <div className="modal-backdrop" role="presentation" onMouseDown={event => { if (event.currentTarget === event.target) onClose(); }}><div className="modal provider-modal" role="dialog" aria-modal="true" aria-labelledby="settings-title"><div className="modal-head"><div><span>设置</span><h2 id="settings-title">大模型设置</h2></div><button className="icon-button" aria-label="关闭" onClick={onClose}><X size={18} /></button></div><div className="ai-local-note"><ShieldWarning size={17} /><p>本地扫描不依赖大模型。启用后，仅在你确认分类或联网搜索时请求所选服务。</p></div><label className="enable-ai"><input type="checkbox" checked={settings.enabled} onChange={event => setSettings(value => ({ ...value, enabled: event.target.checked }))} /><span>启用大模型增强审查</span></label>{active && <div className="provider-form"><label>服务提供方<select value={active.provider} onChange={event => selectProvider(event.target.value)}>{aiProviders.map(provider => <option key={provider.id} value={provider.id}>{provider.name}</option>)}</select></label><label>接口地址<input value={active.baseUrl} onChange={event => updateProfile({ baseUrl: event.target.value })} placeholder="https://api.example.com/v1" /></label><label>模型名称<div className="model-fetch-row"><input value={active.model} onChange={event => updateProfile({ model: event.target.value })} placeholder="输入模型 ID 或点击获取模型" /><button className="button quiet" disabled={fetchingModels} onClick={() => void fetchModels()}>{fetchingModels ? "正在获取" : "获取模型"}</button></div></label><label>API 密钥<input type="password" value={apiKey} onChange={event => setApiKey(event.target.value)} placeholder={active.hasApiKey ? "已安全保存，留空表示不修改" : active.provider === "ollama" ? "本机 Ollama 无需填写" : "输入 API Key"} /></label><p className="compatibility-copy">兼容 OpenAI 请求格式。API Key 按服务商分别保存在 Windows 凭据库。</p><div className="available-models"><div><b>可用模型</b><span>{availableModels.length > 0 ? `${availableModels.length} 个` : "点击“获取模型”加载"}</span></div>{availableModels.length > 0 && <div className="model-options">{availableModels.map(model => <button key={model} className={active.model === model ? "selected" : ""} onClick={() => updateProfile({ model })}>{model}<CheckCircle size={14} weight={active.model === model ? "fill" : "regular"} /></button>)}</div>}</div><details className="advanced-settings"><summary>高级接口设置</summary><label>接口模式<select value={active.apiMode} onChange={event => updateProfile({ apiMode: event.target.value as AiModelProfile["apiMode"] })}><option value="responses-web-search">Responses API + Web Search</option><option value="chat-completions">Chat Completions</option></select></label><label>自定义请求头 JSON<textarea value={headersJson} onChange={event => setHeadersJson(event.target.value)} spellCheck={false} rows={3} /></label></details></div>}{error && <p className="form-error" role="alert">{error}</p>}<div className="modal-actions provider-actions"><span>当前服务：{active?.name ?? "未配置"}</span><button className="button quiet" onClick={onClose}>取消</button><button className="button primary" onClick={() => void save()}>{saved ? "设置已保存" : "保存设置"}</button></div></div></div>;
}

function ClassificationModal({ detail, onClose, onSaved }: { detail: SkillDetail; onClose: () => void; onSaved: () => Promise<void> }) {
  const [category, setCategory] = useState<SkillCategory>(detail.category);
  const [tags, setTags] = useState(detail.tags.join(", "));
  const [sending, setSending] = useState(false);
  async function runAi() { setSending(true); try { await api.classify([detail.id]); await onSaved(); } finally { setSending(false); } }
  async function saveManual() { await api.setClassification(detail.id, category, tags.split(/[,，]/).map(tag => tag.trim()).filter(Boolean)); await onSaved(); }
  return <div className="modal-backdrop"><div className="modal wide" role="dialog" aria-modal="true" aria-labelledby="classification-title"><div className="modal-head"><div><span>分类</span><h2 id="classification-title">{detail.name}</h2></div><button className="icon-button" aria-label="关闭" onClick={onClose}><X size={18} /></button></div><div className="payload-preview"><div><Brain size={19} /><b>将发送给 AI 的内容</b></div><p>完整 SKILL.md，共 {detail.manifestBody?.length ?? 0} 个字符。配套资源不会发送。</p><pre>{detail.manifestBody?.slice(0, 520)}{(detail.manifestBody?.length ?? 0) > 520 ? "..." : ""}</pre></div><button className="button ai-button" disabled={sending} onClick={() => void runAi()}><Sparkle size={16} />{sending ? "正在分类" : "确认并使用 AI 分类"}</button><div className="or"><span>或在本地修正</span></div><label>主分类<select value={category} onChange={event => setCategory(event.target.value as SkillCategory)}>{SKILL_CATEGORIES.map(item => <option key={item}>{item}</option>)}</select></label><label>标签<input value={tags} onChange={event => setTags(event.target.value)} placeholder="使用逗号分隔" /></label><div className="modal-actions"><button className="button quiet" onClick={onClose}>取消</button><button className="button primary" onClick={() => void saveManual()}>保存本地分类</button></div></div></div>;
}

function SkeletonList() { return <div className="skeleton-list">{[1,2,3,4,5].map(item => <div className="skeleton-row" key={item}><i /><div><b /><span /><span /></div></div>)}</div>; }
function EmptyState({ onReset }: { onReset: () => void }) { return <div className="empty"><HardDrives size={38} /><h2>没有匹配的 Skill</h2><p>调整搜索或过滤条件，或者添加一个自定义 Skill 根目录。</p><button className="button quiet" onClick={onReset}>清除过滤器</button></div>; }
function formatBytes(value: number) { return value < 1024 ? `${value} B` : `${(value / 1024).toFixed(1)} KB`; }
