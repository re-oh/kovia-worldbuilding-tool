use iced::widget::{Column, button, column, container, image, row, scrollable, space, text};
use iced::{ContentFit, Element, Fill, alignment};

use crate::data::{NOTES, OPEN_QUESTION, RELATED_ENTITIES, Status, TIMELINE};
use crate::icons;
use crate::theme::{self, MUTED, PHOSPHOR, PRIMARY, TEXT};
use crate::{Kovia, Message};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    Context,
    Map,
    Sources,
    Timeline,
}

pub fn view(state: &Kovia) -> Element<'_, Message> {
    let tabs = [
        (InspectorTab::Context, "Context"),
        (InspectorTab::Map, "Map"),
        (InspectorTab::Sources, "Sources"),
        (InspectorTab::Timeline, "Timeline"),
    ]
    .into_iter()
    .fold(row![].spacing(2), |tabs, (tab, label)| {
        let active = state.inspector_tab == tab;
        tabs.push(
            button(text(label).size(10))
                .height(38)
                .padding([7, 8])
                .style(move |theme, status| theme::tab_button(active, theme, status))
                .on_press(Message::SelectInspector(tab)),
        )
    });

    let content = match state.inspector_tab {
        InspectorTab::Context => context_tab(state),
        InspectorTab::Map => map_tab(state),
        InspectorTab::Sources => sources_tab(),
        InspectorTab::Timeline => timeline_tab(),
    };

    container(column![container(tabs).padding([2, 7]), content])
        .width(354)
        .height(Fill)
        .style(theme::panel)
        .into()
}

fn context_tab(state: &Kovia) -> Element<'_, Message> {
    let turn = if state.inspected_turn == 0 {
        section(
            "Selected turn",
            column![
                text("Question").size(10),
                text("No attachments or tool calls").size(9).color(MUTED),
            ]
            .spacing(4)
            .into(),
        )
    } else {
        section(
            "Selected turn",
            column![
                text("Answer synthesis").size(10),
                text("4 source notes · timeline comparison · contradiction check")
                    .size(9)
                    .color(MUTED),
            ]
            .spacing(4)
            .into(),
        )
    };

    let map = map_image(198);
    let selected_region = section(
        "Selected region",
        column![
            row![
                text("Eastern Rift").size(11).width(Fill),
                status_badge(Status::Unresolved),
            ]
            .align_y(alignment::Vertical::Center),
            map,
            row![
                metric("12", "notes"),
                metric("3", "entities"),
                metric("2", "events"),
            ]
            .spacing(14),
        ]
        .spacing(8)
        .into(),
    );

    let entities =
        RELATED_ENTITIES
            .iter()
            .fold(Column::new().spacing(7), |items, (name, kind, status)| {
                items.push(
                    row![
                        status_dot(*status),
                        text(*name).size(9).width(Fill),
                        text(*kind).size(8).color(MUTED),
                    ]
                    .spacing(7)
                    .align_y(alignment::Vertical::Center),
                )
            });

    let notes = NOTES
        .iter()
        .take(3)
        .fold(Column::new().spacing(10), |items, note| {
            items.push(
                button(
                    column![
                        row![
                            text(icons::FILE_TEXT).font(PHOSPHOR).size(11).color(MUTED),
                            text(note.name).size(9).width(Fill),
                            text(note.rank).size(8).color(if note.rank == "Strong" {
                                PRIMARY
                            } else {
                                MUTED
                            }),
                        ]
                        .spacing(6),
                        text(note.excerpt).size(8).color(MUTED).line_height(1.35),
                    ]
                    .spacing(4),
                )
                .width(Fill)
                .padding(0)
                .style(theme::clear_button)
                .on_press(Message::RunAction("Source note opened")),
            )
        });

    let statuses = [
        Status::Canon,
        Status::Working,
        Status::Unresolved,
        Status::Contradiction,
    ]
    .into_iter()
    .fold(row![].spacing(5), |items, status| {
        items.push(status_badge(status))
    });

    scrollable(
        column![
            turn,
            selected_region,
            section("Related entities", entities.into()),
            section("Retrieved notes", notes.into()),
            section(
                "Open question",
                text(OPEN_QUESTION)
                    .size(9)
                    .line_height(1.4)
                    .color(MUTED)
                    .into(),
            ),
            section("Canon status", statuses.into()),
        ]
        .spacing(0),
    )
    .height(Fill)
    .into()
}

