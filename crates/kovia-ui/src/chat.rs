use iced::widget::{
    Column, button, column, container, row, scrollable, space, text, text_editor, tooltip,
};
use iced::{Color, Element, Fill, Padding, alignment};

use crate::icons;
use crate::inspector::InspectorTab;
use crate::theme::{self, CANON, CONTRADICTION, INFERRED, MUTED, PHOSPHOR, PRIMARY, UNRESOLVED};
use crate::{Kovia, Message};

pub fn view(state: &Kovia) -> Element<'_, Message> {
    let header = row![
        text("Kovia").size(11).color(MUTED),
        text(icons::CARET_RIGHT)
            .font(PHOSPHOR)
            .size(10)
            .color(MUTED),
        text("Eastern Regions").size(12),
        space().width(Fill),
        container(text("Session · Eastern wall review").size(9))
            .padding([5, 8])
            .style(theme::raised),
    ]
    .spacing(7)
    .align_y(alignment::Vertical::Center)
    .padding([8, 14]);

    let conversation = scrollable(
        container(container(conversation_content()).width(820))
            .width(Fill)
            .align_x(alignment::Horizontal::Center)
            .padding([18, 22]),
    )
    .height(Fill);

    let body = row![conversation, message_rail(state)]
        .width(Fill)
        .height(Fill);

    container(column![header, body, composer(state)])
        .width(Fill)
        .height(Fill)
        .style(theme::canvas)
        .into()
}

fn conversation_content<'a>() -> Element<'a, Message> {
    let user = container(
        text("What do we already know about the eastern side of the mountain wall, and are there any contradictions with the early exploration timeline?")
            .size(12)
            .line_height(1.45),
    )
    .width(610)
    .padding([9, 11])
    .style(theme::raised);

    let legend = row![
        legend_item("From notes", PRIMARY),
        legend_item("Inferred", INFERRED),
        legend_item("Contradiction", CONTRADICTION),
        legend_item("Open question", UNRESOLVED),
    ]
    .spacing(16)
    .align_y(alignment::Vertical::Center);

    let supported = container(
        column![
            statement(
                "Historically regarded as the ‘end of the world.’",
                "Eastern Rift.md",
                "Named the end of the world in early records; treated as an impassable edge.",
            ),
            statement(
                "Largely uninhabited for centuries, possibly millennia.",
                "Eastern Rift.md",
                "No permanent settlement is recorded across the surveyed span.",
            ),
            statement(
                "Sparse vegetation, especially on the far slope.",
                "Mountain Geology.md",
                "Thin soils and wind exposure keep the eastern slope near-barren.",
            ),
            statement(
                "The chain follows an ancient geological rift.",
                "Mountain Geology.md",
                "The wall traces a deep rift; uplift predates recorded history.",
            ),
            statement(
                "Later expeditions established that the world was spherical.",
                "Exploration Timeline",
                "Circumnavigation-era records overturn the flat-edge assumption.",
            ),
        ]
        .spacing(8),
    )
    .padding([9, 11])
    .style(theme::primary_tint);

    let inferred = container(
        column![
            text("INFERRED CONNECTION").size(9).color(INFERRED),
            text("Settlement pushed inland from the western coast first, so the eastern slope was probably among the last regions surveyed. This connection is not stated directly in the notes.")
                .size(11)
                .color(MUTED)
                .line_height(1.45),
        ]
        .spacing(5),
    )
    .padding([8, 11])
    .style(theme::inferred);

    let contradiction = container(
        column![
            row![
                text(icons::WARNING)
                    .font(PHOSPHOR)
                    .size(13)
                    .color(CONTRADICTION),
                text("Possible contradiction").size(11).color(CONTRADICTION),
            ]
            .spacing(7),
            text("Exploration Timeline places the first eastward expeditions around Y. 120, but Recolonization.md implies the east was reached several generations later. The accounts do not line up; resolve this before treating either date as canon.")
                .size(11)
                .line_height(1.45),
            row![
                action_button(
                    "Open both notes",
                    Message::SelectInspector(InspectorTab::Sources),
                ),
                action_button(
                    "Compare timeline",
                    Message::SelectInspector(InspectorTab::Timeline),
                ),
            ]
            .spacing(7),
        ]
        .spacing(8),
    )
    .padding(10)
    .style(theme::contradiction);

    let question = container(
        column![
            text("OPEN QUESTION").size(9).color(UNRESOLVED),
            text("Exactly when did regular eastward exploration begin relative to recolonization? No note settles this yet.")
                .size(11)
                .color(MUTED)
                .line_height(1.4),
        ]
        .spacing(5),
    )
    .padding([8, 11])
    .style(theme::unresolved);

    let actions = row![
        icon_action(
            icons::ARTICLE,
            "Open sources",
            Message::SelectInspector(InspectorTab::Sources),
        ),
        icon_action(
            icons::MAP_PIN,
            "Show on map",
            Message::SelectInspector(InspectorTab::Map),
        ),
        icon_action(
            icons::GIT_COMPARE,
            "Compare timeline",
            Message::SelectInspector(InspectorTab::Timeline),
        ),
        icon_action(
            icons::LINK,
            "Related notes",
            Message::SelectInspector(InspectorTab::Context),
        ),
        button(
            row![
                text(icons::PLUS).font(PHOSPHOR).size(11),
                text("Add to context").size(10),
            ]
            .spacing(5),
        )
        .padding([6, 8])
        .style(theme::bordered_button)
        .on_press(Message::AddContext("This answer")),
    ]
    .spacing(5)
    .align_y(alignment::Vertical::Center);

    let decisions = row![
        button(
            row![
                text(icons::BOOK_BOOKMARK)
                    .font(PHOSPHOR)
                    .size(11)
                    .color(CANON),
                text("Mark as canon").size(10).color(CANON),
            ]
            .spacing(5),
        )
        .style(theme::clear_button)
        .on_press(Message::RunAction("Marked for canonical review")),
        button(
            row![
                text(icons::NOTE).font(PHOSPHOR).size(11).color(MUTED),
                text("Create note from conclusion").size(10).color(MUTED),
            ]
            .spacing(5),
        )
        .style(theme::clear_button)
        .on_press(Message::RunAction("Note draft created")),
    ]
    .spacing(12);

    column![
        container(user).width(Fill).align_x(alignment::Horizontal::Right),
        space().height(6),
        legend,
        text("The eastern side of the wall is thinly documented, but a few notes agree on the basics:")
            .size(12)
            .line_height(1.4),
        supported,
        inferred,
        contradiction,
        question,
        actions,
        decisions,
    ]
    .spacing(11)
    .into()
}

