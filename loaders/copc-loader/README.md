# copc-loader

COPC (Cloud Optimized Point Cloud) loader for [loaders.gl](https://loaders.gl) —
WASM LAZ decompression in the browser and Node, HTTP-range streaming off any
static file host. No server, no ETL, no PDAL.

`loaders.gl` has no COPC loader today (tracked in
[visgl/loaders.gl#2911](https://github.com/visgl/loaders.gl/issues/2911)) —
this package fills that gap, powered by the
[wasm-spatial-core](https://www.npmjs.com/package/wasm-spatial-core) engine.

- **Verified against real data** — decoded chunk-exact against
  `autzen-classified.copc.laz` (10.65 M points, 278 chunks, point format 7).
- **loaders.gl-compatible** — plain loader object, works with
  `parse()`/`load()` from `@loaders.gl/core`.
- **Streaming** — `loadCOPC(url)` fetches the header + chunk table (~KB),
  then decompresses chunk byte-ranges as they arrive.
- **deck.gl-ready** — returns `POSITION` / `COLOR_0` attributes.

## Install

```bash
npm install copc-loader
```

## Parse a whole file (loaders.gl)

```js
import { load } from '@loaders.gl/core';
import { COPCLoader } from 'copc-loader';

const data = await load('https://example.com/scan.copc.laz', COPCLoader, {
  copc: { bbox: [636000, 849000, 400, 637000, 850000, 500] }, // optional subset
});

data.header.vertexCount;      // 10_653_336
data.attributes.POSITION;     // { value: Float32Array, size: 3 }
data.attributes.COLOR_0;      // { value: Uint8Array, size: 3 } (when present)
```

## Stream over HTTP without downloading the whole file

```js
import { loadCOPC } from 'copc-loader';

const controller = new AbortController();
const data = await loadCOPC('https://example.com/scan.copc.laz', {
  bbox: [636000, 849000, 400, 637000, 850000, 500], // post-decode filter
  signal: controller.signal,
  onProgress: ({ chunksDone, chunksTotal, points }) =>
    console.log(`${chunksDone}/${chunksTotal} chunks, ${points} points`),
});
```

Requires a host that honors `Range` requests (S3, GitHub Pages, most CDNs and
static servers). If the server ignores ranges, the loader falls back to
downloading and parsing the full file.

## deck.gl

```js
import { Deck } from '@deck.gl/core';
import { PointCloudLayer } from '@deck.gl/layers';
import { loadCOPC } from 'copc-loader';

const { attributes } = await loadCOPC('https://example.com/scan.copc.laz');

new Deck({
  layers: [
    new PointCloudLayer({
      id: 'copc',
      data: { length: attributes.POSITION.value.length / 3, attributes },
      getColor: [255, 255, 255],
      pointSize: 2,
    }),
  ],
});
```

## API

### `COPCLoader`

loaders.gl loader object — `id: 'copc'`, extensions `['copc', 'laz']`,
`test` on the `LASF` magic. Use with any loaders.gl `load`/`parse`.

### `parse(data, options?)`

Parse full file bytes. `options.copc.bbox` restricts output to a
`[minX, minY, minZ, maxX, maxY, maxZ]` subset (hierarchy-aware for COPC
files — only intersecting chunks are decompressed).

### `parseSync(data, options?)`

Synchronous variant; call `await init()` (or any async API) first.

### `loadCOPC(url, options?)`

HTTP-range streaming (see above). Chunks are fetched with bounded
concurrency (6) and decompressed as they arrive.

### `setCore(core)`

Inject an already-initialized `wasm-spatial-core` API instance, if your app
already uses the engine.

## Notes

- Positions are `Float32Array` XYZ in the file's CRS (COPC is typically a
  projected CRS — e.g. UTM meters — check `loaderData.copcInfo` for
  scale/offset/bounds).
- Colors are 8-bit RGB derived from the high bytes of the u16 LAS channels.
- Node ≥ 18 (uses global `fetch`).

## License

MIT
