# Kovia Worldbuilding Tool

Kovia Worldbuilding Tool is a single-process desktop editor that combines the
Iced chat/research workspace with the Bevy Atlas renderer and world model. The
application shell owns one window, one `wgpu` device/queue, and one offscreen
map texture. Bevy renders the Atlas into that texture and Iced composites it
into the Map workspace without IPC or a GPU-to-CPU copy.

## Run

The current target is Linux with Vulkan and either Wayland or X11 available.
Rust is pinned by `rust-toolchain.toml`.

```sh
cargo run -p kovia-worldbuilding-tool
```

Useful validation commands:

```sh
cargo fmt --all --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Workspace

- `apps/kovia-worldbuilding-tool`: the production executable and shared GPU /
  event-loop owner.
- `crates/kovia-ui`: the Iced workspace, chat prototype, Atlas viewport shader,
  and a preserved standalone UI runner.
- `crates/kovia-atlas`: the Bevy terrain, layers, features, persistence,
  offscreen renderer, and an optional standalone Atlas runner.
- `crates/kovia-protocol`: framework-neutral input, command, event, snapshot,
  and stable-ID types crossing the UI/Atlas boundary.

The Map view supports camera navigation, terrain sculpting, region and
settlement tools, selection, undo/redo, and demo-project save/load. The chat
and research interface is currently a high-fidelity local prototype; it is not
yet connected to a vault index or model backend.

The startup world and `Demo settlement` are fixtures. Their visible label is
deliberate: no fixture data is promoted to Kovia canon.

See [the rendering architecture](docs/architecture/shared-device-rendering.md)
and [validation record](docs/validation.md) for implementation details and
current boundaries.
