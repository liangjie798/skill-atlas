const repository = import.meta.env.VITE_GITHUB_REPOSITORY || "YOUR_ORG/skill-atlas";
export const siteConfig = {
  repository,
  repositoryUrl: `https://github.com/${repository}`,
};
