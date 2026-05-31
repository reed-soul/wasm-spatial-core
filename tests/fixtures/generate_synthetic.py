#!/usr/bin/env python3
"""Generate high-quality synthetic test data that simulates real-world spatial datasets.

Files produced:
  - terrain_64x64.tif: 64x64 Float32 GeoTIFF with realistic mountain terrain
  - terrain_256x256.tif: 256x256 Float32 GeoTIFF (larger, for pipeline tests)
  - bunny_color.ply: Stanford Bunny-like PLY with ~3000 vertices
  - cube.obj: Simple triangulated cube OBJ
"""

import struct
import math
import os
import random

FIXTURES_DIR = os.path.dirname(os.path.abspath(__file__))

# ==========================================================================
# GeoTIFF builder (minimal Float32 uncompressed)
# ==========================================================================

def build_tiff_f32(width, height, values, geo_keys=None):
    """Build a minimal uncompressed Float32 GeoTIFF."""
    assert len(values) == width * height

    buf = bytearray()
    # TIFF Header (little-endian)
    buf.extend(b'II')
    buf.extend(struct.pack('<H', 42))
    ifd_offset_pos = len(buf)
    buf.extend(struct.pack('<I', 0))  # placeholder

    # IFD start
    ifd_start = len(buf)
    buf[ifd_offset_pos:ifd_offset_pos+4] = struct.pack('<I', ifd_start)

    # Calculate layout
    num_entries = 8 if geo_keys else 7
    entries_end = ifd_start + 2 + num_entries * 12 + 4
    next_data = entries_end

    if geo_keys:
        geo_key_offset = next_data
        geo_key_size = len(geo_keys) * 2
        strip_data_offset = geo_key_offset + geo_key_size
    else:
        geo_key_offset = None
        strip_data_offset = next_data

    strip_data_size = len(values) * 4

    # Number of IFD entries
    buf.extend(struct.pack('<H', num_entries))

    # Tag 256: ImageWidth
    buf.extend(struct.pack('<HHI', 256, 4, 1))
    buf.extend(struct.pack('<I', width))
    # Tag 257: ImageLength
    buf.extend(struct.pack('<HHI', 257, 4, 1))
    buf.extend(struct.pack('<I', height))
    # Tag 258: BitsPerSample
    buf.extend(struct.pack('<HHI', 258, 3, 1))
    buf.extend(struct.pack('<HH', 32, 0))
    # Tag 259: Compression (1=none)
    buf.extend(struct.pack('<HHI', 259, 3, 1))
    buf.extend(struct.pack('<HH', 1, 0))
    # Tag 273: StripOffsets
    buf.extend(struct.pack('<HHI', 273, 4, 1))
    buf.extend(struct.pack('<I', strip_data_offset))
    # Tag 278: RowsPerStrip
    buf.extend(struct.pack('<HHI', 278, 4, 1))
    buf.extend(struct.pack('<I', height))
    # Tag 279: StripByteCounts
    buf.extend(struct.pack('<HHI', 279, 4, 1))
    buf.extend(struct.pack('<I', strip_data_size))
    # Tag 339: SampleFormat (3=IEEEFP)
    buf.extend(struct.pack('<HHI', 339, 3, 1))
    buf.extend(struct.pack('<HH', 3, 0))

    if geo_keys:
        # Tag 34735: GeoKeyDirectory
        buf.extend(struct.pack('<HHI', 34735, 3, len(geo_keys)))
        buf.extend(struct.pack('<I', geo_key_offset))

    # Next IFD = 0
    buf.extend(struct.pack('<I', 0))

    # GeoKeyDirectory data
    if geo_keys:
        for key in geo_keys:
            buf.extend(struct.pack('<H', key))

    # Strip data (Float32 little-endian)
    for v in values:
        buf.extend(struct.pack('<f', v))

    return bytes(buf)


# ==========================================================================
# Generate realistic terrain elevation data
# ==========================================================================

