use iced::widget::{Column, button, column, container, row, scrollable, space, text};
use iced::{Element, Fill, alignment};

use crate::data::{NOTES, Status};
use crate::icons;
use crate::theme::{self, MUTED, PHOSPHOR, PRIMARY};
use crate::{Kovia, Message};

const NAV: &[(&str, &str)] = &[
    (icons::MAGNIFYING_GLASS, "Search"),
    (icons::CHAT, "Chat"),
    (icons::MAP_TRIFOLD, "Map"),
    (icons::CLOCK, "Timeline"),
    (icons::CUBE, "Entities"),
    (icons::STACK, "Regions"),
    (icons::FILE_TEXT, "Notes"),
    (icons::BOOK_BOOKMARK, "Canon"),
    (icons::WARNING, "Contradictions"),
];

pub fn view(state: &Kovia) -> Element<'_, Message> {
    let vault = row![
        container(text("K").size(12).color(PRIMARY))
            .width(24)
            .height(24)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .style(theme::primary_tint),
        text("Kovia").size(12),
        space().width(Fill),
        text("Vault").size(9).color(MUTED),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center)
    .padding([9, 10]);

    let nav = NAV.iter().enumerate().fold(
        Column::new().spacing(2),
        |items, (index, (glyph, label))| {
            let active = state.selected_nav == index;
            let mut content = row![
                text(*glyph)
                    .font(PHOSPHOR)
                    .size(14)
                    .color(if active { PRIMARY } else { MUTED })
                    .width(22),
                text(*label).size(11).width(Fill),
            ]
            .spacing(7)
            .align_y(alignment::Vertical::Center);

            if index == 0 {
                content = content.push(text("Ctrl+K").size(8).color(MUTED));
            } else if index == 8 {
                content = content.push(
                    container(text("2").size(8).color(theme::CONTRADICTION))
                        .padding([2, 5])
                        .style(theme::contradiction),
                );
            }

            items.push(
                button(content)
                    .width(Fill)
                    .padding([6, 8])
                    .style(move |theme, status| theme::nav_button(active, theme, status))
                    .on_press(if index == 0 {
                        Message::ToggleCommandPalette
                    } else {
                        Message::SelectNav(index)
                    }),
            )
        },
    );

    let geography = button(
        row![
            text(if state.geography_open {
                icons::CARET_DOWN
            } else {
                icons::CARET_RIGHT
            })
            .font(PHOSPHOR)
            .size(11)
            .color(MUTED),
            text(icons::FOLDER).font(PHOSPHOR).size(12).color(MUTED),
            text("Geography").size(10),
        ]
        .spacing(7)
        .align_y(alignment::Vertical::Center),
    )
    .width(Fill)
    .padding([5, 5])
    .style(theme::clear_button)
    .on_press(Message::ToggleGeography);

    let mut tree = Column::new().spacing(1).push(geography);
    if state.geography_open {
        tree = tree
            .push(tree_leaf("Mountain Regions", Status::Canon, 18))
            .push(tree_leaf("Clifflands", Status::Working, 18))
            .push(tree_leaf("Eastern Rift", Status::Unresolved, 18));
    }
    for name in [
        "Nations",
        "History",
        "Cultures",
        "Technology",
        "Religion",
        "Characters",
        "Scratchpad",
    ] {
        tree = tree.push(
            button(
                row![
                    text(icons::CARET_RIGHT)
                        .font(PHOSPHOR)
                        .size(10)
                        .color(MUTED),
                    text(name).size(10).color(MUTED),
                ]
                .spacing(7),
            )
            .width(Fill)
            .padding([5, 6])
            .style(theme::clear_button),
        );
    }

    let recent = NOTES.iter().fold(Column::new().spacing(1), |items, note| {
        items.push(
            button(
                row![
                    text(icons::FILE_TEXT).font(PHOSPHOR).size(11).color(MUTED),
                    text(note.name).size(9).color(MUTED).width(Fill),
                    status_dot(note.status),
                ]
                .spacing(6)
                .align_y(alignment::Vertical::Center),
            )
            .width(Fill)
            .padding([5, 6])
            .style(theme::clear_button),
        )
    });

    let content = column![
        vault,
        container(nav).padding([7, 8]),
        container(space().height(1))
            .width(Fill)
            .style(theme::raised),
        scrollable(
            column![
                text("KOVIA").size(8).color(MUTED),
                tree,
                space().height(14),
                text("RECENTLY OPENED").size(8).color(MUTED),
                recent,
            ]
            .spacing(6)
            .padding(10),
        )
        .height(Fill),
    ];

    container(content)
        .width(244)
        .height(Fill)
        .style(theme::panel)
        .into()
}

fn tree_leaf(label: &'static str, status: Status, indent: u32) -> Element<'static, Message> {
    button(
        row![
            space().width(indent),
            text(label).size(10).color(MUTED).width(Fill),
            status_dot(status),
        ]
        .align_y(alignment::Vertical::Center),
    )
    .width(Fill)
    .padding([5, 6])
    .style(theme::clear_button)
    .into()
}

fn status_dot(status: Status) -> Element<'static, Message> {
    container(space().width(7).height(7))
        .style(move |_| {
            iced::widget::container::Style::default()
                .background(status.color())
                .border(iced::Border {
                    radius: 7.into(),
                    ..iced::Border::default()
                })
        })
        .into()
}
