import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowsClockwise, Brain, CheckCircle, Clipboard, Code, FolderOpen, Funnel,
  Gear, GitDiff, GridFour, HardDrives, Heartbeat, MagnifyingGlass, Path,
  Plus, Robot, ShieldWarning, SidebarSimple, Sparkle, Stack, Tag, Warning,
  X,
} from "@phosphor-icons/react";
import type { AiSettings, DashboardSummary, SkillAsset, SkillCategory, SkillDetail, SkillQuery } from "@skill-atlas/contracts";
import { SKILL_CATEGORIES } from "@skill-atlas/contracts";
import { api } from "./api";

type NavView = "library" | "agents" | "conflicts" | "health" | "categories";
type DetailTab = "overview" | "manifest" | "files";

const categoryShort: Record<SkillCategory, string> = {
  "编程开发": "开发", "测试与质量": "测试", "DevOps 与云": "运维", "数据与 AI": "数据",
  "设计与媒体": "设计", "文档与办公": "文档", "研究与知识": "研究", "业务与营销": "业务",
  "安全与合规": "安全", "Agent 与工具": "Agent", "其他": "其他",
};

const providerLabels: Record<string, string> = {
  codex: "Codex", "claude-code": "Claude Code", cursor: "Cursor", "gemini-cli": "Gemini CLI", shared: "Shared", custom: "Custom",
};

