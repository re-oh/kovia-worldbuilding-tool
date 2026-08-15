use super::knowledge::SearchHit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct Turn {
    pub id: usize,
    pub role: Role,
    pub text: String,
    pub sources: Vec<SearchHit>,
}

impl Turn {
    pub fn user(id: usize, text: impl Into<String>) -> Self {
        Self {
            id,
            role: Role::User,
            text: text.into(),
            sources: Vec::new(),
        }
    }

    pub fn retrieval(id: usize, query: &str, sources: Vec<SearchHit>) -> Self {
        let text = match sources.len() {
            0 => format!(
                "No local note matched \"{query}\". Try a name, place, event, or phrase used in the vault."
            ),
            1 => "One local note matched. Its passage is shown below without adding new lore."
                .to_owned(),
            count => format!(
                "{count} local notes matched. Passages are ordered by textual match and source authority; no canon decision has been made."
            ),
        };

        Self {
            id,
            role: Role::Assistant,
            text,
            sources,
        }
    }
}
