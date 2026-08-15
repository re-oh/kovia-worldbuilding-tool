# Bevy knowledge for Kovia Atlas

This is the project's living record of Bevy-specific architectural decisions. Update it when the pinned Bevy version changes or when a Bevy feature materially changes an Atlas subsystem.

## Verified baseline

The project targets **Bevy 0.19.1**, released August 13, 2026. It is the newest stable Bevy release as of August 15, 2026. Bevy 0.19 was released June 19, 2026. `bevy_egui` 0.41.1 is the current matching integration and its recommended multi-pass path uses `EguiPrimaryContextPass`.

Primary references:

- [Bevy 0.19 release notes](https://bevy.org/news/bevy-0-19/)
- [Bevy 0.18 to 0.19 migration guide](https://bevy.org/learn/migration-guides/0-18-to-0-19/)
- [Bevy 0.19.1 release](https://github.com/bevyengine/bevy/releases/tag/v0.19.1)
- [Bevy observer example](https://bevy.org/examples/ecs-entity-component-system/observers/)
- [Bevy 0.19.1 observer API](https://docs.rs/bevy/0.19.1/bevy/ecs/prelude/struct.Observer.html)
- [bevy_egui 0.41.1](https://docs.rs/bevy_egui/0.41.1/bevy_egui/)

## Architectural rule: there is no Atlas god-plugin

`main.rs` is the product composition root. It explicitly selects independent capability plugins. There is no `KoviaAtlasPlugin` that owns all state or secretly installs the whole application.

Each capability plugin owns one coherent unit:

| Plugin | Owns | Does not own |
|---|---|---|
| `PlanetPlugin` | Stable planetary reference | Terrain, rendering, editor |
| `TerrainPlugin` | Height tiles and terrain edit observers | Camera, UI, hydrology |
| `LayerPlugin` | Layer definitions, palettes, categorical masks | Terrain, UI, rendering |
| `FeaturePlugin` | Cities and vector feature records | Terrain and labels |
| `HistoryPlugin` | Optional cross-capability undo chronology | Editing tools and UI |
| `ChangeTrackingPlugin` | Derived-product invalidation | Derived computation |
| `HydrologyPlugin` | Hydrology cache and reference backend | River rendering and canonical edits |
| `ErosionPlugin` | Revision-guarded erosion proposals | Canonical terrain mutation |
| `TileResidencyPlugin` | CPU/GPU residency policy | Persistence and rendering |
| `EditorStatePlugin` | Transient tool intent | Domain data |
| `MapCameraPlugin` | Viewport scene and navigation | Painting and terrain rendering |
| `MapInteractionPlugin` | Gesture-to-domain-event adapter | Direct data mutation |
| `TerrainRenderPlugin` | Disposable terrain visualization cache | Authoritative terrain |
| `HydrologyOverlayRenderPlugin` | River overlay | Hydrology computation |
| `AtlasUiPlugin` | egui widgets and event emission | Direct domain mutation |
| `StarterWorldPlugin` | Optional demonstration content | Application architecture |

A vertical slice is the explicit assembly of independent horizontal capabilities into one end-to-end workflow. It is not permission to collapse those capabilities into one vertically integrated module.

## BSN: use it for scene composition, not geographic data

Bevy 0.19 introduced the next-generation scene system and `bsn!` / `bsn_list!`. BSN scenes are composable patches, can express relationships, and resolve component and asset dependencies. Atlas uses BSN for the static viewport ECS scene: camera, lighting, and later editor-only scene furniture.

Do not represent terrain cells, region cells, rainfall samples, or biome samples as BSN/ECS entities. Those remain packed tiled arrays. BSN is a better construction language for entity compositions; it is not a replacement for a spatial database.

Bevy 0.19 does not yet ship a first-party `.bsn` asset loader. Use code-defined BSN scenes now. Do not base project persistence on external `.bsn` files until upstream loading is stable and appropriate.

## Observer events versus buffered messages

Bevy 0.19 observers watch `Event` types through `On<E>`. `World::trigger` runs observers immediately. `Commands::trigger` runs them at the next ECS synchronization point. Nested triggers are recursively evaluated at that synchronization point.

Atlas policy:

- Use observer `Event`s for sparse semantic commands and lifecycle reactions: sculpt stamp, create layer, place settlement, terrain changed, undo, recompute request.
- Use buffered `Message`s only for genuine streams or queues where batching and independent reader cursors are meaningful: input streams, background job completion streams, telemetry, or bulk import progress.
- Never assume ordering between multiple observers of the same event; Bevy documents that ordering as arbitrary. If order matters, trigger a second explicit event or place ordinary systems in ordered sets.
- Domain plugins own their request observers. UI, keyboard shortcuts, scripts, tests, and future MCP tools all trigger the same public events.

This replaces the old central `.chain()` of camera, painting, mesh synchronization, river drawing, and UI systems.

### Deferred commands for generic cross-cutting work

Bevy 0.19's `Command` trait has an associated `Out` type and is queued with `Commands::queue`. Atlas uses a deferred custom command for undo/redo because applying an arbitrary domain-owned undo action requires controlled `&mut World` access. The history service stores `Box<dyn UndoAction>` and imports no terrain, layer, or feature type. Each domain implements its own undo action beside the data and events it owns.

This is preferable to a central `HistoryEntry` enum: adding a new editable plugin no longer requires modifying `history.rs`, and history never reaches into a list of known domain stores.

## Resources and ownership

Bevy 0.19 stores resources internally as components on singleton entities, enabling hooks, observers, relationships, and resource queries. Atlas still uses resources where cardinality is naturally one, but it uses **several narrow resources**, not one `AtlasWorld` resource.

Examples:

- `TerrainStore`
- `LayerRegistry`
- `LayerStore`
- `FeatureStore`
- `EditHistory`
- `DirtyTracker`
- `HydrologyCache`
- `TileResidencyManager`

The rule is one owner per canonical data class. Render entities and meshes are caches and may be destroyed at any time.

## Rendering implications in Bevy 0.19

Bevy 0.19 moved more rendering preparation to the GPU, improved batching, and reduced CPU work for large scenes. That helps Atlas, but it does not justify one entity per cell. Atlas should still render a bounded set of tile entities and keep cell data in textures or buffers.

The old render graph node model has been replaced by ECS schedules in the render world. When Atlas adds custom height displacement, layer sampling, GPU erosion, or compute-to-render synchronization, implement render work as systems in schedules such as `Core3d`, not custom legacy render-graph nodes.

The current CPU mesh rebuild is only a reference visualization. The scalable path remains:

1. Reusable grid meshes.
2. Per-tile height textures or storage buffers.
3. GPU displacement and normal derivation.
4. Region and biome masks sampled in the terrain material.
5. Camera-driven tile residency and LOD transitions.

## Editor features worth adopting

Bevy 0.19's `TransformGizmoPlugin` is deliberately decoupled from input. It is a good future fit for moving cities, path control points, label anchors, and imported stamps. It is not a terrain sculpting tool.

`InfiniteGridPlugin` is useful for empty-space orientation, projection debugging, and model-editing modes. It should remain optional because the actual terrain surface is the map and weak laptops should not pay for editor decoration they do not need.

The built-in diagnostics overlay can expose FPS, mesh, and material diagnostics without custom egui plumbing. Add it behind a developer/performance toggle.

Text gizmos are useful for ASCII debug labels such as tile IDs and LOD levels. They are not suitable for production Kovia place names or non-ASCII map typography.

## Persistence and settings

Bevy 0.19 adds runtime asset saving through `save_using_saver`. This is relevant to derived artifacts, imported assets, generated meshes, and cached map products. Canonical Atlas project storage still needs a versioned chunk format, atomic save policy, and crash recovery; it should not become a monolithic scene save.

Bevy's new app-settings framework is appropriate for panel layouts, brush defaults, graphics budgets, window state, and tool preferences. Those settings must remain separate from the authored Kovia world.

## Performance notes

Bevy 0.19 adds contiguous query access for dense ECS tables, but Atlas cell simulation is not an ECS query workload. Hydraulic erosion, hydrology, moisture, and biome computation should operate over packed slices directly, where cache behavior and SIMD are explicit.

The weak-laptop rules remain:

- no ECS entity per cell;
- explicit CPU and GPU tile budgets;
- no automatic world-scale recomputation during brush strokes;
- bounded compute windows with halo cells;
- render only resident tiles;
- disable expensive shadows and decorative editor effects by default;
- keep canonical data separate from derived caches.

## Questions to revisit at each Bevy upgrade

1. Is code-defined BSN still the correct stable scene path, and is a first-party `.bsn` loader now available?
2. Have observer ordering or command flush semantics changed?
3. Has the render-world schedule API changed for custom GPU compute and terrain passes?
4. Is there a stable upstream terrain, virtual-texturing, or clipmap primitive worth adopting?
5. Can runtime asset saving safely support Atlas chunk outputs, or should it remain limited to derived assets?
6. Have Bevy's editor gizmos, diagnostics, or settings APIs become stable enough to replace local equivalents?
