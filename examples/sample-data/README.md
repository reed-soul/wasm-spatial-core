# Sample Point Cloud Data for Testing

This directory is a placeholder for real-world LAS/LAZ sample data.

## Where to Get Sample Data

### ASPRS Official Samples
The American Society for Photogrammetry and Remote Sensing provides free LAS sample files:
- <https://www.asprs.org/divisions-committees/lidar-division/laser-las-file-format-exchange-activities>

### Open Topography
Free high-resolution LiDAR point cloud datasets:
- <https://opentopography.org/>
- Various formats available including LAS/LAZ

### Potree Test Data
The Potree project hosts sample point clouds used for testing:
- <https://github.com/mrdoob/three.js> (small examples)
- <https://github.com/potree/potree> (test data in `pointclouds/`)

### libLAS Sample Data
Legacy but still useful for testing:
- <https://github.com/libLAS/libLAS/tree/master/test/data>

### Synthetic Data
For quick testing, use the built-in synthetic point cloud generator in the test suite:
```rust
// tests/point_cloud_pipeline.rs — generate_synthetic_cloud(n, size)
```

## Usage

Once you have a `.las` or `.laz` file, place it in this directory or reference it from tests:

```bash
# After obtaining a sample LAS file
cp ~/Downloads/sample.las examples/sample-data/
```

The Phase A pipeline (`octree` + `pnts` + `tileset`) works with raw `[x, y, z, ...]` float arrays.
LAS parsing support is available via `parse_las_points_core` (feature: `point-cloud`).
