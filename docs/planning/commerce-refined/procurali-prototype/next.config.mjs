/** Static export: every route is pre-rendered to HTML at build time (no server needed). */
const buildCpus = Number.parseInt(process.env.PROCURALI_BUILD_CPUS || "", 10);

const nextConfig = {
  output: "export",
  trailingSlash: true,
  images: { unoptimized: true },
  poweredByHeader: false,
  reactStrictMode: true,
  ...(process.env.PROCURALI_DIST_DIR ? { distDir: process.env.PROCURALI_DIST_DIR } : {}),
  // The VPS site sync caps static-generation workers to bound memory inside its systemd slice.
  ...(buildCpus > 0 ? { experimental: { cpus: buildCpus } } : {}),
};

export default nextConfig;
