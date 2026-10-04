export const SKILL_CATEGORIES = [
  "编程开发", "测试与质量", "DevOps 与云", "数据与 AI", "设计与媒体",
  "文档与办公", "研究与知识", "业务与营销", "安全与合规", "Agent 与工具", "其他",
] as const;

export type SkillCategory = (typeof SKILL_CATEGORIES)[number];
export type ProviderId = "codex" | "claude-code" | "cursor" | "gemini-cli" | "shared" | "custom";
export type SkillScope = "user" | "project" | "system" | "plugin" | "custom";
export type FindingSeverity = "info" | "warning" | "error";
export type ClassificationSource = "author" | "rule" | "ai" | "user" | "none";

export interface HealthFinding {
  id: string; assetId: string; code: string; severity: FindingSeverity;
  title: string; detail: string; path?: string;
}

export interface SkillInstallation {
  id: string; assetId: string; provider: ProviderId; providerLabel: string;
  scope: SkillScope; displayPath: string; normalizedPath: string; resolvedTarget?: string;
  linkType: "directory" | "symlink" | "junction" | "unknown";
  readOnly: boolean; modifiedAt?: string;
}

export interface ClassificationRecord {
  category: SkillCategory; tags: string[]; source: ClassificationSource;
  rationale?: string; model?: string; contentFingerprint?: string; isStale: boolean;
}

export interface SkillAsset {
  id: string; logicalKey: string; name: string; description: string; contentFingerprint: string;
  compatibility?: string; license?: string; author?: string; bodyPreview: string; manifestBody?: string;
  category: SkillCategory; tags: string[]; classification: ClassificationRecord;
  installationCount: number; providers: ProviderId[]; scopes: SkillScope[];
  health: "healthy" | "attention" | "error"; findingCount: number;
  hasConflict: boolean; hasScripts: boolean; updatedAt?: string;
}

export interface SkillDetail extends SkillAsset {
  installations: SkillInstallation[];
  findings: HealthFinding[];
  files: Array<{ path: string; size: number; kind: "file" | "directory" }>;
}

export interface SkillQuery {
  search?: string; provider?: ProviderId; scope?: SkillScope; category?: SkillCategory;
  health?: SkillAsset["health"]; conflictsOnly?: boolean; page?: number; pageSize?: number;
}

export interface PagedSkills { items: SkillAsset[]; total: number; page: number; pageSize: number; }

export interface ProviderRoot {
  id: string; provider: ProviderId; providerLabel: string; scope: SkillScope;
  displayPath: string; normalizedPath: string; exists: boolean; custom: boolean; readOnly: boolean;
}

export interface ScanSummary {
  scanId: string; assets: number; installations: number; findings: number;
  conflicts: number; rootsScanned: number; durationMs: number;
}

export interface DashboardSummary {
  assets: number; installations: number; providers: number; findings: number; conflicts: number;
  categories: Array<{ category: SkillCategory; count: number }>;
}

export type AiApiMode = "chat-completions" | "responses-web-search";
export interface AiModelProfile {
  id: string; name: string; baseUrl: string; model: string; apiMode: AiApiMode;
  customHeaders: Record<string, string>; hasApiKey: boolean;
}
export interface AiSettings { profiles: AiModelProfile[]; activeProfileId: string; }

export interface OnlineSkillResult {
  name: string; description: string; sourceUrl: string; repositoryUrl?: string;
  author?: string; whyRelevant: string; tags: string[];
}

export interface ClassificationConsent { confirmed: boolean; includeManifestBody: true; }
