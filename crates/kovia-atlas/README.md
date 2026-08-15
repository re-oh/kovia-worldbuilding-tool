# Kovia Atlas

Kovia Atlas is a paintable, Google-Earth-like world authoring tool built with Bevy 0.19.1, egui, and Phosphor Icons. Geographic cells live in sparse packed tiles; Bevy ECS composes capabilities, transient editor state, and render caches.

## Current workflow assembly

The current executable assembles enough independent capabilities to complete one end-to-end authoring workflow:

1. Navigate a rendered terrain tile.
2. Sculpt packed elevation samples.
3. Create categorical layers and named regions.
4. Paint region IDs independently on each layer.
5. Place, render, select, update, and delete structured settlement features.
6. Produce revision-guarded erosion changes.
7. Compute D8 hydrology and display river candidates.
8. Undo and redo compact terrain, region, and feature patches.
9. Save and load the authored world through a versioned, validated project file.

This is a vertical slice in the product sense: an end-to-end path assembled from separate horizontal capabilities. It is not a single vertically integrated implementation.

## Composition

There is no `KoviaAtlasPlugin` and no `AtlasWorld` god-resource. `main.rs` explicitly adds each capability plugin. Terrain, layers, and features own separate resources and expose observer events as their public edit ports. UI and viewport input trigger those events; they do not mutate domain stores.

The static viewport scene uses Bevy 0.19's `bsn_list!`. Geographic cells remain packed arrays rather than BSN or ECS entities.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the module contracts and [bevy_knowledge.md](bevy_knowledge.md) for the version-specific Bevy decisions.

## Run

Install stable Rust, then:

```bash
cargo run
```

Controls:

- right-drag: orbit;
- mouse wheel: zoom;
- left-drag with Terrain selected: sculpt;
- left-drag with Regions selected: assign cells;
- left-click with Cities selected: add a settlement feature.
- click an existing settlement marker or list entry: select it for editing;
- Save / Load: atomically persist or restore the path shown in the project bar.

The default project path is `kovia-atlas.kovia.json`. Project files contain the
planet specification and authoritative terrain, layer, mask, and feature data.
Loading validates the complete snapshot before replacing the live world and
clears undo history and disposable derived caches. Loading over unsaved edits
requires an explicit discard confirmation.

Validation:

```bash
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```

The boundary tests construct `TerrainPlugin` and `LayerPlugin` independently, without the editor, renderer, or a root Atlas plugin.

## Performance rules

1. A terrain or semantic cell is packed data, never a Bevy entity.
2. Canonical stores and disposable render/compute caches are distinct.
3. Only resident tiles receive CPU or GPU memory.
4. Edits publish invalidation; expensive derived work is explicit.
5. Computation consumes bounded tile windows and revision-tagged snapshots.
6. UI, scripts, shortcuts, tests, and future external tools use the same domain events.
7. Static entity compositions use BSN; spatial field data does not.
