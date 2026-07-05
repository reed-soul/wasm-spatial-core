// Playwright config — W3.6 headless Cesium terrain acceptance test.
//
// This is the project's first browser test (introduced here). Scope is
// intentionally minimal: one spec (tests/cesium-terrain.spec.mjs) proving
// CesiumTerrainProvider can load our quantized-mesh tiles. The webServer hook
// runs tests/terrain_tms_server.mjs which generates the TMS pyramid from the
// current WASM build and serves the repo root + /terrain-tms/* over HTTP.
//
// Local usage:
//   npm run build:pkg
//   wasm-pack build --target nodejs --release --out-dir pkg-node --features point-cloud,geotiff
//   npm install
//   npx playwright install --with-deps chromium
//   npm run test:browser

import { defineConfig } from '@playwright/test';

const PORT = process.env.TERRAIN_TEST_PORT || 8090;

export default defineConfig({
  testDir: './tests',
  testMatch: /(cesium-terrain|webgpu-bench)\.spec\.mjs$/,
  // Headless Cesium + WebGL init + tile fetch is slow; give one test plenty
  // of headroom. The per-step waits inside the spec are tighter. The webgpu
  // bench needs even more (10M-point transform + GPU shader compile).
  timeout: 180_000,
  expect: { timeout: 30_000 },
  fullyParallel: false,
  workers: 1,
  // Headed mode is useful locally; CI is always headless.
  use: {
    headless: true,
    viewport: { width: 1280, height: 720 },
    // Cesium's CDN script needs to load; don't fail the run on transient
    // network blips — the spec retries via waitForFunction instead.
    ignoreHTTPSErrors: true,
    // WebGPU in headless Chromium needs these flags. On macOS the Metal
    // adapter is picked up automatically; on Linux CI you also need a real
    // GPU + Vulkan. The webgpu-bench test self-skips when no adapter.
    launchOptions: {
      args: [
        '--enable-unsafe-webgpu',
        '--enable-features=Vulkan,UseSkiaRenderer',
        '--disable-gpu-sandbox',
        '--use-angle=metal',
      ],
    },
  },
  webServer: {
    command: `node tests/terrain_tms_server.mjs`,
    port: PORT,
    cwd: '.',
    timeout: 60_000,
    // In CI always start fresh; locally reuse a running dev server if present.
    reuseExistingServer: !process.env.CI,
    env: {
      ...process.env,
      PORT: String(PORT),
    },
  },
});
