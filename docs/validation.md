# Validation

Validated locally on 2026-08-16 under Sway with a RADV Vulkan adapter.

## Automated

- `cargo check -p kovia-worldbuilding-tool`
- `cargo test --workspace --all-targets` (24 tests across the protocol, UI,
  shell, Atlas library, and Atlas plugin-boundary integration tests)
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Dependency inspection confirms one `wgpu 29.0.4` and one `naga 29.0.4` in
  the production application, with no `bevy_winit` or `bevy_egui` dependency.

## Live application

- Exactly one `Kovia — Worldbuilding Tool` window and one application process.
- The Iced chat/research workspace renders with its bundled TX-02 and Phosphor
  assets.
- Iced's async task executor updates the startup status, and its keyboard
  subscription opens the Ctrl+K command palette.
- The Map navigation item reveals the live Bevy Atlas inside the Iced layout.
- Window resizing rebuilds the single shared target for both consumers.
- A sculpt gesture updates terrain and project revision; releasing the button
  stops revision changes instead of leaving the tool active.
- The initial demo fixture remains Saved until the user edits it.
- Closing the window reaches a clean bounded GPU drain (`QueueEmpty`).

The Atlas startup content is a demo fixture and is visibly identified as such.
This validation does not assert that any fixture text or geography is Kovia
canon.
