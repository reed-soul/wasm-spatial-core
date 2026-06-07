/**
 * Lightweight WebGL point cloud viewer with orbit controls.
 * Zero dependencies — used by the demo hub and webgl-pointcloud demo.
 */

const VS_WEBGL2 = `#version 300 es
precision highp float;
layout(location=0) in vec3 a_position;
layout(location=1) in vec3 a_color;
uniform mat4 u_mvp;
uniform float u_pointSize;
uniform vec3 u_cameraPos;
out vec3 v_color;
void main() {
  vec4 worldPos = vec4(a_position, 1.0);
  gl_Position = u_mvp * worldPos;
  float dist = length(a_position - u_cameraPos);
  gl_PointSize = clamp(u_pointSize * (300.0 / max(dist, 1.0)), 1.0, 64.0);
  v_color = a_color;
}`;

const FS_WEBGL2 = `#version 300 es
precision highp float;
in vec3 v_color;
out vec4 fragColor;
void main() {
  vec2 cxy = 2.0 * gl_PointCoord - 1.0;
  float r = dot(cxy, cxy);
  if (r > 1.0) discard;
  float alpha = 1.0 - smoothstep(0.7, 1.0, r);
  fragColor = vec4(v_color * (1.0 - 0.25 * r), alpha);
}`;

const VS_WEBGL1 = `precision highp float;
attribute vec3 a_position;
attribute vec3 a_color;
uniform mat4 u_mvp;
uniform float u_pointSize;
uniform vec3 u_cameraPos;
varying vec3 v_color;
void main() {
  gl_Position = u_mvp * vec4(a_position, 1.0);
  float dist = length(a_position - u_cameraPos);
  gl_PointSize = clamp(u_pointSize * (300.0 / max(dist, 1.0)), 1.0, 64.0);
  v_color = a_color;
}`;

const FS_WEBGL1 = `precision highp float;
varying vec3 v_color;
void main() {
  vec2 cxy = 2.0 * gl_PointCoord - 1.0;
  float r = dot(cxy, cxy);
  if (r > 1.0) discard;
  float alpha = 1.0 - smoothstep(0.7, 1.0, r);
  gl_FragColor = vec4(v_color * (1.0 - 0.25 * r), alpha);
}`;

function compileShader(gl, src, type) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, src);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    throw new Error(gl.getShaderInfoLog(shader) || 'Shader compile failed');
  }
  return shader;
}

function createProgram(gl, vsSrc, fsSrc) {
  const program = gl.createProgram();
  gl.attachShader(program, compileShader(gl, vsSrc, gl.VERTEX_SHADER));
  gl.attachShader(program, compileShader(gl, fsSrc, gl.FRAGMENT_SHADER));
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(program) || 'Program link failed');
  }
  return program;
}

function mat4Identity() {
  return new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]);
}

function mat4Perspective(fov, aspect, near, far) {
  const f = 1.0 / Math.tan(fov / 2);
  const nf = 1 / (near - far);
  return new Float32Array([
    f / aspect, 0, 0, 0,
    0, f, 0, 0,
    0, 0, (far + near) * nf, -1,
    0, 0, 2 * far * near * nf, 0,
  ]);
}

function mat4LookAt(eye, center, up) {
  const zx = eye[0] - center[0];
  const zy = eye[1] - center[1];
  const zz = eye[2] - center[2];
  let len = Math.hypot(zx, zy, zz);
  const z0 = zx / len, z1 = zy / len, z2 = zz / len;
  const xx = up[1] * z2 - up[2] * z1;
  const xy = up[2] * z0 - up[0] * z2;
  const xz = up[0] * z1 - up[1] * z0;
  len = Math.hypot(xx, xy, xz);
  const x0 = xx / len, x1 = xy / len, x2 = xz / len;
  const y0 = z1 * x2 - z2 * x1;
  const y1 = z2 * x0 - z0 * x2;
  const y2 = z0 * x1 - z1 * x0;
  return new Float32Array([
    x0, y0, z0, 0,
    x1, y1, z1, 0,
    x2, y2, z2, 0,
    -(x0 * eye[0] + x1 * eye[1] + x2 * eye[2]),
    -(y0 * eye[0] + y1 * eye[1] + y2 * eye[2]),
    -(z0 * eye[0] + z1 * eye[1] + z2 * eye[2]),
    1,
  ]);
}