def generate_terrain(width, height, seed=42):
    """Generate terrain with realistic features: mountains, valleys, ridges."""
    rng = random.Random(seed)

    # Start with multiple octaves of noise-like terrain
    values = [0.0] * (width * height)

    # Layer 1: Large-scale hills (Gaussian bumps)
    peaks = [(rng.uniform(0.1, 0.9), rng.uniform(0.1, 0.9),
              rng.uniform(0.15, 0.35), rng.uniform(200, 800))
             for _ in range(5)]

    for y in range(height):
        for x in range(width):
            fx, fy = x / width, y / height
            h = 50.0  # base elevation
            for px, py, sigma, amplitude in peaks:
                d2 = (fx - px)**2 + (fy - py)**2
                h += amplitude * math.exp(-d2 / (2 * sigma**2))
            values[y * width + x] = h

    # Layer 2: Ridge line
    for y in range(height):
        fy = y / height
        ridge_x = 0.3 + 0.4 * fy + 0.05 * math.sin(fy * 12)
        for x in range(width):
            fx = x / width
            d = abs(fx - ridge_x)
            values[y * width + x] += 150 * math.exp(-d * d / 0.005)

    # Layer 3: Small-scale noise
    for i in range(len(values)):
        values[i] += rng.gauss(0, 3)

    # Layer 4: Valley
    for y in range(height):
        fy = y / height
        valley_x = 0.6 + 0.1 * math.sin(fy * 8)
        for x in range(width):
            fx = x / width
            d = abs(fx - valley_x)
            values[y * width + x] -= 80 * math.exp(-d * d / 0.003)

    return values


# ==========================================================================
# Generate PLY file (bunny-like triangulated mesh)
# ==========================================================================

def generate_ply_mesh(n=3000, seed=42):
    """Generate a triangulated mesh PLY with vertex colors (simulating a 3D scan)."""
    rng = random.Random(seed)

    # Generate vertices on a deformed sphere (bunny-like)
    vertices = []
    for i in range(n):
        theta = math.acos(1 - 2 * (i + 0.5) / n)  # polar angle
        phi = math.pi * (1 + 5**0.5) * i  # golden ratio spiral

        r = 1.0
        # Deform: elongate, add bumps
        x = r * math.sin(theta) * math.cos(phi)
        y = r * math.sin(theta) * math.sin(phi)
        z = r * math.cos(theta)

        # Elongate (bunny shape)
        z *= 1.3
        x *= 0.8

        # Ears
        if abs(x) > 0.5 and z > 0.8:
            z += 0.3 * (abs(x) - 0.5)

        # Noise
        x += rng.gauss(0, 0.005)
        y += rng.gauss(0, 0.005)
        z += rng.gauss(0, 0.005)

        vertices.append((x, y, z))

    # Generate faces (triangles) using Delaunay-like connectivity
    # Simple approach: connect each point to its 3 nearest neighbors
    faces = []
    for i in range(min(n - 1, 6000)):
        d1 = (i + 1) % n
        d2 = (i + 2) % n
        d3 = (i + (1 + int(math.sqrt(n)))) % n
        if d1 != d2 and d2 != d3 and d1 != d3:
            faces.append((i, d1, d3))
            if len(faces) >= 6000:
                break

    # Generate colors (height-based)
    colors = []
    for x, y, z in vertices:
        h = (z + 1.5) / 3.0  # normalize to [0,1]
        r = int(max(0, min(255, h * 200 + 55)))
        g = int(max(0, min(255, h * 150 + 80)))
        b = int(max(0, min(255, h * 100 + 100)))
        colors.append((r, g, b))

    # Write PLY
    lines = [
        "ply",
        "format ascii 1.0",
        f"element vertex {len(vertices)}",
        "property float x",
        "property float y",
        "property float z",
        "property uchar red",
        "property uchar green",
        "property uchar blue",
        f"element face {len(faces)}",
        "property list uchar uint vertex_indices",
        "end_header",
    ]
    for (x, y, z), (r, g, b) in zip(vertices, colors):
        lines.append(f"{x:.6f} {y:.6f} {z:.6f} {r} {g} {b}")
    for a, b, c in faces:
        lines.append(f"3 {a} {b} {c}")

    return '\n'.join(lines) + '\n'


# ==========================================================================
# Generate OBJ file (simple cube)
# ==========================================================================

def generate_cube_obj():
    """Generate a simple triangulated cube OBJ."""
    vertices = [
        (-1, -1, -1), (1, -1, -1), (1, 1, -1), (-1, 1, -1),
        (-1, -1, 1), (1, -1, 1), (1, 1, 1), (-1, 1, 1),
    ]
    # Two triangles per face (6 faces * 2 = 12 triangles)
    faces = [
        (0,1,2), (0,2,3),  # front
        (4,6,5), (4,7,6),  # back
        (0,4,5), (0,5,1),  # bottom
        (3,2,6), (3,6,7),  # top
        (0,3,7), (0,7,4),  # left
        (1,5,6), (1,6,2),  # right
    ]
    normals = [
        (0,0,-1), (0,0,1), (0,-1,0), (0,1,0), (-1,0,0), (1,0,0)
    ]

    lines = ["# Simple cube OBJ\n"]
    for x, y, z in vertices:
        lines.append(f"v {x:.6f} {y:.6f} {z:.6f}")
    for nx, ny, nz in normals:
        lines.append(f"vn {nx:.6f} {ny:.6f} {nz:.6f}")
    for i, (a, b, c) in enumerate(faces):
        lines.append(f"f {a+1}//{i//2+1} {b+1}//{i//2+1} {c+1}//{i//2+1}")

    return '\n'.join(lines) + '\n'


