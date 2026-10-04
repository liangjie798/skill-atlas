const repository = import.meta.env.VITE_GITHUB_REPOSITORY || "liangjie798/skill-atlas";
export const siteConfig = {
  repository,
  repositoryUrl: `https://github.com/${repository}`,
  downloadUrl: `https://github.com/${repository}/releases/latest/download/Skill-Atlas-x64-setup.exe`,
  checksumsUrl: `https://github.com/${repository}/releases/latest/download/SHA256SUMS.txt`,
  releasesUrl: `https://github.com/${repository}/releases`,
};