function mat4Multiply(a, b) {
  const out = new Float32Array(16);
  for (let c = 0; c < 4; c++) {
    for (let r = 0; r < 4; r++) {
      out[c * 4 + r] =
        a[r] * b[c * 4] +
        a[4 + r] * b[c * 4 + 1] +
        a[8 + r] * b[c * 4 + 2] +
        a[12 + r] * b[c * 4 + 3];
    }
  }
  return out;
}

function mat4Translate(m, tx, ty, tz) {
  const out = new Float32Array(m);
  out[12] += m[0] * tx + m[4] * ty + m[8] * tz;
  out[13] += m[1] * tx + m[5] * ty + m[9] * tz;
  out[14] += m[2] * tx + m[6] * ty + m[10] * tz;
  out[15] += m[3] * tx + m[7] * ty + m[11] * tz;
  return out;
}

function computeBounds(positions) {
  let minX = Infinity, minY = Infinity, minZ = Infinity;
  let maxX = -Infinity, maxY = -Infinity, maxZ = -Infinity;
  const n = positions.length / 3;
  for (let i = 0; i < n; i++) {
    const x = positions[i * 3];
    const y = positions[i * 3 + 1];
    const z = positions[i * 3 + 2];
    minX = Math.min(minX, x); minY = Math.min(minY, y); minZ = Math.min(minZ, z);
    maxX = Math.max(maxX, x); maxY = Math.max(maxY, y); maxZ = Math.max(maxZ, z);
  }
  return { minX, minY, minZ, maxX, maxY, maxZ };
}

function heightColors(positions, bounds) {
  const n = positions.length / 3;
  const colors = new Float32Array(n * 3);
  const rangeZ = bounds.maxZ - bounds.minZ || 1;
  for (let i = 0; i < n; i++) {
    const t = (positions[i * 3 + 2] - bounds.minZ) / rangeZ;
    colors[i * 3] = Math.min(1, Math.max(0, t < 0.5 ? 0 : t < 0.75 ? (t - 0.5) * 4 : 1));
    colors[i * 3 + 1] = Math.min(1, Math.max(0, t < 0.25 ? t * 4 : t < 0.75 ? 1 : 1 - (t - 0.75) * 4));
    colors[i * 3 + 2] = Math.min(1, Math.max(0, t < 0.25 ? 1 : t < 0.5 ? 1 - (t - 0.25) * 4 : 0));
  }
  return colors;
}

function originalColors(rgbBytes) {
  const n = rgbBytes.length / 3;
  const colors = new Float32Array(n * 3);
  for (let i = 0; i < n * 3; i++) colors[i] = rgbBytes[i] / 255;
  return colors;
}

export function extentFromPositions(positions) {
  const b = computeBounds(positions);
  return Math.max(b.maxX - b.minX, b.maxY - b.minY, b.maxZ - b.minZ, 1);
}

export function suggestPointSize(positions, base = 2.5) {
  const size = extentFromPositions(positions);
  return Math.min(8, Math.max(1.5, base * (80 / size)));
}

export function generateTerrainCloud(count = 100_000) {
  const positions = new Float32Array(count * 3);
  const colors = new Uint8Array(count * 3);

  for (let i = 0; i < count; i++) {
    const x = (Math.random() - 0.5) * 100;
    const y = (Math.random() - 0.5) * 100;
    const z =
      Math.sin(x * 0.1) * Math.cos(y * 0.1) * 8 +
      Math.exp(-(x * x + y * y) / 400) * 15 +
      Math.random() * 1.5;

    positions[i * 3] = x;
    positions[i * 3 + 1] = y;
    positions[i * 3 + 2] = z;

    const t = Math.max(0, Math.min(1, (z + 5) / 25));
    if (t < 0.35) {
      colors[i * 3] = 30;
      colors[i * 3 + 1] = 90 + Math.floor((t / 0.35) * 120);
      colors[i * 3 + 2] = 40;
    } else if (t < 0.7) {
      colors[i * 3] = 120 + Math.floor(((t - 0.35) / 0.35) * 100);
      colors[i * 3 + 1] = 170;
      colors[i * 3 + 2] = 50;
    } else {
      colors[i * 3] = 210;
      colors[i * 3 + 1] = 210;
      colors[i * 3 + 2] = 230;
    }
  }

  return { positions, colors };
}

