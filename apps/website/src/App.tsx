import { useEffect, useState } from "react";
import { ArrowDown, CheckCircle, GithubLogo, ShieldCheck, WindowsLogo } from "@phosphor-icons/react";
import { siteConfig } from "./config";

const capabilities = [
  ["一个 Skill，多处安装", "相同内容自动归并，Codex、Claude Code、Cursor 等安装位置仍完整保留。"],
  ["分类有依据", "区分规则推断、AI 分类和用户修正，每个结论都可追溯。"],
  ["只读扫描", "不会移动、删除、改写或执行任何 Skill 文件，索引可随时重建。"],
  ["冲突直接可见", "识别同名异构、重复副本、断裂链接、未知来源和规范问题。"],
];

interface ReleaseMeta { version: string; publishedAt: string; size: string; sha256: string; requirements: string; changes: string[] }
const releaseFallback: ReleaseMeta = { version: "v0.1.0", publishedAt: "待发布", size: "约 5 MiB", sha256: "发布后提供", requirements: "Windows 10 1809 或更高版本，x64，当前用户安装", changes: [] };

export default function App() {
  const [release, setRelease] = useState(releaseFallback);
  useEffect(() => { void fetch("./release.json").then(response => response.ok ? response.json() as Promise<ReleaseMeta> : Promise.reject()).then(setRelease).catch(() => undefined); }, []);
  return (
    <div className="site-shell">
      <header className="site-nav">
        <a className="wordmark" href="#top" aria-label="Skill Atlas 首页">
          <img src="./app-icon-source.png" alt="" /><span>Skill Atlas</span>
        </a>
        <nav aria-label="官网导航">
          <a href="#capabilities">能力</a><a href="#architecture">架构</a>
          <a href="#download">下载</a><a href={siteConfig.repositoryUrl}>GitHub</a>
        </nav>
      </header>

      <main id="top">
        <section className="hero">
          <div className="hero-copy">
            <span className="kicker">LOCAL-FIRST SKILL INVENTORY</span>
            <h1>本机 Skill 一处看清</h1>
            <p>用途、路径、来源、分类和健康状态，集中在一个只读桌面工具中。</p>
            <div className="hero-actions">
              <a className="button primary" href="#download"><WindowsLogo size={18} weight="fill" />下载 Windows 预览版</a>
              <a className="button secondary" href={siteConfig.repositoryUrl}><GithubLogo size={18} />查看源码</a>
            </div>
          </div>
          <figure className="hero-visual"><img src="./hero-library.png" alt="多组本地技能目录汇入统一资产库" /></figure>
        </section>

        <section className="platform-line" aria-label="支持的 Agent">
          <span>Codex</span><span>Claude Code</span><span>Cursor</span><span>Gemini CLI</span><span>.agents/skills</span>
        </section>

        <section className="capabilities" id="capabilities">
          <div className="section-copy"><h2>目录不是资产模型</h2><p>一个 Skill 可能被复制、链接或安装到多个项目。Skill Atlas 保留这些关系，不把每个文件夹都算作新 Skill。</p></div>
          <div className="capability-grid">
            {capabilities.map(([title, detail], index) => <article className={index === 0 || index === 3 ? "feature feature-wide" : "feature"} key={title}><h3>{title}</h3><p>{detail}</p></article>)}
          </div>
        </section>

        <section className="product-view">
          <div className="product-copy"><h2>重复、漂移和风险，一眼找到</h2><p>搜索名称、用途、标签和路径。详情检查器同时展示分类依据、真实路径与健康发现。</p></div>
          <div className="screenshot-frame"><img src="./product-screenshot.png" alt="Skill Atlas 桌面端资产库界面" /></div>
        </section>

        <section className="architecture" id="architecture">
          <div><h2>为 Windows 构建，为 macOS 留好边界</h2><p>跨平台扫描逻辑位于独立 Rust Core。Tauri 提供轻量桌面壳，React 负责界面，API Key 保存在系统凭据库。</p></div>
          <dl><div><dt>核心</dt><dd>Rust</dd></div><div><dt>桌面</dt><dd>Tauri 2</dd></div><div><dt>界面</dt><dd>React 19</dd></div><div><dt>索引</dt><dd>SQLite FTS5</dd></div></dl>
        </section>

        <section className="download" id="download">
          <div className="download-copy"><ShieldCheck size={34} weight="duotone" /><h2>下载 Windows 预览版</h2><p>安装包托管在 GitHub Releases。当前预览版尚未签名，Windows SmartScreen 可能要求你确认来源。</p></div>
          <div className="download-actions">
            <a className="button primary" href={siteConfig.downloadUrl}><ArrowDown size={18} weight="bold" />下载 EXE</a>
            <a className="text-link" href={siteConfig.checksumsUrl}>查看 SHA-256 校验文件</a>
            <a className="text-link" href={siteConfig.releasesUrl}>全部版本与变更记录</a>
          </div>
          <dl className="release-meta"><div><dt>版本</dt><dd>{release.version}</dd></div><div><dt>发布日期</dt><dd>{release.publishedAt}</dd></div><div><dt>包体</dt><dd>{release.size}</dd></div><div><dt>系统要求</dt><dd>{release.requirements}</dd></div><div className="checksum"><dt>SHA-256</dt><dd>{release.sha256}</dd></div>{release.changes.length > 0 && <div className="changes"><dt>本版更新</dt><dd>{release.changes.join(" · ")}</dd></div>}</dl>
          <div className="privacy-note"><CheckCircle size={20} weight="fill" /><p>Skill 扫描完全在本机完成。AI 分类只会在你确认后发送所选的 SKILL.md，不会发送 scripts、references 或 assets。</p></div>
        </section>
      </main>

      <footer><span>Skill Atlas</span><span>本地优先，源代码开放</span><a href={siteConfig.repositoryUrl}>GitHub 仓库</a></footer>
    </div>
  );
}