fn statement<'a>(
    copy: &'static str,
    source: &'static str,
    excerpt: &'static str,
) -> Element<'a, Message> {
    row![
        container(space().width(3).height(18))
            .style(|_| { iced::widget::container::Style::default().background(PRIMARY) }),
        text(copy).size(11).width(Fill),
        citation(source, excerpt),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center)
    .into()
}

fn citation<'a>(source: &'static str, excerpt: &'static str) -> Element<'a, Message> {
    let trigger = button(
        row![
            text(icons::FILE_TEXT).font(PHOSPHOR).size(10).color(MUTED),
            text(source).size(9).color(MUTED),
        ]
        .spacing(4),
    )
    .padding([3, 5])
    .style(theme::bordered_button)
    .on_press(Message::RunAction("Source note opened"));

    let preview = container(
        column![
            text(source).size(10).color(PRIMARY),
            text(excerpt).size(10).color(MUTED).line_height(1.35),
        ]
        .spacing(5),
    )
    .width(250)
    .padding(9)
    .style(theme::tooltip);

    tooltip(trigger, preview, tooltip::Position::Top)
        .gap(6)
        .into()
}

fn legend_item<'a>(label: &'static str, color: Color) -> Element<'a, Message> {
    row![
        container(space().width(7).height(7)).style(move |_| {
            iced::widget::container::Style::default()
                .background(color)
                .border(iced::Border {
                    radius: 7.into(),
                    ..iced::Border::default()
                })
        }),
        text(label).size(9).color(MUTED),
    ]
    .spacing(5)
    .align_y(alignment::Vertical::Center)
    .into()
}

fn action_button<'a>(label: &'static str, message: Message) -> Element<'a, Message> {
    button(text(label).size(10))
        .padding([5, 7])
        .style(theme::bordered_button)
        .on_press(message)
        .into()
}

fn icon_action<'a>(
    glyph: &'static str,
    label: &'static str,
    message: Message,
) -> Element<'a, Message> {
    button(
        row![
            text(glyph).font(PHOSPHOR).size(11).color(MUTED),
            text(label).size(10).color(MUTED),
        ]
        .spacing(5),
    )
    .padding([6, 7])
    .style(theme::bordered_button)
    .on_press(message)
    .into()
}

