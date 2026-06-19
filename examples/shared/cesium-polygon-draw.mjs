/**
 * Interactive polygon drawing on a Cesium globe for terrain mask selection.
 */

/**
 * @param {Cesium.Viewer} viewer
 * @param {{ onChange?: (state: PolygonDrawerState) => void }} [options]
 */
export function createPolygonDrawer(viewer, options = {}) {
  const Cesium = globalThis.Cesium;
  if (!Cesium) throw new Error('Cesium global required');

  /** @type {{ lng: number, lat: number }[]} */
  let vertices = [];
  let drawing = false;
  let closed = false;
  /** @type {Cesium.ScreenSpaceEventHandler | null} */
  let handler = null;
  /** @type {Cesium.Entity[]} */
  const overlayEntities = [];

  function notify() {
    options.onChange?.({
      drawing,
      closed,
      vertexCount: vertices.length,
      polygon: getPolygonFlat(),
      hasPolygon: vertices.length >= 3,
    });
  }

  function removeOverlays() {
    for (const entity of overlayEntities) {
      viewer.entities.remove(entity);
    }
    overlayEntities.length = 0;
  }

  function refreshOverlays() {
    removeOverlays();
    if (vertices.length === 0) return;

    const positions = vertices.map(
      (v) => Cesium.Cartesian3.fromDegrees(v.lng, v.lat, 0),
    );

    for (const v of vertices) {
      overlayEntities.push(
        viewer.entities.add({
          position: Cesium.Cartesian3.fromDegrees(v.lng, v.lat, 0),
          point: {
            pixelSize: 8,
            color: Cesium.Color.CYAN,
            outlineColor: Cesium.Color.BLACK,
            outlineWidth: 1,
            disableDepthTestDistance: Number.POSITIVE_INFINITY,
          },
        }),
      );
    }

    if (vertices.length >= 2) {
      const linePositions = closed ? [...positions, positions[0]] : positions;
      overlayEntities.push(
        viewer.entities.add({
          polyline: {
            positions: linePositions,
            width: 2,
            material: Cesium.Color.CYAN.withAlpha(0.9),
            clampToGround: true,
          },
        }),
      );
    }

    if (closed && vertices.length >= 3) {
      overlayEntities.push(
        viewer.entities.add({
          polygon: {
            hierarchy: new Cesium.PolygonHierarchy(positions),
            material: Cesium.Color.CYAN.withAlpha(0.2),
            outline: true,
            outlineColor: Cesium.Color.CYAN,
            height: 0,
          },
        }),
      );
    }
  }

  function pickLngLat(screenPosition) {
    const scene = viewer.scene;
    let cartesian = scene.pickPosition(screenPosition);
    if (!Cesium.defined(cartesian)) {
      cartesian = viewer.camera.pickEllipsoid(screenPosition, scene.globe.ellipsoid);
    }
    if (!Cesium.defined(cartesian)) return null;
    const carto = Cesium.Cartographic.fromCartesian(cartesian);
    return {
      lng: Cesium.Math.toDegrees(carto.longitude),
      lat: Cesium.Math.toDegrees(carto.latitude),
    };
  }

  function attachHandler() {
    if (handler) return;
    handler = new Cesium.ScreenSpaceEventHandler(viewer.canvas);
    handler.setInputAction((movement) => {
      if (!drawing) return;
      const picked = pickLngLat(movement.position);
      if (!picked) return;
      closed = false;
      vertices.push(picked);
      refreshOverlays();
      notify();
    }, Cesium.ScreenSpaceEventType.LEFT_CLICK);

    handler.setInputAction(() => {
      if (!drawing || vertices.length < 3) return;
      finish();
    }, Cesium.ScreenSpaceEventType.LEFT_DOUBLE_CLICK);
  }

  function detachHandler() {
    if (handler) {
      handler.destroy();
      handler = null;
    }
  }

  function startDrawing() {
    drawing = true;
    closed = false;
    vertices = [];
    removeOverlays();
    attachHandler();
    viewer.canvas.style.cursor = 'crosshair';
    notify();
  }

  function finish() {
    if (vertices.length < 3) return false;
    drawing = false;
    closed = true;
    detachHandler();
    viewer.canvas.style.cursor = '';
    refreshOverlays();
    notify();
    return true;
  }

  function undoVertex() {
    if (vertices.length === 0) return;
    vertices.pop();
    closed = false;
    refreshOverlays();
    notify();
  }

  function clear() {
    drawing = false;
    closed = false;
    vertices = [];
    detachHandler();
    viewer.canvas.style.cursor = '';
    removeOverlays();
    notify();
  }

  function getPolygonFlat() {
    const flat = [];
    for (const v of vertices) {
      flat.push(v.lng, v.lat);
    }
    if (closed && vertices.length >= 3) {
      flat.push(vertices[0].lng, vertices[0].lat);
    }
    return flat;
  }

  /** Closed ring with ≥3 unique vertices, or null. */
  function getClosedPolygon() {
    if (!closed || vertices.length < 3) return null;
    return getPolygonFlat();
  }

  function destroy() {
    clear();
  }

  return {
    startDrawing,
    finish,
    undoVertex,
    clear,
    getPolygonFlat,
    getClosedPolygon,
    destroy,
    get drawing() {
      return drawing;
    },
    get closed() {
      return closed;
    },
    get vertexCount() {
      return vertices.length;
    },
  };
}

/**
 * Draw a geographic bounds rectangle helper (west,south,east,north).
 * @param {Cesium.Viewer} viewer
 * @param {number[]} bounds
 * @returns {() => void} cleanup
 */
export function showBoundsOutline(viewer, bounds) {
  const Cesium = globalThis.Cesium;
  const [west, south, east, north] = bounds;
  const entity = viewer.entities.add({
    name: 'terrain-bounds',
    rectangle: {
      coordinates: Cesium.Rectangle.fromDegrees(west, south, east, north),
      material: Cesium.Color.ORANGE.withAlpha(0.08),
      outline: true,
      outlineColor: Cesium.Color.ORANGE,
      height: 0,
    },
  });
  return () => viewer.entities.remove(entity);
}

/**
 * @typedef {{ drawing: boolean, closed: boolean, vertexCount: number, polygon: number[], hasPolygon: boolean }} PolygonDrawerState
 */
