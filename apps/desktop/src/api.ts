import type {
  AiSettings, DashboardSummary, HealthFinding, PagedSkills, ProviderRoot,
  SkillCategory, SkillDetail, SkillQuery,
} from "@skill-atlas/contracts";

const isTauri = () => Boolean(window.__TAURI_INTERNALS__);

async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) throw new Error("Tauri runtime is not available");
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(name, args);
}

const demoAssets: SkillDetail[] = [
  {
    id: "asset-design-taste", logicalKey: "design-taste-frontend", name: "design-taste-frontend",
    description: "为落地页和作品集提供前端设计规范、视觉判断与交付检查。",
    contentFingerprint: "b4c2a8f8c31d", category: "设计与媒体", tags: ["React", "前端", "设计系统"],
    classification: { category: "设计与媒体", tags: ["React", "前端", "设计系统"], source: "ai", rationale: "内容主要关注视觉系统与前端交付。", model: "gpt-4.1-mini", contentFingerprint: "b4c2a8f8c31d", isStale: false },
    installationCount: 2, providers: ["codex", "shared"], scopes: ["user"], health: "healthy", findingCount: 0,
    hasConflict: false, hasScripts: false, updatedAt: "2026-10-04T09:42:00Z", bodyPreview: "读取设计 brief，推断合适的视觉方向并交付界面。",
    manifestBody: "---\nname: design-taste-frontend\ndescription: 为落地页和作品集提供前端设计规范。\n---\n\n读取设计 brief，推断合适的视觉方向并交付界面。",
    files: [{ path: "SKILL.md", size: 48320, kind: "file" }],
    installations: [
      { id: "i1", assetId: "asset-design-taste", provider: "codex", providerLabel: "Codex", scope: "user", displayPath: "C:\\Users\\you\\.codex\\skills\\design-taste-frontend", normalizedPath: "c:/users/you/.codex/skills/design-taste-frontend", linkType: "directory", readOnly: true },
      { id: "i2", assetId: "asset-design-taste", provider: "shared", providerLabel: "Shared", scope: "user", displayPath: "C:\\Users\\you\\.agents\\skills\\design-taste-frontend", normalizedPath: "c:/users/you/.agents/skills/design-taste-frontend", linkType: "symlink", resolvedTarget: "C:\\Users\\you\\.codex\\skills\\design-taste-frontend", readOnly: true },
    ], findings: [],
  },
  {
    id: "asset-pdf", logicalKey: "pdf", name: "pdf", description: "读取、创建、检查与渲染 PDF 文件。",
    contentFingerprint: "4a900f72a103", category: "文档与办公", tags: ["PDF", "文档"],
    classification: { category: "文档与办公", tags: ["PDF", "文档"], source: "rule", rationale: "名称与描述命中文档处理规则。", isStale: false },
    installationCount: 1, providers: ["codex"], scopes: ["plugin"], health: "attention", findingCount: 1,
    hasConflict: false, hasScripts: true, updatedAt: "2026-10-03T18:14:00Z", bodyPreview: "使用渲染工具检查 PDF 的布局与表单。",
    files: [{ path: "SKILL.md", size: 7360, kind: "file" }, { path: "scripts/render.py", size: 9440, kind: "file" }],
    installations: [{ id: "i3", assetId: "asset-pdf", provider: "codex", providerLabel: "Codex Plugin", scope: "plugin", displayPath: "C:\\Users\\you\\.codex\\plugins\\cache\\runtime\\pdf\\skills\\pdf", normalizedPath: "c:/users/you/.codex/plugins/cache/runtime/pdf/skills/pdf", linkType: "directory", readOnly: true }],
    findings: [{ id: "f1", assetId: "asset-pdf", code: "executable-content", severity: "info", title: "包含可执行脚本", detail: "发现 scripts 目录。Skill Atlas 不会执行其中内容。" }],
  },
  {
    id: "asset-review", logicalKey: "code-review", name: "code-review", description: "审查代码变更并输出可执行反馈。",
    contentFingerprint: "7d8cc11f2b98", category: "测试与质量", tags: ["Review"],
    classification: { category: "测试与质量", tags: ["Review"], source: "user", rationale: "用户修正", isStale: false },
    installationCount: 2, providers: ["claude-code", "cursor"], scopes: ["user", "project"], health: "error", findingCount: 2,
    hasConflict: true, hasScripts: false, bodyPreview: "检查 pull request 的正确性、回归风险和测试覆盖。",
    files: [{ path: "SKILL.md", size: 2840, kind: "file" }],
    installations: [{ id: "i4", assetId: "asset-review", provider: "claude-code", providerLabel: "Claude Code", scope: "user", displayPath: "C:\\Users\\you\\.claude\\skills\\code-review", normalizedPath: "c:/users/you/.claude/skills/code-review", linkType: "directory", readOnly: true }],
    findings: [
      { id: "f2", assetId: "asset-review", code: "name-conflict", severity: "error", title: "同名内容冲突", detail: "另一个安装位置存在名称相同但内容不同的 Skill。" },
      { id: "f3", assetId: "asset-review", code: "missing-reference", severity: "warning", title: "引用文件缺失", detail: "SKILL.md 引用了 references/checklist.md，但文件不存在。" },
    ],
  },
];