function healthLabel(health: SkillAsset["health"]) {
  return health === "healthy" ? "健康" : health === "attention" ? "需关注" : "有错误";
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
    } catch (error) { setNotice(error instanceof Error ? error.message : "加载失败"); }
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
          setNotice(`扫描完成：${event.payload.assets ?? 0} 个资产，${event.payload.installations ?? 0} 个安装实例，用时 ${event.payload.durationMs ?? 0} ms`);
        }
      });
      unlistenFailed = await listen<{ scanId: string; error: string }>("scan://failed", event => {
        if (event.payload.scanId === activeScan.current) { setScanning(false); activeScan.current = undefined; setNotice(event.payload.error); }
      });
      unlistenProgress = await listen<{ scanId: string; progress: number }>("scan://progress", event => {
        if (event.payload.scanId === activeScan.current) setNotice(`正在扫描批准的根目录：${event.payload.progress}%`);
      });
    });
    return () => { window.clearTimeout(retry); unlisten?.(); unlistenFailed?.(); unlistenProgress?.(); };
  }, [load]);
  useEffect(() => {
    if (!selectedId) { setDetail(undefined); return; }
    void api.getSkillDetail(selectedId).then(setDetail).catch(error => setNotice(String(error)));
  }, [selectedId]);

  async function scan() {
    setScanning(true); setNotice(undefined);
    try {
      activeScan.current = await api.scanRoots();
      setNotice("扫描任务已启动");
    } catch (error) { setNotice(error instanceof Error ? error.message : "扫描失败"); }
    if (!api.isTauri()) { setScanning(false); await load(); }
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
    <div className="app-shell">
      <header className="topbar" data-tauri-drag-region>
        <div className="brand"><img src="/app-icon-source.png" alt="" /><strong>Skill Atlas</strong><span className="read-only">只读模式</span></div>
        <div className="top-actions">
          <button className="button quiet" onClick={() => void addRoot()}><Plus size={16} />添加目录</button>
          <button className={scanning ? "button quiet" : "button primary"} onClick={() => void (scanning ? cancelScan() : scan())}><ArrowsClockwise size={16} className={scanning ? "spin" : ""} />{scanning ? "取消扫描" : "重新扫描"}</button>
          <button className="icon-button" aria-label="设置" onClick={() => setSettingsOpen(true)}><Gear size={19} /></button>
        </div>
      </header>

      <div className="workspace">
        <aside className="sidebar">
          <nav aria-label="资产视图">
            <NavButton active={view === "library"} icon={<GridFour />} label="资产库" count={summary?.assets} onClick={() => { setView("library"); setQuery({ pageSize: 100 }); }} />
            <NavButton active={view === "agents"} icon={<Robot />} label="按 Agent" count={summary?.providers} onClick={() => setView("agents")} />
            <NavButton active={view === "categories"} icon={<Tag />} label="分类" onClick={() => setView("categories")} />
            <NavButton active={view === "conflicts"} icon={<GitDiff />} label="冲突" count={summary?.conflicts} tone="danger" onClick={() => { setView("conflicts"); setQuery(q => ({ ...q, conflictsOnly: true })); }} />
            <NavButton active={view === "health"} icon={<Heartbeat />} label="健康检查" count={summary?.findings} tone="warning" onClick={() => { setView("health"); setQuery(q => ({ ...q, health: "attention" })); }} />
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

        <section className="library-panel">
          <div className="panel-toolbar">
            <div className="search"><MagnifyingGlass size={17} /><input value={query.search ?? ""} onChange={event => setQuery(q => ({ ...q, search: event.target.value }))} placeholder="搜索名称、用途、标签或路径" aria-label="搜索 Skill" /></div>
            <button className="filter-button" aria-label="过滤器"><Funnel size={17} /><span>{assets.length}</span></button>
          </div>

          {notice && <div className="notice"><CheckCircle size={16} /><span>{notice}</span><button aria-label="关闭通知" onClick={() => setNotice(undefined)}><X size={14} /></button></div>}
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
        </section>

        <aside className="inspector">
          {detail ? <>
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

function SettingsModal({ onClose }: { onClose: () => void }) {
  const [settings, setSettings] = useState<AiSettings>({ baseUrl: "", model: "", customHeaders: {}, hasApiKey: false });
  const [apiKey, setApiKey] = useState("");
  const [headersJson, setHeadersJson] = useState("{}");
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);
  useEffect(() => { void api.getAiSettings().then(value => { setSettings(value); setHeadersJson(JSON.stringify(value.customHeaders, null, 2)); }); }, []);
  async function save() {
    setError("");
    try {
      const parsed = JSON.parse(headersJson) as unknown;
      if (!parsed || Array.isArray(parsed) || typeof parsed !== "object" || Object.values(parsed).some(value => typeof value !== "string")) throw new Error("自定义请求头必须是字符串键值的 JSON 对象。");
      await api.saveAiSettings({ ...settings, customHeaders: parsed as Record<string, string> }, apiKey || undefined);
      setSaved(true); window.setTimeout(() => setSaved(false), 1800);
    } catch (reason) { setError(reason instanceof Error ? reason.message : "设置保存失败。"); }
  }
  return <div className="modal-backdrop" role="presentation" onMouseDown={event => { if (event.currentTarget === event.target) onClose(); }}><div className="modal" role="dialog" aria-modal="true" aria-labelledby="settings-title"><div className="modal-head"><div><span>设置</span><h2 id="settings-title">AI 分类服务</h2></div><button className="icon-button" aria-label="关闭" onClick={onClose}><X size={18} /></button></div><p className="modal-copy">使用 OpenAI-compatible 接口。API Key 只保存在系统凭据库，不写入索引或日志。</p><label>Base URL<input value={settings.baseUrl} onChange={event => setSettings({ ...settings, baseUrl: event.target.value })} placeholder="https://api.openai.com/v1" /></label><label>模型<input value={settings.model} onChange={event => setSettings({ ...settings, model: event.target.value })} placeholder="gpt-4.1-mini" /></label><label>API Key<input type="password" value={apiKey} onChange={event => setApiKey(event.target.value)} placeholder={settings.hasApiKey ? "已安全保存，留空表示不修改" : "sk-..."} /></label><label>自定义请求头 JSON<textarea value={headersJson} onChange={event => setHeadersJson(event.target.value)} spellCheck={false} rows={4} placeholder={'{"X-Organization": "team"}'} /></label>{error && <p className="form-error" role="alert">{error}</p>}<div className="privacy-box"><ShieldWarning size={18} /><p>分类时会发送你确认过的完整 SKILL.md。不会发送 scripts、references 或 assets。</p></div><div className="modal-actions"><button className="button quiet" onClick={onClose}>取消</button><button className="button primary" onClick={() => void save()}>{saved ? "已保存" : "保存设置"}</button></div></div></div>;
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
