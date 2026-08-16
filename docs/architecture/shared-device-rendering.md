# Shared-device rendering

## Decision

The editor is one executable and one top-level event loop. The application
shell owns the Winit window and the complete `wgpu` lifetime:

```text
Winit / application shell
  ├── wgpu instance, adapter, device, queue, surface
  ├── Bevy App (manual update and render)
  │     └── writes the shell-owned offscreen texture
  └── Iced runtime and renderer
        └── samples that texture in a shader widget, then presents
```

Iced owns the editor interaction model. Bevy remains the authoritative map
domain and renderer, but it does not create a window or run its own runner.
The shell calls `App::update`, renders the Atlas, then composites Iced using the
same queue. Resize creates one replacement texture view and supplies it to both
consumers. Shutdown performs one bounded device poll after both render paths
have stopped producing work.

This avoids DMA-BUF, IPC, cross-device texture import, and frame ownership
protocols. The viewport never makes a CPU copy of the rendered map.

## Dependency seam

The integrated target uses:

- Bevy `0.19.1` without Bevy's Winit feature.
- Iced pinned to revision
  `2cffa99b395d84fe469b44dccb56bbacd2f1a157`.
- Exactly one `wgpu` and `naga` line, version `29.0.4`.

The Iced revision is intentional. Iced `0.14` resolves to `wgpu 27`, whose
resource types cannot be passed to Bevy `0.19`'s `wgpu 29` renderer.

The optional Atlas standalone feature retains Bevy Winit and egui for focused
development. Those dependencies are not enabled by the production executable.

## Boundary

`kovia-protocol` is the only semantic bridge. Iced sends viewport-local
physical pointer input and map commands; Atlas returns events and a read-only
snapshot. Bevy entities and renderer resources do not cross the boundary.

Stable persisted IDs live in the protocol crate. `RegionRef` combines a
`LayerId` and `RegionCode`, preventing a region code from being interpreted
without its owning layer. Existing UUID and tuple serialization shapes are
preserved.

## Input and scheduling

- Iced's runtime remains active, including async tasks, subscriptions, widget
  operations, clipboard actions, IME state, and redraw wakeups.
- The viewport shader translates logical widget coordinates to physical Atlas
  coordinates.
- A raw shell-level mouse-release fallback clears Atlas button state even when
  a widget-local release is lost during capture, focus, or layout changes.
- Atlas rendering precedes Iced compositing on the same queue, so queue order
  makes the new map frame available to the UI without an idle wait.

## Provenance

The repository preserves both source histories as subtrees:

- Chat/UI baseline: `9bb1a7b8bf755bb34dea61b46320d619f60b58ba`.
- Atlas baseline: `df29551fceb8a049ab5b7298737d93f1b8ad4480`.

The shared-device design was validated separately in the local integration
spike at `c6e735a` before being implemented here. The spike itself is not a
third application merged into this repository.
