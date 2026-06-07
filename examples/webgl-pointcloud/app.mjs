import init, {
  parseLasPointsWithProgress,
  parsePointCloudAuto,
  pointCloudStats,
} from '../../pkg/wasm_spatial_core.js';
import { PointCloudViewer, generateTerrainCloud, suggestPointSize } from '../shared/pc-viewer.mjs';

const state = { viewer: null, wasmReady: false };

function $(id) {
  return document.getElementById(id);
}

function formatBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1048576).toFixed(1)} MB`;
}

function showOverlay(text) {
  $('overlay').classList.remove('hidden');
  $('overlayText').textContent = text;
}

function hideOverlay() {
  $('overlay').classList.add('hidden');
  $('progressBar').style.display = 'none';
}

function showProgress(pct) {
  $('progressBar').style.display = '';
  $('progressFill').style.width = `${pct}%`;
}

async function loadWasm() {
  showOverlay('Loading WASM…');
  try {
    await init();
    state.wasmReady = true;
    hideOverlay();
  } catch (e) {
    $('overlayText').textContent = `WASM load failed: ${e.message}`;
  }
}

function ensureViewer() {
  if (state.viewer) return state.viewer;
  state.viewer = new PointCloudViewer($('glCanvas'), { pointSize: 2, colorMode: 'height' });
  state.viewer.start();
  return state.viewer;
}

function updateInfo({ numPoints, fileSize, bounds }) {
  $('infoPanel').style.display = '';
  $('statPoints').textContent = numPoints.toLocaleString();
  $('statSize').textContent = fileSize;
  $('statBoundsX').textContent = `${bounds.minX.toFixed(1)} → ${bounds.maxX.toFixed(1)}`;
  $('statBoundsY').textContent = `${bounds.minY.toFixed(1)} → ${bounds.maxY.toFixed(1)}`;
  $('statBoundsZ').textContent = `${bounds.minZ.toFixed(1)} → ${bounds.maxZ.toFixed(1)}`;
  $('statRendered').textContent = `${numPoints.toLocaleString()} points`;
}

function boundsFromPositions(positions) {
  let minX = Infinity, minY = Infinity, minZ = Infinity;
  let maxX = -Infinity, maxY = -Infinity, maxZ = -Infinity;
  for (let i = 0; i < positions.length; i += 3) {
    minX = Math.min(minX, positions[i]);
    minY = Math.min(minY, positions[i + 1]);
    minZ = Math.min(minZ, positions[i + 2]);
    maxX = Math.max(maxX, positions[i]);
    maxY = Math.max(maxY, positions[i + 1]);
    maxZ = Math.max(maxZ, positions[i + 2]);
  }
  return { minX, minY, minZ, maxX, maxY, maxZ };
}

async function displayCloud(positions, colors, meta) {
  const viewer = ensureViewer();
  const size = suggestPointSize(positions, state.pointSize);
  state.pointSize = size;
  $('pointSize').value = String(size);
  $('pointSizeLabel').textContent = size.toFixed(1);
  viewer.setColorMode(state.colorMode);
  viewer.setPointSize(size);
  viewer.setData(positions, colors);

  const b = viewer.bounds || boundsFromPositions(positions);
  updateInfo({
    numPoints: positions.length / 3,
    fileSize: meta.fileSize,
    bounds: b,
  });
  hideOverlay();
}

async function loadPointCloud(file) {
  if (!state.wasmReady) return;
  showOverlay(`Loading ${file.name}…`);
  showProgress(10);

  const bytes = new Uint8Array(await file.arrayBuffer());
  showProgress(25);

  let cloud;
  try {
    cloud = parseLasPointsWithProgress(bytes, (done, total) => {
      const pct = 25 + Math.floor((done / total) * 50);
      showProgress(pct);
      $('overlayText').textContent = `Parsing ${done.toLocaleString()} / ${total.toLocaleString()} points…`;
    });
  } catch (e) {
    cloud = parsePointCloudAuto(bytes);
  }

  const positions = new Float32Array(cloud.positions);
  const colors = cloud.colors ? new Uint8Array(cloud.colors) : null;
  showProgress(90);

  try {
    const stats = JSON.parse(pointCloudStats(positions));
    console.log('[pointCloudStats]', stats);
  } catch (e) { /* optional */ }

  await displayCloud(positions, colors, { fileSize: formatBytes(file.size) });
}

async function loadDemoCloud() {
  if (!state.wasmReady) return;
  showOverlay('Generating demo point cloud…');
  showProgress(20);

  const { positions, colors } = generateTerrainCloud(50_000);
  showProgress(80);
  await displayCloud(positions, colors, { fileSize: '(demo)' });
}

function setupUi() {
  state.colorMode = 'height';
  state.pointSize = 2;

  $('dropZone').addEventListener('click', () => $('fileInput').click());
  $('dropZone').addEventListener('dragover', (e) => {
    e.preventDefault();
    $('dropZone').classList.add('dragover');
  });
  $('dropZone').addEventListener('dragleave', () => $('dropZone').classList.remove('dragover'));
  $('dropZone').addEventListener('drop', (e) => {
    e.preventDefault();
    $('dropZone').classList.remove('dragover');
    if (e.dataTransfer.files.length > 0) loadPointCloud(e.dataTransfer.files[0]);
  });
  $('fileInput').addEventListener('change', (e) => {
    if (e.target.files.length > 0) loadPointCloud(e.target.files[0]);
  });
  $('loadDemo').addEventListener('click', loadDemoCloud);

  document.querySelectorAll('[data-color]').forEach((btn) => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('[data-color]').forEach((b) => b.classList.remove('active'));
      btn.classList.add('active');
      state.colorMode = btn.dataset.color;
      state.viewer?.setColorMode(state.colorMode);
    });
  });

  $('pointSize').addEventListener('input', (e) => {
    state.pointSize = parseFloat(e.target.value);
    $('pointSizeLabel').textContent = state.pointSize.toFixed(1);
    state.viewer?.setPointSize(state.pointSize);
  });

  // FPS overlay (approximate — viewer has no hook, count rAF externally)
  let frames = 0;
  let last = performance.now();
  const tickFps = () => {
    frames++;
    const now = performance.now();
    if (now - last >= 1000) {
      $('fpsCounter').textContent = `${frames} FPS`;
      frames = 0;
      last = now;
    }
    requestAnimationFrame(tickFps);
  };
  requestAnimationFrame(tickFps);
}

async function main() {
  setupUi();
  await loadWasm();
  ensureViewer();
  if (!$('overlay').classList.contains('hidden')) hideOverlay();
}

main();