export class PointCloudViewer {
  constructor(canvas, options = {}) {
    this.canvas = canvas;
    this.pointSize = options.pointSize ?? 2.5;
    this.colorMode = options.colorMode ?? 'height';

    this.gl =
      canvas.getContext('webgl2', { antialias: true, alpha: false }) ||
      canvas.getContext('webgl', { antialias: true, alpha: false });
    if (!this.gl) throw new Error('WebGL not supported');

    const isWebGL2 = this.gl instanceof WebGL2RenderingContext;
    this.program = createProgram(
      this.gl,
      isWebGL2 ? VS_WEBGL2 : VS_WEBGL1,
      isWebGL2 ? FS_WEBGL2 : FS_WEBGL1,
    );

    this.loc = {
      position: this.gl.getAttribLocation(this.program, 'a_position'),
      color: this.gl.getAttribLocation(this.program, 'a_color'),
      mvp: this.gl.getUniformLocation(this.program, 'u_mvp'),
      pointSize: this.gl.getUniformLocation(this.program, 'u_pointSize'),
      cameraPos: this.gl.getUniformLocation(this.program, 'u_cameraPos'),
    };

    this.posBuffer = this.gl.createBuffer();
    this.colorBuffer = this.gl.createBuffer();

    this.positions = null;
    this.rgbBytes = null;
    this.bounds = null;
    this.numPoints = 0;

    this.rotationX = -0.45;
    this.rotationY = 0.65;
    this.distance = 120;
    this.panX = 0;
    this.panY = 0;
    this.center = [0, 0, 0];

    this.isDragging = false;
    this.isPanning = false;
    this.lastMouse = [0, 0];
    this.lastTouchDist = 0;
    this.running = false;
    this.rafId = 0;

    this._bindEvents();
  }

  setData(positions, rgbBytes = null, { recenter = true } = {}) {
    this.positions = positions instanceof Float32Array ? positions : new Float32Array(positions);
    this.rgbBytes = rgbBytes
      ? rgbBytes instanceof Uint8Array
        ? rgbBytes
        : new Uint8Array(rgbBytes)
      : null;
    this.numPoints = this.positions.length / 3;
    this.bounds = computeBounds(this.positions);

    if (recenter) {
      const { minX, minY, minZ, maxX, maxY, maxZ } = this.bounds;
      const cx = (minX + maxX) / 2;
      const cy = (minY + maxY) / 2;
      const cz = (minZ + maxZ) / 2;
      for (let i = 0; i < this.numPoints; i++) {
        this.positions[i * 3] -= cx;
        this.positions[i * 3 + 1] -= cy;
        this.positions[i * 3 + 2] -= cz;
      }
      this.bounds = computeBounds(this.positions);
      this.center = [0, 0, 0];
      const size = Math.max(
        this.bounds.maxX - this.bounds.minX,
        this.bounds.maxY - this.bounds.minY,
        this.bounds.maxZ - this.bounds.minZ,
      );
      this.distance = Math.max(size * 1.6, 10);
      this.rotationX = -0.45;
      this.rotationY = 0.65;
      this.panX = 0;
      this.panY = 0;
    }

    this._uploadBuffers();
  }

  setColorMode(mode) {
    this.colorMode = mode;
    this._uploadBuffers();
  }

  setPointSize(size) {
    this.pointSize = size;
  }

