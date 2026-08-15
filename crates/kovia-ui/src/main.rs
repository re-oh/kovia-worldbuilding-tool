mod chat;
mod data;
mod icons;
mod inspector;
mod palette;
mod sidebar;
mod theme;

use iced::keyboard::{self, Key, key::Named};
use iced::widget::{container, stack, text_editor};
use iced::{Element, Fill, Subscription, Task, Theme};

use inspector::InspectorTab;

pub fn main() -> iced::Result {
    iced::application(Kovia::new, Kovia::update, Kovia::view)
        .title("Kovia — Worldbuilding Workbench")
        .theme(app_theme)
        .subscription(Kovia::subscription)
        .font(include_bytes!("/usr/share/fonts/TTF/Phosphor.ttf").as_slice())
        .font(include_bytes!("/usr/share/fonts/TX-02/TX-02-Regular.ttf").as_slice())
        .default_font(theme::TX02)
        .window_size((1440.0, 900.0))
        .centered()
        .run()
}

fn app_theme(_: &Kovia) -> Theme {
    Theme::custom("Kovia", theme::palette())
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
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
}

pub(crate) struct Kovia {
    pub editor: text_editor::Content,
    pub selected_nav: usize,
    pub inspector_tab: InspectorTab,
    pub command_open: bool,
    pub geography_open: bool,
    pub recording: bool,
    pub context_chips: Vec<&'static str>,
    pub inspected_turn: usize,
    pub notice: Option<&'static str>,
}

impl Kovia {
    fn new() -> Self {
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
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Editor(action) => self.editor.perform(action),
            Message::Send => {
                if !self.editor.text().trim().is_empty() {
                    self.editor = text_editor::Content::new();
                    self.notice = Some("Draft prompt added to the local UI thread");
                }
            }
            Message::SelectNav(index) => {
                self.selected_nav = index;
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
                self.notice = Some(action);
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
        }

        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().map(Message::Keyboard)
    }

    fn view(&self) -> Element<'_, Message> {
        let workspace = container(iced::widget::row![
            sidebar::view(self),
            chat::view(self),
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
}