# ==========================================================================
# Generate PLY point cloud (non-uniform distribution)
# ==========================================================================

def generate_ply_pointcloud(n=5000, seed=42):
    """Generate a PLY point cloud with non-uniform distribution."""
    rng = random.Random(seed)
    lines = [
        "ply",
        "format ascii 1.0",
        f"element vertex {n}",
        "property float x",
        "property float y",
        "property float z",
        "property uchar red",
        "property uchar green",
        "property uchar blue",
        "end_header",
    ]
    for i in range(n):
        # Non-uniform: denser in center, sparser at edges
        while True:
            x = rng.gauss(0, 1.5)
            y = rng.gauss(0, 1.5)
            if abs(x) < 5 and abs(y) < 5:
                break
        z = math.sin(x * 0.5) * math.cos(y * 0.5) * 2.0 + rng.gauss(0, 0.1)

        # Height-based coloring
        h = (z + 3) / 6.0
        r = int(max(0, min(255, h * 220 + 30)))
        g = int(max(0, min(255, (1 - h) * 200 + 50)))
        b = int(max(0, min(255, h * 80 + 100)))
        lines.append(f"{x:.6f} {y:.6f} {z:.6f} {r} {g} {b}")

    return '\n'.join(lines) + '\n'


# ==========================================================================
# Main
# ==========================================================================

def main():
    print("Generating synthetic test data...")

    # 1. 64x64 terrain GeoTIFF
    print("  terrain_64x64.tif ...", end=" ", flush=True)
    values_64 = generate_terrain(64, 64, seed=42)
    geo_keys_64 = [
        1, 1, 0, 1024,  # GeoKeyDirectoryHeader: 1 key, version 1.0, 1024 keys
        1024, 0, 1, 1,  # GTModelTypeGeoKey: Geographic
        1026, 0, 1, 4326,  # GeographicTypeGeoKey: WGS84
    ]
    tiff_64 = build_tiff_f32(64, 64, values_64, geo_keys_64)
    with open(os.path.join(FIXTURES_DIR, 'terrain_64x64.tif'), 'wb') as f:
        f.write(tiff_64)
    print(f"OK ({len(tiff_64)} bytes, {min(values_64):.0f}-{max(values_64):.0f}m)")

    # 2. 256x256 terrain GeoTIFF
    print("  terrain_256x256.tif ...", end=" ", flush=True)
    values_256 = generate_terrain(256, 256, seed=123)
    tiff_256 = build_tiff_f32(256, 256, values_256, geo_keys_64)
    with open(os.path.join(FIXTURES_DIR, 'terrain_256x256.tif'), 'wb') as f:
        f.write(tiff_256)
    print(f"OK ({len(tiff_256)} bytes, {min(values_256):.0f}-{max(values_256):.0f}m)")

    # 3. PLY mesh (bunny-like)
    print("  bunny_color.ply ...", end=" ", flush=True)
    ply_mesh = generate_ply_mesh(n=3000, seed=42)
    with open(os.path.join(FIXTURES_DIR, 'bunny_color.ply'), 'w') as f:
        f.write(ply_mesh)
    print(f"OK ({len(ply_mesh)} bytes)")

    # 4. PLY point cloud
    print("  pointcloud_5k.ply ...", end=" ", flush=True)
    ply_pc = generate_ply_pointcloud(n=5000, seed=42)
    with open(os.path.join(FIXTURES_DIR, 'pointcloud_5k.ply'), 'w') as f:
        f.write(ply_pc)
    print(f"OK ({len(ply_pc)} bytes)")

    # 5. OBJ cube
    print("  cube.obj ...", end=" ", flush=True)
    obj = generate_cube_obj()
    with open(os.path.join(FIXTURES_DIR, 'cube.obj'), 'w') as f:
        f.write(obj)
    print(f"OK ({len(obj)} bytes)")

    print("\nAll synthetic test data generated.")

if __name__ == '__main__':
    main()
