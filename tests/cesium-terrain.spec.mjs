// W3.6 acceptance test — CesiumTerrainProvider loads our quantized-mesh tiles.
//
// This is the load-bearing test for W3.6 spec compliance: it drives a real
// headless Chromium to fetch `layer.json` + the {z}/{x}/{y}.terrain tiles we
// generated (via terrain_tms_generate.mjs), build a CesiumTerrainProvider,
// attach it to the globe, and sample terrain heights.
//
// `sampleTerrainMostDetailed` only resolves if Cesium successfully decoded our
// quantized-mesh bytes into a heightfield. If our encoder is non-compliant
// (bad header layout, wrong zig-zag/HWM scheme, malformed index stream), this
// test fails — that's the whole point. The existing
// `quantized_mesh_roundtrip_test.rs` only proves the bytes are self-consistent
// against our OWN decoder; this spec proves a third-party consumer (Cesium)
// accepts them.

import { test, expect } from '@playwright/test';

const TEST_PAGE = '/tests/fixtures/cesium-terrain-loader.html';

test('CesiumTerrainProvider loads W3.6 quantized-mesh tiles', async ({ page }) => {
  const networkErrors = [];
  const consoleErrors = [];
  const terrainHttpErrors = [];

  page.on('requestfailed', (req) => {
    const url = req.url();
    // Only flag terrain-related failures — random analytics/CDN hiccups
    // unrelated to our test shouldn't break it.
    if (url.includes('.terrain') || url.includes('layer.json')) {
      networkErrors.push(`${url} — ${req.failure()?.errorText || 'request failed'}`);
    }
  });
  page.on('response', (resp) => {
    const url = resp.url();
    if ((url.includes('.terrain') || url.includes('layer.json')) && resp.status() >= 400) {
      terrainHttpErrors.push(`${resp.status()} ${url}`);
    }
  });
  page.on('pageerror', (err) => {
    consoleErrors.push(err.message);
  });

  await page.goto(TEST_PAGE, { waitUntil: 'domcontentloaded' });

  // Poll for either ready or error signal. The page writes one of these two
  // flags on completion of its async terrain-loading pipeline.
  await page.waitForFunction(
    () => window.__terrainReady === true || typeof window.__terrainError === 'string',
    {},
    { timeout: 60_000 },
  );

  // Surface any error the page caught, with diagnostic context. The page
  // doesn't expose internal provider state on success, so we only need
  // detail on failure.
  const pageError = await page.evaluate(() => window.__terrainError);
  if (pageError) {
    throw new Error(
      `page reported error during terrain load: ${pageError}\n` +
        `terrain HTTP errors: ${terrainHttpErrors.join('; ') || '(none)'}\n` +
        `console errors: ${consoleErrors.join('; ') || '(none)'}`,
    );
  }

  // Strong assertion: terrain was sampled successfully. This means Cesium
  // fetched a .terrain tile, decoded the quantized-mesh bytes, and returned
  // a finite height — the most direct proof of spec compliance.
  const ready = await page.evaluate(() => window.__terrainReady);
  expect(ready, 'window.__terrainReady must be true').toBe(true);

  const sampledHeight = await page.evaluate(() => window.__sampledHeight);
  expect(sampledHeight, 'sampleTerrainMostDetailed returned a height').not.toBeNull();
  expect(Number.isFinite(sampledHeight), 'sampled height is finite').toBe(true);

  const providerAccepted = await page.evaluate(() => window.__providerAccepted);
  expect(providerAccepted, 'CesiumTerrainProvider.fromUrl resolved').toBe(true);

  const tileFetches = await page.evaluate(() => window.__tileFetchCount);
  expect(tileFetches, 'Cesium fetched ≥1 .terrain tile').toBeGreaterThan(0);

  // No terrain-related fetch or page errors.
  expect(networkErrors, 'no .terrain / layer.json fetch failures').toEqual([]);
  expect(consoleErrors, 'no uncaught page errors').toEqual([]);
  expect(terrainHttpErrors, 'no 4xx/5xx on terrain URLs').toEqual([]);
});
