use iced::widget::{Column, button, column, container, row, space, stack, text};
use iced::{Background, Border, Color, Element, Fill, Shadow, alignment};

use crate::Message;
use crate::icons;
use crate::inspector::InspectorTab;
use crate::theme::{self, MUTED, PHOSPHOR, PRIMARY};

pub fn view<'a>() -> Element<'a, Message> {
    let backdrop = button(space().width(Fill).height(Fill))
        .width(Fill)
        .height(Fill)
        .padding(0)
        .style(|_, _| iced::widget::button::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.62))),
            text_color: Color::TRANSPARENT,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: false,
        })
        .on_press(Message::CloseCommandPalette);

    let actions = [
        (
            icons::LINK,
            "Retrieve related context",
            Message::SelectInspector(InspectorTab::Context),
        ),
        (
            icons::GIT_COMPARE,
            "Compare sources",
            Message::SelectInspector(InspectorTab::Sources),
        ),
        (
            icons::WARNING,
            "Find inconsistencies",
            Message::RunAction("Inconsistency review opened"),
        ),
        (
            icons::MAP_TRIFOLD,
            "Open map selection",
            Message::SelectInspector(InspectorTab::Map),
        ),
        (
            icons::BOOK_BOOKMARK,
            "Mark selection as canon",
            Message::RunAction("Marked for canonical review"),
        ),
    ];

    let action_list = actions
        .into_iter()
        .fold(Column::new().spacing(2), |list, item| {
            list.push(command(item.0, item.1, item.2, false))
        });

    let notes = [
        "Eastern Rift.md",
        "Mountain Geology.md",
        "Exploration Timeline.md",
        "Recolonization.md",
    ]
    .into_iter()
    .fold(Column::new().spacing(2), |list, name| {
        list.push(command(
            icons::FILE_TEXT,
            name,
            Message::RunAction("Source note opened"),
            true,
        ))
    });

    let search = row![
        text(icons::MAGNIFYING_GLASS)
            .font(PHOSPHOR)
            .size(14)
            .color(MUTED),
        text("Search notes, entities, or run an action")
            .size(11)
            .color(MUTED)
            .width(Fill),
        button(text("Esc").size(8).color(MUTED))
            .padding([3, 6])
            .style(theme::bordered_button)
            .on_press(Message::CloseCommandPalette),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center)
    .padding([10, 12]);

    let modal = container(column![
        search,
        container(
            column![
                text("ACTIONS").size(8).color(MUTED),
                action_list,
                space().height(7),
                text("NOTES").size(8).color(MUTED),
                notes,
            ]
            .spacing(5),
        )
        .padding([8, 10]),
    ])
    .width(560)
    .style(|_| {
        iced::widget::container::Style::default()
            .background(theme::PANEL)
            .border(Border {
                color: theme::LINE,
                width: 1.0,
                radius: 7.into(),
            })
            .shadow(Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.55),
                offset: iced::Vector::new(0.0, 10.0),
                blur_radius: 32.0,
            })
    });

    let placed = container(column![space().height(115), modal, space().height(Fill)])
        .width(Fill)
        .height(Fill)
        .align_x(alignment::Horizontal::Center);

    stack![backdrop, placed].into()
}

fn command<'a>(
    glyph: &'static str,
    label: &'static str,
    message: Message,
    note: bool,
) -> Element<'a, Message> {
    button(
        row![
            text(glyph)
                .font(PHOSPHOR)
                .size(13)
                .color(if note { MUTED } else { PRIMARY }),
            text(label).size(if note { 9 } else { 10 }).width(Fill),
            text(icons::CARET_RIGHT)
                .font(PHOSPHOR)
                .size(10)
                .color(MUTED),
        ]
        .spacing(9)
        .align_y(alignment::Vertical::Center),
    )
    .width(Fill)
    .padding([7, 8])
    .style(theme::clear_button)
    .on_press(message)
    .into()
}