fn map_tab(state: &Kovia) -> Element<'_, Message> {
    let attached = state.context_chips.contains(&"Eastern Rift");
    let action = if attached {
        button(
            row![
                text(icons::CHECK).font(PHOSPHOR).size(12),
                text("In chat context").size(9),
            ]
            .spacing(6),
        )
        .width(Fill)
        .padding([7, 9])
        .style(theme::bordered_button)
    } else {
        button(
            row![
                text(icons::PLUS).font(PHOSPHOR).size(12),
                text("Add selection to chat context").size(9),
            ]
            .spacing(6),
        )
        .width(Fill)
        .padding([7, 9])
        .style(theme::primary_button)
        .on_press(Message::AddContext("Eastern Rift"))
    };

    scrollable(
        column![
            row![
                button(
                    row![
                        text(icons::MAP_PIN).font(PHOSPHOR).size(11),
                        text("Regions").size(9),
                    ]
                    .spacing(5),
                )
                .padding([6, 8])
                .style(theme::bordered_button),
                button(text("Draw area").size(9))
                    .padding([6, 8])
                    .style(theme::bordered_button)
                    .on_press(Message::RunAction("Area selection mode enabled")),
            ]
            .spacing(5),
            map_image(430),
            container(
                column![
                    text("Eastern Rift").size(11),
                    row![
                        metric("12", "linked notes"),
                        metric("3", "entities"),
                        metric("2", "timeline events"),
                    ]
                    .spacing(12),
                    action,
                ]
                .spacing(9),
            )
            .padding(10)
            .style(theme::raised),
        ]
        .spacing(9)
        .padding(12),
    )
    .height(Fill)
    .into()
}

fn sources_tab<'a>() -> Element<'a, Message> {
    let sources = NOTES.iter().fold(Column::new().spacing(8), |items, note| {
        items.push(
            button(
                container(
                    column![
                        row![
                            text(icons::FILE_TEXT).font(PHOSPHOR).size(11).color(MUTED),
                            text(note.name).size(9).width(Fill),
                            text(note.rank).size(8).color(if note.rank == "Strong" {
                                PRIMARY
                            } else {
                                MUTED
                            }),
                        ]
                        .spacing(6),
                        text(note.path).size(8).color(MUTED),
                        text(note.excerpt).size(9).color(MUTED).line_height(1.35),
                    ]
                    .spacing(5),
                )
                .padding(9)
                .style(theme::raised),
            )
            .width(Fill)
            .padding(0)
            .style(theme::clear_button)
            .on_press(Message::RunAction("Source note opened")),
        )
    });

    scrollable(
        column![
            text("4 sources in this answer").size(9).color(MUTED),
            sources
        ]
        .spacing(9)
        .padding(12),
    )
    .height(Fill)
    .into()
}

fn timeline_tab<'a>() -> Element<'a, Message> {
    let events = TIMELINE
        .iter()
        .fold(Column::new().spacing(0), |items, event| {
            items.push(
                row![
                    status_dot(event.status),
                    text(event.label).size(9).width(Fill),
                    text(event.era).size(8).color(MUTED),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center)
                .padding([8, 2]),
            )
        });

    scrollable(
        column![
            row![
                text("Eastern Regions").size(9).color(MUTED),
                space().width(Fill),
                text("Pre-history → Y. 210").size(8).color(MUTED),
            ],
            events,
            container(
                text("Conflict: ‘First eastward expeditions’ (Y. ~120) overlaps the period Recolonization.md marks the east as unreached.")
                    .size(9)
                    .line_height(1.4),
            )
            .padding(10)
            .style(theme::contradiction),
        ]
        .spacing(9)
        .padding(12),
    )
    .height(Fill)
    .into()
}

fn map_image<'a>(height: u32) -> Element<'a, Message> {
    container(
        image(image::Handle::from_bytes(
            include_bytes!("../assets/kovia-map.png").as_slice(),
        ))
        .width(Fill)
        .height(height)
        .content_fit(ContentFit::Cover),
    )
    .width(Fill)
    .height(height)
    .style(theme::inset)
    .into()
}

fn section<'a>(title: &'static str, content: Element<'a, Message>) -> Element<'a, Message> {
    container(column![text(title).size(9).color(MUTED), content].spacing(8))
        .padding([11, 13])
        .width(Fill)
        .style(|_| {
            iced::widget::container::Style::default().border(iced::Border {
                color: theme::LINE,
                width: 1.0,
                radius: 0.into(),
            })
        })
        .into()
}

fn metric<'a>(value: &'static str, label: &'static str) -> Element<'a, Message> {
    row![
        text(value).size(9).color(TEXT),
        text(label).size(8).color(MUTED),
    ]
    .spacing(4)
    .into()
}

fn status_badge(status: Status) -> Element<'static, Message> {
    let color = status.color();
    container(
        row![
            status_dot(status),
            text(status.label()).size(8).color(color)
        ]
        .spacing(5)
        .align_y(alignment::Vertical::Center),
    )
    .padding([3, 6])
    .style(move |_| {
        iced::widget::container::Style::default()
            .background(iced::Color::from_rgba(color.r, color.g, color.b, 0.08))
            .border(iced::Border {
                color: iced::Color::from_rgba(color.r, color.g, color.b, 0.4),
                width: 1.0,
                radius: 3.into(),
            })
    })
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
