/** Static export: every route is pre-rendered to HTML at build time (no server needed). */
const nextConfig = {
  output: "export",
  trailingSlash: true,
  images: { unoptimized: true },
  poweredByHeader: false,
  reactStrictMode: true,
  ...(process.env.PROCURALI_DIST_DIR ? { distDir: process.env.PROCURALI_DIST_DIR } : {}),
};

export default nextConfig;