  resetView() {
    if (!this.bounds) return;
    const size = Math.max(
      this.bounds.maxX - this.bounds.minX,
      this.bounds.maxY - this.bounds.minY,
      this.bounds.maxZ - this.bounds.minZ,
    );
    this.distance = Math.max(size * 1.6, 10);
    this.rotationX = -0.45;
    this.rotationY = 0.65;
    this.panX = 0;
    this.panY = 0;
  }

  start() {
    if (this.running) return;
    this.running = true;
    const loop = () => {
      if (!this.running) return;
      this._draw();
      this.rafId = requestAnimationFrame(loop);
    };
    this.rafId = requestAnimationFrame(loop);
  }

  stop() {
    this.running = false;
    if (this.rafId) cancelAnimationFrame(this.rafId);
  }

  destroy() {
    this.stop();
    this._unbindEvents();
    this.gl.deleteBuffer(this.posBuffer);
    this.gl.deleteBuffer(this.colorBuffer);
    this.gl.deleteProgram(this.program);
  }

  _colorsForMode() {
    if (!this.positions || this.numPoints === 0) return new Float32Array(0);
    if (this.colorMode === 'original' && this.rgbBytes?.length) {
      return originalColors(this.rgbBytes);
    }
    return heightColors(this.positions, this.bounds);
  }

