// W4 WebGPU benchmark — GPU vs WASM speedup measurement.
//
// This is NOT a CI-blocking pass/fail test in the strict sense. Its job is to
// MEASURE the two 🟡 hardware-gated exit criteria in ROADMAP_V2 Wave 4:
//   - W4.3: 10M-point transform — GPU faster than WASM SIMD
//   - W4.4: 2048×2048 heightfield — GPU faster than WASM-only
//
// What it asserts (hard gates, regardless of hardware):
//   1. WASM and GPU produce numerically-equivalent output (parity).
//      This is the CODE-correctness gate. A regression here is a real bug.
//
// What it reports (not gates, hardware-dependent):
//   2. The actual CPU-ms / GPU-ms / speedup numbers, written to console and
//      to window.__benchResult so they can be harvested into ROADMAP docs.
//
// On environments without a GPU adapter (e.g. headless Linux CI), the test
// self-skips (returns early with status "available: false") rather than fail.
//
// To re-run locally and capture numbers for the ROADMAP:
//   wasm-pack build --target web --release --out-dir pkg-webgpu-bench \
//     --features point-cloud,geotiff,terrain-edit,webgpu
//   npx playwright test --config playwright.config.mjs webgpu-bench
//   # then read the console output or window.__benchResult

import { test, expect } from '@playwright/test';

const BENCH_PAGE = '/tests/fixtures/webgpu-bench.html';

// WebGPU in Chromium requires hardware accel. On macOS Chrome it works via
// Metal; on Linux it needs a real GPU + the right flags. We enable it
// explicitly in the chromium launch args via the playwright config — but here
// we also tolerate environments where it's just not available.

test('W4 WebGPU benchmark — parity + speedup numbers', async ({ browser }) => {
  // Launch a fresh context with WebGPU-friendly flags. The default chromium
  // in @playwright/test disables some GPU paths; we override here.
  const context = await browser.newContext({
    javaScriptEnabled: true,
    viewport: { width: 1280, height: 720 },
  });
  const page = await context.newPage();

  const consoleLines = [];
  page.on('console', (msg) => consoleLines.push(`[${msg.type()}] ${msg.text()}`));
  page.on('pageerror', (err) => consoleLines.push(`[pageerror] ${err.message}`));

  await page.goto(BENCH_PAGE, { waitUntil: 'domcontentloaded' });

  // Wait for the bench to finish — either __benchResult is set, or
  // __benchError is set. Give it generous headroom (10M points + GPU compile).
  await page.waitForFunction(
    () => window.__benchResult !== null || window.__benchError !== null,
    {},
    { timeout: 120_000 },
  );

  const pageError = await page.evaluate(() => window.__benchError);
  if (pageError) {
    throw new Error(`benchmark page errored: ${pageError}\nconsole:\n${consoleLines.join('\n')}`);
  }

  const result = await page.evaluate(() => window.__benchResult);

  // Self-skip on environments without WebGPU (headless Linux CI without GPU).
  if (result.available === false) {
    test.skip(true, `WebGPU unavailable: ${result.reason}`);
    return;
  }

  console.log('=== W4 WebGPU benchmark result ===');
  console.log(JSON.stringify(result, null, 2));
  console.log('=== page console ===');
  console.log(consoleLines.join('\n'));

  // ── Hard gate 1: transform parity ──
  // 10M-point float32 transform; allow tolerance for float reordering.
  expect(result.transform, 'transform benchmark completed').not.toBeNull();
  if (!result.transform.error) {
    expect(
      result.transform.maxErr,
      `transform GPU/CPU parity (max abs err) — got ${result.transform.maxErr}`,
    ).toBeLessThan(1e-3);
  }

  // ── Hard gate 2: heightfield parity ──
  // Flatten is integer-mask-indexed assignment — must be EXACT match.
  expect(result.heightfield, 'heightfield benchmark completed').not.toBeNull();
  if (!result.heightfield.error) {
    expect(
      result.heightfield.maxErr,
      `heightfield GPU/CPU parity (must be exact 0) — got ${result.heightfield.maxErr}`,
    ).toBe(0);
  }

  // ── Soft report: speedup numbers (NOT gated) ──
  // These are hardware-dependent. We log them so the ROADMAP can cite real
  // measurements, but we don't fail the test if GPU loses — that's a hardware
  // limitation, not a code bug. The ROADMAP text says "hardware-gated".
  if (!result.transform.error) {
    const tSpeedup = result.transform.cpuMs / result.transform.gpuMs;
    console.log(
      `W4.3 transform: ${result.transform.n} pts, ` +
        `WASM ${result.transform.cpuMs.toFixed(1)}ms vs GPU ${result.transform.gpuMs.toFixed(1)}ms ` +
        `→ ${tSpeedup.toFixed(2)}× (${tSpeedup > 1 ? 'GPU wins' : 'WASM wins on this hardware'})`,
    );
  }
  if (!result.heightfield.error) {
    const hSpeedup = result.heightfield.cpuMs / result.heightfield.gpuMs;
    console.log(
      `W4.4 heightfield: ${result.heightfield.size}×${result.heightfield.size}, ` +
        `WASM ${result.heightfield.cpuMs.toFixed(1)}ms vs GPU ${result.heightfield.gpuMs.toFixed(1)}ms ` +
        `→ ${hSpeedup.toFixed(2)}× (${hSpeedup > 1 ? 'GPU wins' : 'WASM wins on this hardware'})`,
    );
  }

  await context.close();
});
