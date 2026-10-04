import { siteConfig } from "./config";

const capabilities = [
  ["一个 Skill，多处安装", "把逻辑资产与 Codex、Claude Code、Cursor 等目录中的安装实例分开建模。"],
  ["用途与分类有依据", "区分作者描述、规则推断和用户修正，让每个结论都能追溯。"],
  ["只读扫描起步", "首次运行不会移动、删除或执行任何 Skill 文件，先看清现状再决定。"],
  ["冲突直接可见", "识别同名不同内容、相同内容副本、断裂链接、未知来源和规范问题。"],
];

export default function App() {
  return (
    <div className="site-shell">
      <header className="site-nav">
        <a className="wordmark" href="#top" aria-label="Skill Atlas 首页">
          <img src="./app-icon-source.png" alt="" />
          <span>Skill Atlas</span>
        </a>
        <nav aria-label="官网导航">
          <a href="#capabilities">能力</a>
          <a href="#architecture">架构</a>
          <a href={siteConfig.repositoryUrl}>GitHub</a>
        </nav>
      </header>

      <main id="top">
        <section className="hero">
          <div className="hero-copy">
            <span className="kicker">LOCAL-FIRST SKILL INVENTORY</span>
            <h1>本机 Skill 一处看清</h1>
            <p>用途、路径、来源、分类和健康状态，集中在一个本地桌面工具中。</p>
            <div className="hero-actions">
              <a className="button primary" href="#capabilities">查看核心能力</a>
              <a className="button secondary" href={siteConfig.repositoryUrl}>查看源码</a>
            </div>
          </div>
          <figure className="hero-visual">
            <img src="./hero-library.png" alt="多组本地技能目录汇入统一资产库的视觉隐喻" />
          </figure>
        </section>

        <section className="platform-line" aria-label="计划支持的 Agent">
          <span>Codex</span>
          <span>Claude Code</span>
          <span>Cursor</span>
          <span>Gemini CLI</span>
          <span>.agents/skills</span>
        </section>

        <section className="capabilities" id="capabilities">
          <div className="section-copy">
            <h2>目录不是资产模型</h2>
            <p>同一个 Skill 可能被复制、链接或安装到多个项目。Skill Atlas 保留这些关系，而不是把每个文件夹都算作新 Skill。</p>
          </div>
          <div className="capability-grid">
            {capabilities.map(([title, detail], index) => (
              <article className={index === 0 || index === 3 ? "feature feature-wide" : "feature"} key={title}>
                <h3>{title}</h3>
                <p>{detail}</p>
              </article>
            ))}
          </div>
        </section>

        <section className="product-view">
          <div className="product-copy">
            <h2>一眼找到重复、漂移和风险</h2>
            <p>搜索名称、用途、标签和路径。详情检查器同时展示作者声明、分类依据、真实路径和健康发现。</p>
          </div>
          <div className="screenshot-frame">
            <img src="./product-screenshot.png" alt="Skill Atlas 桌面端资产库界面" />
          </div>
        </section>

        <section className="architecture" id="architecture">
          <div>
            <h2>为 Windows 构建，也为 macOS 留好边界</h2>
            <p>跨平台扫描逻辑位于独立 Rust Core。Tauri 提供轻量桌面壳，React 负责界面，操作系统凭据保留在系统钥匙串。</p>
          </div>
          <dl>
            <div><dt>核心</dt><dd>Rust</dd></div>
            <div><dt>桌面</dt><dd>Tauri 2</dd></div>
            <div><dt>界面</dt><dd>React</dd></div>
            <div><dt>索引</dt><dd>SQLite FTS5</dd></div>
          </dl>
        </section>

      </main>

      <footer>
        <span>Skill Atlas</span>
        <span>本地优先，源代码开放</span>
        <a href={siteConfig.repositoryUrl}>GitHub 仓库</a>
      </footer>
    </div>
  );
}
