use iced::Color;

use crate::theme::{CANON, CONTRADICTION, UNRESOLVED, WORKING};

#[derive(Debug, Clone, Copy)]
pub enum Status {
    Canon,
    Working,
    Unresolved,
    Contradiction,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::Canon => "Canon",
            Self::Working => "Working",
            Self::Unresolved => "Unresolved",
            Self::Contradiction => "Contradiction",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Self::Canon => CANON,
            Self::Working => WORKING,
            Self::Unresolved => UNRESOLVED,
            Self::Contradiction => CONTRADICTION,
        }
    }
}

pub struct Note {
    pub name: &'static str,
    pub path: &'static str,
    pub excerpt: &'static str,
    pub rank: &'static str,
    pub status: Status,
}

pub const NOTES: &[Note] = &[
    Note {
        name: "Eastern Rift.md",
        path: "Geography / Eastern Rift",
        excerpt: "Historically named the end of the world; uninhabited for centuries.",
        rank: "Strong",
        status: Status::Unresolved,
    },
    Note {
        name: "Mountain Geology.md",
        path: "Geography / Mountain Regions",
        excerpt: "The wall traces an ancient rift; sparse vegetation on the far slope.",
        rank: "Strong",
        status: Status::Canon,
    },
    Note {
        name: "Exploration Timeline.md",
        path: "History",
        excerpt: "Eastward expeditions resume only well after recolonization.",
        rank: "Related",
        status: Status::Working,
    },
    Note {
        name: "Recolonization.md",
        path: "History",
        excerpt: "Settlement pushes inland from the western coast first.",
        rank: "Related",
        status: Status::Canon,
    },
];

pub struct TimelineEvent {
    pub label: &'static str,
    pub era: &'static str,
    pub status: Status,
}

pub const TIMELINE: &[TimelineEvent] = &[
    TimelineEvent {
        label: "Ancient rift forms",
        era: "Pre-history",
        status: Status::Canon,
    },
    TimelineEvent {
        label: "Regarded as world's edge",
        era: "Early records",
        status: Status::Canon,
    },
    TimelineEvent {
        label: "The Long Vacancy",
        era: "centuries",
        status: Status::Working,
    },
    TimelineEvent {
        label: "Recolonization begins",
        era: "Y. 0",
        status: Status::Canon,
    },
    TimelineEvent {
        label: "First eastward expeditions",
        era: "Y. ~120?",
        status: Status::Unresolved,
    },
    TimelineEvent {
        label: "World confirmed spherical",
        era: "Y. 210",
        status: Status::Canon,
    },
];

pub const RELATED_ENTITIES: &[(&str, &str, Status)] = &[
    ("Great Eastern Mountain Wall", "Landmark", Status::Canon),
    ("Central Continent", "Region", Status::Canon),
    ("Clifflands", "Region", Status::Working),
];

pub const OPEN_QUESTION: &str =
    "Exactly when did regular eastward exploration begin relative to recolonization?";