function filterDemo(query: SkillQuery): PagedSkills {
  const search = query.search?.trim().toLocaleLowerCase() ?? "";
  const items = demoAssets.filter((item) => {
    if (search && !`${item.name} ${item.description} ${item.tags.join(" ")} ${item.installations.map(i => i.displayPath).join(" ")}`.toLocaleLowerCase().includes(search)) return false;
    if (query.provider && !item.providers.includes(query.provider)) return false;
    if (query.category && item.category !== query.category) return false;
    if (query.health && item.health !== query.health) return false;
    if (query.conflictsOnly && !item.hasConflict) return false;
    return true;
  });
  return { items, total: items.length, page: 1, pageSize: 100 };
}

export const api = {
  isTauri,
  async listSkills(query: SkillQuery): Promise<PagedSkills> { return isTauri() ? command("list_skills", { query }) : filterDemo(query); },
  async getSkillDetail(assetId: string): Promise<SkillDetail> { return isTauri() ? command("get_skill_detail", { assetId }) : demoAssets.find(item => item.id === assetId) ?? demoAssets[0]; },
  async dashboardSummary(): Promise<DashboardSummary> {
    if (isTauri()) return command("dashboard_summary");
    return { assets: demoAssets.length, installations: 5, providers: 4, findings: 3, conflicts: 1, categories: [
      { category: "设计与媒体", count: 1 }, { category: "文档与办公", count: 1 }, { category: "测试与质量", count: 1 },
    ] };
  },
  async listHealthFindings(): Promise<HealthFinding[]> { return isTauri() ? command("list_health_findings", { query: {} }) : demoAssets.flatMap(item => item.findings); },
  async listProviderRoots(): Promise<ProviderRoot[]> { return isTauri() ? command("list_provider_roots") : []; },
  async scanRoots(): Promise<string> { return isTauri() ? command("scan_roots", { request: { includeDefaultRoots: true } }) : "demo"; },
  async cancelScan(scanId: string): Promise<boolean> { return isTauri() ? command("cancel_scan", { scanId }) : true; },
  async setClassification(assetId: string, category: SkillCategory, tags: string[]): Promise<void> { if (isTauri()) await command("set_user_classification", { assetId, category, tags }); },
  async classify(assetIds: string[]): Promise<void> { if (isTauri()) await command("classify_skills", { assetIds, consent: { confirmed: true, includeManifestBody: true } }); },
  async getAiSettings(): Promise<AiSettings> { return isTauri() ? command("get_ai_settings") : { baseUrl: "https://api.openai.com/v1", model: "gpt-4.1-mini", customHeaders: {}, hasApiKey: false }; },
  async saveAiSettings(settings: AiSettings, apiKey?: string): Promise<void> { if (isTauri()) await command("save_ai_settings", { settings, apiKey }); },
  async addCustomRoot(path: string): Promise<void> { if (isTauri()) await command("add_custom_root", { path, provider: "custom", scope: "custom" }); },
  async reveal(path: string): Promise<void> { if (isTauri()) await command("reveal_in_file_manager", { path }); },
  async copyPath(path: string): Promise<void> { if (isTauri()) await command("copy_path", { path }); else await navigator.clipboard.writeText(path); },
};
