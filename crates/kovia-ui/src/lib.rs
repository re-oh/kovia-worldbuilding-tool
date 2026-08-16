mod atlas;
mod chat;
mod data;
mod icons;
mod inspector;
mod palette;
mod sidebar;
mod theme;
mod viewport;

use iced::keyboard::{self, Key, key::Named};
use iced::widget::{container, stack, text_editor};
use iced::{Element, Fill, Subscription, Task, Theme};
use iced_wgpu::wgpu;
use kovia_protocol::{MapCommand, MapEvent, MapInput, MapSnapshot};

pub use inspector::InspectorTab;
pub use viewport::MapViewport;

pub const PHOSPHOR_FONT_BYTES: &[u8] = include_bytes!("/usr/share/fonts/TTF/Phosphor.ttf");
pub const TX02_FONT_BYTES: &[u8] = include_bytes!("/usr/share/fonts/TX-02/TX-02-Regular.ttf");

pub fn run_standalone() -> iced::Result {
    iced::application(Kovia::new_standalone, Kovia::update, Kovia::view)
        .title("Kovia — Worldbuilding Workbench")
        .theme(app_theme)
        .subscription(Kovia::subscription)
        .font(PHOSPHOR_FONT_BYTES)
        .font(TX02_FONT_BYTES)
        .default_font(theme::TX02)
        .window_size((1440.0, 900.0))
        .centered()
        .run()
}

fn app_theme(_: &Kovia) -> Theme {
    Theme::custom("Kovia", theme::palette())
}

#[derive(Debug, Clone)]
pub enum Message {
    Editor(text_editor::Action),
    Send,
    SelectNav(usize),
    SelectInspector(InspectorTab),
    ToggleCommandPalette,
    CloseCommandPalette,
    ToggleGeography,
    ToggleRecording,
    RemoveContext(usize),
    AddContext(&'static str),
    InspectTurn(usize),
    RunAction(&'static str),
    Keyboard(keyboard::Event),
    MapInput(MapInput),
    MapCommand(MapCommand),
    RuntimeReady,
}

pub struct Kovia {
    pub editor: text_editor::Content,
    pub selected_nav: usize,
    pub inspector_tab: InspectorTab,
    pub command_open: bool,
    pub geography_open: bool,
    pub recording: bool,
    pub context_chips: Vec<&'static str>,
    pub inspected_turn: usize,
    pub notice: Option<String>,
    pub map_view: Option<MapViewport>,
    pub map_snapshot: MapSnapshot,
}

impl Kovia {
    fn new_standalone() -> Self {
        Self::with_viewport(None)
    }

    pub fn boot(map_view: MapViewport) -> (Self, Task<Message>) {
        let state = Self::with_viewport(Some(map_view));
        let task = Task::perform(
            async {
                std::thread::sleep(std::time::Duration::from_millis(15));
            },
            |_| Message::RuntimeReady,
        );
        (state, task)
    }

    fn with_viewport(map_view: Option<MapViewport>) -> Self {
        Self {
            editor: text_editor::Content::new(),
            selected_nav: 1,
            inspector_tab: InspectorTab::Context,
            command_open: false,
            geography_open: true,
            recording: false,
            context_chips: vec!["Eastern Rift", "Clifflands", "3 notes", "Map selection"],
            inspected_turn: 1,
            notice: None,
            map_view,
            map_snapshot: MapSnapshot::default(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Editor(action) => self.editor.perform(action),
            Message::Send => {
                if !self.editor.text().trim().is_empty() {
                    self.editor = text_editor::Content::new();
                    self.notice = Some("Draft prompt added to the local UI thread".into());
                }
            }
            Message::SelectNav(index) => {
                self.selected_nav = index;
                if index == 2 {
                    self.inspector_tab = InspectorTab::Map;
                }
                self.notice = None;
            }
            Message::SelectInspector(tab) => {
                self.inspector_tab = tab;
                self.command_open = false;
                self.notice = None;
            }
            Message::ToggleCommandPalette => self.command_open = !self.command_open,
            Message::CloseCommandPalette => self.command_open = false,
            Message::ToggleGeography => self.geography_open = !self.geography_open,
            Message::ToggleRecording => {
                self.recording = !self.recording;
                self.notice = None;
            }
            Message::RemoveContext(index) => {
                if index < self.context_chips.len() {
                    self.context_chips.remove(index);
                }
            }
            Message::AddContext(label) => {
                if !self.context_chips.contains(&label) {
                    self.context_chips.push(label);
                }
                self.notice = None;
            }
            Message::InspectTurn(index) => {
                self.inspected_turn = index;
                self.inspector_tab = if index == 0 {
                    InspectorTab::Context
                } else {
                    InspectorTab::Sources
                };
            }
            Message::RunAction(action) => {
                self.notice = Some(action.into());
                self.command_open = false;
            }
            Message::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                repeat,
                ..
            }) if !repeat => match key.as_ref() {
                Key::Character("k") if modifiers.command() => {
                    self.command_open = !self.command_open;
                }
                Key::Named(Named::Escape) if self.command_open => {
                    self.command_open = false;
                }
                _ => {}
            },
            Message::Keyboard(_) => {}
            Message::MapInput(_) | Message::MapCommand(_) => {}
            Message::RuntimeReady => {
                self.notice = Some("Iced task runtime connected".into());
            }
        }

        Task::none()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().map(Message::Keyboard)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let center = if self.selected_nav == 2 {
            atlas::view(self)
        } else {
            chat::view(self)
        };
        let workspace = container(iced::widget::row![
            sidebar::view(self),
            center,
            inspector::view(self),
        ])
        .width(Fill)
        .height(Fill)
        .style(theme::canvas);

        if self.command_open {
            stack![workspace, palette::view()].into()
        } else {
            workspace.into()
        }
    }

    pub fn replace_viewport(
        &mut self,
        texture_view: wgpu::TextureView,
        physical_size: [u32; 2],
        scale_factor: f32,
    ) {
        if let Some(viewport) = self.map_view.as_mut() {
            viewport.replace(texture_view, physical_size, scale_factor);
        } else {
            self.map_view = Some(MapViewport::new(texture_view, physical_size, scale_factor));
        }
    }

    pub fn set_map_snapshot(&mut self, snapshot: MapSnapshot) {
        self.map_snapshot = snapshot;
    }

    pub fn apply_map_event(&mut self, event: &MapEvent) {
        self.notice = Some(match event {
            MapEvent::SelectionChanged(Some(_)) => "Atlas selection updated".into(),
            MapEvent::SelectionChanged(None) => "Atlas selection cleared".into(),
            MapEvent::CameraChanged(_) => "Atlas camera updated".into(),
            MapEvent::CommandAccepted { .. } => "Atlas command accepted".into(),
            MapEvent::CommandRejected { error } => format!("Atlas rejected command: {error}"),
            MapEvent::ProjectChanged { dirty: true } => "Project has unsaved changes".into(),
            MapEvent::ProjectChanged { dirty: false } => "Project is saved".into(),
            MapEvent::ProjectSaved { path } => format!("Saved {path}"),
            MapEvent::ProjectLoaded { path } => format!("Loaded {path}"),
            MapEvent::ProjectIoFailed { operation, error } => {
                format!("Could not {operation} project: {error}")
            }
        });
    }

    pub fn theme(&self) -> Theme {
        app_theme(self)
    }

    pub fn default_font() -> iced::Font {
        theme::TX02
    }
}
