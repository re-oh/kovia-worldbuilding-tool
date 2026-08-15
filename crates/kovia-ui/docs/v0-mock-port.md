# V0 mock analysis and Iced port plan

Date: 2026-08-15

## Source reviewed

The V0 archive was extracted to `/home/rio/dump/kovia-ui-mock`. Its UI is
already split into readable components: workspace shell, left sidebar, chat,
prompt composer, context inspector, command palette, citations, status badges,
and interactive world map.

Desktop states captured with Grim:

- `mock-default.png`
- `mock-command-palette.png`
- `mock-map-tab.png`
- `mock-sources-tab.png`
- `mock-timeline-tab.png`
- `mock-citation-tooltip.png`

All captures live beside the extracted source. The local browser window and
Next development server were closed after capture.

## State model

- Left and right panel widths are resizable within fixed limits.
- Command palette opens from Search or Ctrl+K, filters actions and notes, and
  supports keyboard navigation.
- Inspector switches between Context, Map, Sources, and Timeline.
- Prompt context is represented by removable typed chips.
- Map supports region selection and drawn-area selection, then adds that
  selection to chat context.
- Citations expose source excerpts on hover.
- Answers distinguish sourced statements, inferred connections,
  contradictions, and unresolved questions.
- Answer actions open evidence tabs, add working context, mark canon, or create
  a note.

## Visual system

- Three-pane workbench with hairline separators and dense controls.
- Near-black slate surfaces; color is reserved for semantic meaning.
- Teal identifies sourced/contextual material, violet identifies inference,
  amber identifies unresolved material, and red identifies contradiction.
- Content uses compact blocks, inline citations, narrow status markers, and
  explicit evidence hierarchy instead of decorative cards.
- TX-02 is the Iced interface font. Phosphor remains the icon font.

## Iced adaptation

The port is not a literal copy. Earlier deletion decisions remain in force:
there is no decorative brand header, generic recent-chat list, profile footer,
system-status badge, or passive session telemetry.

The Iced UI includes:

1. A worldbuilding navigation/file tree without the deleted recent-chat area.
2. A dense evidence-aware answer with source, inference, contradiction, and
   open-question blocks.
3. Removable context chips above the large transcription-capable composer.
4. A functional Context/Map/Sources/Timeline inspector.
5. A command-palette overlay driven by normal Iced state.
6. Citation and message-rail tooltips for source and execution details.
7. The V0 world-map asset in the Map and Context inspector states.

The mock is a UI reference only; the Iced implementation remains local mock
state with no model, vault, or persistence backend.

## Verified native states

The port was built and exercised as a native Iced application at 1440 x 900.
The following interactions were checked in the running binary:

- Ctrl+K opens the command palette and Escape closes it.
- Palette and answer actions switch to Context, Map, Sources, or Timeline.
- Geography expands and collapses.
- Prompt context references can be removed or added.
- The microphone control exposes a distinct recording state.
- Message-rail clicks select a turn and expose its evidence/tool summary.
- Citation and message-rail hover targets provide compact source previews.

`cargo check`, `cargo test`, `cargo fmt --check`, and Clippy with warnings denied
all pass on the final pure-UI build.
