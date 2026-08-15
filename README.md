# Kovia Chat Layer

A desktop-first worldbuilding research workbench built in Rust with Iced. The
current build is a pure UI prototype: it surfaces sources, inferred links,
contradictions, timeline context, and map context without generating new lore.

## Run

```bash
cargo run
```

The current build uses mock conversation data and local UI state only. There is no model or backend integration yet.

## Current UI states

- Evidence-aware chat with inline source previews
- Context, Map, Sources, and Timeline inspector tabs
- Clickable message rail with turn metadata tooltips
- Removable prompt context and transcription control
- Search/command palette with Ctrl+K (Cmd+K on macOS)
- TX-02 interface typography and locally installed Phosphor icons

The V0 reference analysis and adaptation notes are in
[`docs/v0-mock-port.md`](docs/v0-mock-port.md).
