use iced::widget::{button, column, container, row, space, stack, text};
use iced::{Element, Fill, alignment};
use kovia_protocol::{MapCommand, MapTool};

use crate::theme::{self, MUTED, PRIMARY};
use crate::{Kovia, Message};

pub fn view(state: &Kovia) -> Element<'_, Message> {
    let Some(viewport) = state.map_view.as_ref() else {
        return container(text(
            "The live Atlas viewport is available in the combined application.",
        ))
        .width(Fill)
        .height(Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(theme::canvas)
        .into();
    };

    let tool = state.map_snapshot.active_tool;
    let toolbar = row![
        tool_button("Navigate", MapTool::Navigate, tool),
        tool_button("Sculpt", MapTool::Sculpt, tool),
        tool_button("Regions", MapTool::Regions, tool),
        tool_button("Settlement", MapTool::Settlement, tool),
        space().width(Fill),
        button(text("Undo").size(9))
            .padding([7, 9])
            .style(theme::bordered_button)
            .on_press_maybe(
                state
                    .map_snapshot
                    .undo_available
                    .then_some(Message::MapCommand(MapCommand::Undo)),
            ),
        button(text("Redo").size(9))
            .padding([7, 9])
            .style(theme::bordered_button)
            .on_press_maybe(
                state
                    .map_snapshot
                    .redo_available
                    .then_some(Message::MapCommand(MapCommand::Redo)),
            ),
        button(text("Save").size(9))
            .padding([7, 10])
            .style(theme::bordered_button)
            .on_press(Message::MapCommand(MapCommand::SaveProject)),
        button(text("Load").size(9))
            .padding([7, 10])
            .style(theme::bordered_button)
            .on_press(Message::MapCommand(MapCommand::LoadProject)),
    ]
    .spacing(5)
    .align_y(alignment::Vertical::Center);

    let selected = state.map_snapshot.selected_feature.as_ref().map_or_else(
        || "No feature selected".to_owned(),
        |feature| format!("{} · {}", feature.name, feature.kind),
    );
    let status = container(
        row![
            text(if state.map_snapshot.project_dirty {
                "Unsaved"
            } else {
                "Saved"
            })
            .size(9)
            .color(if state.map_snapshot.project_dirty {
                PRIMARY
            } else {
                MUTED
            }),
            text(selected).size(9),
            space().width(Fill),
            text(&state.map_snapshot.status).size(8).color(MUTED),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center),
    )
    .padding([7, 10])
    .style(theme::panel);

    let chrome = column![
        container(toolbar).padding(8).style(theme::panel),
        space().height(Fill),
        status
    ];

    stack![viewport.view(), chrome]
        .width(Fill)
        .height(Fill)
        .into()
}

fn tool_button<'a>(label: &'static str, value: MapTool, active: MapTool) -> Element<'a, Message> {
    button(text(label).size(9))
        .padding([7, 10])
        .style(move |theme, status| theme::tab_button(active == value, theme, status))
        .on_press(Message::MapCommand(MapCommand::SetTool(value)))
        .into()
}
