/**
 * copc-loader tests.
 *
 * Two tiers:
 *  1. Static tests always run (loader shape, magic detection).
 *  2. Real-file tests run when a COPC sample is available at
 *     /tmp/autzen.copc.laz (or $COPC_SAMPLE). Download once:
 *
 *     curl -L -o /tmp/autzen.copc.laz \
 *       https://s3.amazonaws.com/hobu-lidar/autzen-classified.copc.laz
 *
 * Real-file tests spin up a local HTTP server with Range support and verify
 * the full streaming path end to end.
 */
import { readFileSync, statSync } from 'node:fs';
import { createServer } from 'node:http';
import assert from 'node:assert/strict';
import { isCOPCFile, parse, parseSync, loadCOPC, COPCLoader, init } from '../src/index.js';

const SAMPLE =
  process.env.COPC_SAMPLE || '/tmp/autzen.copc.laz';
const hasSample = (() => {
  try {
    return statSync(SAMPLE).isFile();
  } catch {
    return false;
  }
})();

// ── Static tests ────────────────────────────────────────────────────

let passed = 0;
function ok(name, fn) {
  try {
    fn();
    passed++;
    console.log(`  ✅ ${name}`);
  } catch (e) {
    console.error(`  ❌ ${name}\n${e.stack}`);
    process.exitCode = 1;
  }
}

ok('loader object shape', () => {
  assert.equal(COPCLoader.id, 'copc');
  assert.equal(COPCLoader.category, 'pointcloud');
  assert.deepEqual(COPCLoader.extensions, ['copc', 'laz']);
  assert.equal(typeof COPCLoader.parse, 'function');
  assert.equal(typeof COPCLoader.parseSync, 'function');
  assert.equal(typeof COPCLoader.test, 'function');
});

ok('test(): LASF magic accepted', () => {
  const buf = new Uint8Array(400);
  buf.set([0x4c, 0x41, 0x53, 0x46], 0);
  assert.equal(isCOPCFile(buf), true);
});

ok('test(): non-LASF rejected', () => {
  const buf = new Uint8Array(400).fill(0x61);
  assert.equal(isCOPCFile(buf), false);
  assert.equal(isCOPCFile('string'), false);
  assert.equal(isCOPCFile(null), false);
});

// ── Real-file tests ─────────────────────────────────────────────────

if (!hasSample) {
  console.log(`\nℹ️  real-file tests skipped (${SAMPLE} not present)`);
  console.log(`\n${passed} static tests passed`);
} else {
  const bytes = new Uint8Array(readFileSync(SAMPLE));

  /** Minimal static file server with Range support. */
  function serve(buf) {
    const server = createServer((req, res) => {
      const range = req.headers.range;
      if (!range) {
        res.writeHead(200, { 'content-length': buf.length, 'accept-ranges': 'bytes' });
        res.end(buf);
        return;
      }
      const m = /bytes=(\d+)-(\d*)/.exec(range);
      const start = Number(m[1]);
      const end = m[2] ? Math.min(Number(m[2]), buf.length - 1) : buf.length - 1;
      const slice = buf.subarray(start, end + 1);
      res.writeHead(206, {
        'content-length': slice.length,
        'content-range': `bytes ${start}-${end}/${buf.length}`,
        'accept-ranges': 'bytes',
      });
      res.end(slice);
    });
    return new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(server)));
  }

  const server = await serve(bytes);
  const url = `http://127.0.0.1:${server.address().port}/sample.copc.laz`;

  const results = [];
  async function test(name, fn) {
    try {
      await fn();
      passed++;
      console.log(`  ✅ ${name}`);
    } catch (e) {
      console.error(`  ❌ ${name}\n${e.stack}`);
      process.exitCode = 1;
    }
  }

  console.log('\nreal-file tests (streaming over local HTTP):');

  await test('parse: full file', async () => {
    const data = await parse(bytes);
    assert.equal(data.header.vertexCount, 10653336);
    assert.ok(data.attributes.POSITION.value instanceof Float32Array);
    assert.equal(data.attributes.POSITION.value.length, 10653336 * 3);
    assert.ok(data.attributes.COLOR_0, 'format 7 has RGB');
    const b = data.loaderData.copcInfo.bounds;
    assert.ok(Math.abs(b[3] - 639003.73) < 0.01, `maxX ${b[3]}`);
  });

  await test('parse: bbox subset', async () => {
    const info = (await parse(bytes)).loaderData.copcInfo;
    const b = info.bounds;
    const data = await parse(bytes, {
      copc: { bbox: [b[0], b[1], b[2], (b[0] + b[3]) / 2, b[4], b[5]] },
    });
    assert.ok(data.header.vertexCount > 0);
    assert.ok(data.header.vertexCount < 10653336);
    const pos = data.attributes.POSITION.value;
    for (let i = 0; i < pos.length; i += 3) {
      assert.ok(pos[i] <= (b[0] + b[3]) / 2 + 1e-3, 'all x below midplane');
    }
  });

  await test('parseSync after init', async () => {
    await init();
    const data = parseSync(bytes);
    assert.equal(data.header.vertexCount, 10653336);
  });

  await test('loadCOPC: HTTP-range streaming end to end', async () => {
    let lastProgress = null;
    const data = await loadCOPC(url, {
      onProgress: (p) => {
        lastProgress = p;
      },
    });
    assert.equal(data.header.vertexCount, 10653336, 'all points via streaming');
    assert.ok(data.attributes.COLOR_0, 'colors via streaming');
    // Spot-check a few known points (chunk-0 first point from engine tests).
    assert.ok(
      Math.abs(data.attributes.POSITION.value[0] - 636450.56) < 0.5,
      `first x ${data.attributes.POSITION.value[0]}`,
    );
    assert.ok(lastProgress && lastProgress.chunksTotal === 278, 'progress reported for 278 chunks');
  });

  await test('loadCOPC: bbox filter', async () => {
    const b = (await parse(bytes)).loaderData.copcInfo.bounds;
    const data = await loadCOPC(url, {
      bbox: [b[0], b[1], b[2], (b[0] + b[3]) / 2, b[4], b[5]],
    });
    assert.ok(data.header.vertexCount > 0 && data.header.vertexCount < 10653336);
  });

  await test('streaming uses range requests, not full-file downloads', async () => {
    const sizes = [];
    const recordingFetch = async (input, init) => {
      const res = await fetch(input, init);
      const clone = res.clone();
      sizes.push((await clone.arrayBuffer()).byteLength);
      return res;
    };
    // Tiny bbox: chunks are still all fetched (chunk tables carry no
    // per-chunk bboxes — bbox filtering is post-decompression), but the
    // protocol must be many small 206 range fetches, never one big GET.
    await loadCOPC(url, { fetch: recordingFetch, bbox: [0, 0, 0, 1, 1, 1] });
    assert.equal(sizes[0], 375, 'first request fetches only the 375-byte header');
    assert.ok(sizes.length > 10, `many range requests (${sizes.length}), not one GET`);
    const maxSingle = Math.max(...sizes);
    assert.ok(maxSingle < bytes.length / 10, `largest single fetch ${maxSingle} is a fraction of the file`);
  });

  server.close();
  console.log(`\n${passed} tests passed`);
}