  _uploadBuffers() {
    if (!this.positions) return;
    const colors = this._colorsForMode();
    const gl = this.gl;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.posBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, this.positions, gl.STATIC_DRAW);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.colorBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, colors, gl.STATIC_DRAW);
  }

  _cameraPosition() {
    const [cx, cy, cz] = this.center;
    return [
      cx + this.distance * Math.cos(this.rotationX) * Math.sin(this.rotationY),
      cy + this.distance * Math.sin(this.rotationX),
      cz + this.distance * Math.cos(this.rotationX) * Math.cos(this.rotationY),
    ];
  }

  _mvpMatrix() {
    const rect = this.canvas.getBoundingClientRect();
    const aspect = rect.width / Math.max(rect.height, 1);
    const proj = mat4Perspective(Math.PI / 4, aspect, 0.1, 50_000);
    const eye = this._cameraPosition();
    const view = mat4LookAt(eye, this.center, [0, 1, 0]);
    const model = mat4Translate(mat4Identity(), this.panX, this.panY, 0);
    return mat4Multiply(proj, mat4Multiply(view, model));
  }

  _resize() {
    const rect = this.canvas.parentElement?.getBoundingClientRect() ?? this.canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    const w = Math.max(1, Math.floor(rect.width * dpr));
    const h = Math.max(1, Math.floor(rect.height * dpr));
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w;
      this.canvas.height = h;
    }
  }

  _draw() {
    if (!this.positions || this.numPoints === 0) return;

    this._resize();
    const gl = this.gl;
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.clearColor(0.02, 0.04, 0.07, 1);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    gl.enable(gl.DEPTH_TEST);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

    gl.useProgram(this.program);
    gl.uniformMatrix4fv(this.loc.mvp, false, this._mvpMatrix());
    gl.uniform1f(this.loc.pointSize, this.pointSize);
    gl.uniform3fv(this.loc.cameraPos, this._cameraPosition());

    gl.bindBuffer(gl.ARRAY_BUFFER, this.posBuffer);
    gl.enableVertexAttribArray(this.loc.position);
    gl.vertexAttribPointer(this.loc.position, 3, gl.FLOAT, false, 0, 0);

    gl.bindBuffer(gl.ARRAY_BUFFER, this.colorBuffer);
    gl.enableVertexAttribArray(this.loc.color);
    gl.vertexAttribPointer(this.loc.color, 3, gl.FLOAT, false, 0, 0);

    gl.drawArrays(gl.POINTS, 0, this.numPoints);
  }

  _onMouseDown = (e) => {
    if (e.button === 0) this.isDragging = true;
    else if (e.button === 2) this.isPanning = true;
    this.lastMouse = [e.clientX, e.clientY];
    this.canvas.style.cursor = 'grabbing';
  };

  _onMouseMove = (e) => {
    if (!this.isDragging && !this.isPanning) return;
    const dx = e.clientX - this.lastMouse[0];
    const dy = e.clientY - this.lastMouse[1];
    if (this.isDragging) {
      this.rotationY -= dx * 0.005;
      this.rotationX += dy * 0.005;
      this.rotationX = Math.max(-Math.PI / 2 + 0.01, Math.min(Math.PI / 2 - 0.01, this.rotationX));
    } else {
      this.panX -= dx * 0.4;
      this.panY += dy * 0.4;
    }
    this.lastMouse = [e.clientX, e.clientY];
  };

  _onMouseUp = () => {
    this.isDragging = false;
    this.isPanning = false;
    this.canvas.style.cursor = 'grab';
  };

  _onWheel = (e) => {
    e.preventDefault();
    const factor = e.deltaY > 0 ? 1.08 : 0.92;
    this.distance = Math.max(2, Math.min(20_000, this.distance * factor));
  };

  _onDblClick = () => this.resetView();

  _onTouchStart = (e) => {
    if (e.touches.length === 1) {
      this.isDragging = true;
      this.lastMouse = [e.touches[0].clientX, e.touches[0].clientY];
    } else if (e.touches.length === 2) {
      const dx = e.touches[0].clientX - e.touches[1].clientX;
      const dy = e.touches[0].clientY - e.touches[1].clientY;
      this.lastTouchDist = Math.hypot(dx, dy);
    }
  };

  _onTouchMove = (e) => {
    e.preventDefault();
    if (e.touches.length === 1 && this.isDragging) {
      const dx = e.touches[0].clientX - this.lastMouse[0];
      const dy = e.touches[0].clientY - this.lastMouse[1];
      this.rotationY -= dx * 0.005;
      this.rotationX += dy * 0.005;
      this.rotationX = Math.max(-Math.PI / 2 + 0.01, Math.min(Math.PI / 2 - 0.01, this.rotationX));
      this.lastMouse = [e.touches[0].clientX, e.touches[0].clientY];
    } else if (e.touches.length === 2) {
      const dx = e.touches[0].clientX - e.touches[1].clientX;
      const dy = e.touches[0].clientY - e.touches[1].clientY;
      const dist = Math.hypot(dx, dy);
      if (this.lastTouchDist > 0) {
        this.distance = Math.max(2, Math.min(20_000, this.distance * (this.lastTouchDist / dist)));
      }
      this.lastTouchDist = dist;
    }
  };

  _onTouchEnd = () => {
    this.isDragging = false;
    this.lastTouchDist = 0;
  };

  _bindEvents() {
    this.canvas.style.cursor = 'grab';
    this.canvas.addEventListener('mousedown', this._onMouseDown);
    this.canvas.addEventListener('mousemove', this._onMouseMove);
    this.canvas.addEventListener('mouseup', this._onMouseUp);
    this.canvas.addEventListener('mouseleave', this._onMouseUp);
    this.canvas.addEventListener('contextmenu', (e) => e.preventDefault());
    this.canvas.addEventListener('wheel', this._onWheel, { passive: false });
    this.canvas.addEventListener('dblclick', this._onDblClick);
    this.canvas.addEventListener('touchstart', this._onTouchStart, { passive: true });
    this.canvas.addEventListener('touchmove', this._onTouchMove, { passive: false });
    this.canvas.addEventListener('touchend', this._onTouchEnd);
  }

  _unbindEvents() {
    this.canvas.removeEventListener('mousedown', this._onMouseDown);
    this.canvas.removeEventListener('mousemove', this._onMouseMove);
    this.canvas.removeEventListener('mouseup', this._onMouseUp);
    this.canvas.removeEventListener('mouseleave', this._onMouseUp);
    this.canvas.removeEventListener('wheel', this._onWheel);
    this.canvas.removeEventListener('dblclick', this._onDblClick);
    this.canvas.removeEventListener('touchstart', this._onTouchStart);
    this.canvas.removeEventListener('touchmove', this._onTouchMove);
    this.canvas.removeEventListener('touchend', this._onTouchEnd);
  }
}
