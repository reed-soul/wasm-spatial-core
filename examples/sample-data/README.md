# Sample Point Cloud Data

## Bundled demo file

| File | Points | Size | Usage |
|------|--------|------|-------|
| `demo_terrain.las` | 80,000 | ~2.2 MB | One-click load in demo hub (**Sample LAS** button) |

Generated with (100 m extent for visible density in the hub preview):

```bash
cargo run --example gen_test_las -- 80000 examples/sample-data/demo_terrain.las 100
```

## More sample data

### ASPRS Official Samples
- <https://www.asprs.org/divisions-committees/lidar-division/laser-las-file-format-exchange-activities>

### Open Topography
- <https://opentopography.org/>

### libLAS Test Data
- <https://github.com/libLAS/libLAS/tree/master/test/data>

Place additional `.las` / `.laz` files here and load them from any point-cloud demo via drag-and-drop.