fn composer(state: &Kovia) -> Element<'_, Message> {
    let chips = state.context_chips.iter().enumerate().fold(
        row![text("Context").size(8).color(MUTED)].spacing(6),
        |chips, (index, label)| {
            chips.push(
                container(
                    row![
                        text(icons::LINK).font(PHOSPHOR).size(9).color(PRIMARY),
                        text(*label).size(8).color(PRIMARY),
                        button(text(icons::X).font(PHOSPHOR).size(9).color(PRIMARY))
                            .padding(1)
                            .style(theme::clear_button)
                            .on_press(Message::RemoveContext(index)),
                    ]
                    .spacing(4)
                    .align_y(alignment::Vertical::Center),
                )
                .padding([3, 5])
                .style(theme::primary_tint),
            )
        },
    );

    let editor = text_editor(&state.editor)
        .placeholder("Ask about Kovia — reference notes, regions, entities, or a date range")
        .on_action(Message::Editor)
        .height(105)
        .size(11)
        .padding(10)
        .style(theme::editor);

    let tools = row![
        composer_tool(icons::PAPERCLIP, "Attach note", "Note picker opened"),
        composer_tool(icons::MAP_TRIFOLD, "Map selection", "Map selection opened"),
        composer_tool(icons::CUBE, "Add entity", "Entity picker opened"),
        composer_tool(icons::STACK, "Select region", "Region picker opened"),
        composer_tool(icons::CALENDAR, "Date range", "Date range picker opened"),
        button(
            text(if state.recording {
                icons::MICROPHONE_SLASH
            } else {
                icons::MICROPHONE
            })
            .font(PHOSPHOR)
            .size(13)
            .color(if state.recording {
                CONTRADICTION
            } else {
                MUTED
            }),
        )
        .padding([5, 7])
        .style(theme::clear_button)
        .on_press(Message::ToggleRecording),
        space().width(Fill),
        button(text(icons::PAPER_PLANE_RIGHT).font(PHOSPHOR).size(14),)
            .padding([7, 10])
            .style(theme::primary_button)
            .on_press(Message::Send),
    ]
    .spacing(3)
    .align_y(alignment::Vertical::Center);

    let notice: Element<'_, Message> = state.notice.as_deref().map_or_else(
        || space().height(0).into(),
        |notice| text(notice).size(8).color(MUTED).into(),
    );

    container(container(column![chips, editor, tools, notice].spacing(6)).width(860))
        .width(Fill)
        .align_x(alignment::Horizontal::Center)
        .padding(Padding {
            top: 8.0,
            right: 16.0,
            bottom: 10.0,
            left: 16.0,
        })
        .style(theme::panel)
        .into()
}

fn composer_tool<'a>(
    glyph: &'static str,
    label: &'static str,
    action: &'static str,
) -> Element<'a, Message> {
    let trigger = button(text(glyph).font(PHOSPHOR).size(13).color(MUTED))
        .padding([5, 7])
        .style(theme::clear_button)
        .on_press(Message::RunAction(action));
    tooltip(
        trigger,
        container(text(label).size(8))
            .padding([4, 6])
            .style(theme::tooltip),
        tooltip::Position::Top,
    )
    .gap(5)
    .into()
}

fn message_rail(state: &Kovia) -> Element<'_, Message> {
    let turns = [
        (PRIMARY, "01 · You", "Question · no tools · no attachments"),
        (
            INFERRED,
            "02 · Kovia",
            "4 sources · 2 comparisons · 1 contradiction",
        ),
    ];

    let bars = turns.into_iter().enumerate().fold(
        Column::new().spacing(7),
        |items, (index, (color, title, detail))| {
            let active = state.inspected_turn == index;
            let segment =
                container(space().width(if active { 7 } else { 4 }).height(42)).style(move |_| {
                    iced::widget::container::Style::default()
                        .background(color)
                        .border(iced::Border {
                            radius: 3.into(),
                            ..iced::Border::default()
                        })
                });
            let trigger = button(
                container(segment)
                    .width(22)
                    .align_x(alignment::Horizontal::Center),
            )
            .width(24)
            .height(46)
            .padding(2)
            .style(theme::clear_button)
            .on_press(Message::InspectTurn(index));
            let info = container(
                column![
                    text(title).size(9).color(color),
                    text(detail).size(8).color(MUTED),
                ]
                .spacing(4),
            )
            .width(230)
            .padding(9)
            .style(theme::tooltip);
            items.push(tooltip(trigger, info, tooltip::Position::Left).gap(7))
        },
    );

    container(column![text("MSG").size(7).color(MUTED), bars].spacing(8))
        .width(38)
        .height(Fill)
        .padding([12, 6])
        .style(theme::panel)
        .into()
}
