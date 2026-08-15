use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonState {
    Canon,
    Working,
    Unresolved,
    Contradiction,
    Archive,
}

impl CanonState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Canon => "Canon",
            Self::Working => "Working",
            Self::Unresolved => "Unresolved",
            Self::Contradiction => "Contradiction",
            Self::Archive => "Archive",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceAuthority {
    Archive,
    AssistantReconstruction,
    AssistantSynthesis,
    StructuredVault,
    RecoveredUserContext,
    UserStated,
    UserCorrected,
    CanonSynthesis,
}

impl SourceAuthority {
    pub fn label(self) -> &'static str {
        match self {
            Self::Archive => "Archive",
            Self::AssistantReconstruction => "Reconstruction",
            Self::AssistantSynthesis => "Assistant synthesis",
            Self::StructuredVault => "Structured vault",
            Self::RecoveredUserContext => "Recovered context",
            Self::UserStated => "User-stated",
            Self::UserCorrected => "User-corrected",
            Self::CanonSynthesis => "Canon synthesis",
        }
    }

    pub fn weight(self) -> u32 {
        match self {
            Self::Archive => 0,
            Self::AssistantReconstruction => 1,
            Self::AssistantSynthesis => 2,
            Self::StructuredVault => 3,
            Self::RecoveredUserContext => 4,
            Self::UserStated => 5,
            Self::UserCorrected => 6,
            Self::CanonSynthesis => 7,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KnowledgeNote {
    pub id: usize,
    pub title: String,
    pub relative_path: String,
    pub absolute_path: PathBuf,
    pub excerpt: String,
    pub body: String,
    pub tags: Vec<String>,
    pub canon_state: CanonState,
    pub authority: SourceAuthority,
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub note_id: usize,
    pub score: u32,
    pub snippet: String,
}

#[derive(Debug, Clone)]
pub struct VaultIndex {
    pub root: PathBuf,
    pub notes: Vec<KnowledgeNote>,
}

impl VaultIndex {
    pub fn note(&self, id: usize) -> Option<&KnowledgeNote> {
        self.notes.get(id).filter(|note| note.id == id)
    }

    pub fn search(&self, query: &str, context: &[String], limit: usize) -> Vec<SearchHit> {
        let mut terms = meaningful_terms(query);
        for chip in context {
            terms.extend(meaningful_terms(chip));
        }
        terms.sort();
        terms.dedup();

        if terms.is_empty() {
            return Vec::new();
        }

        let mut hits = self
            .notes
            .iter()
            .filter_map(|note| {
                let title = note.title.to_lowercase();
                let path = note.relative_path.to_lowercase();
                let tags = note.tags.join(" ").to_lowercase();
                let body = note.body.to_lowercase();

                let lexical_score = terms.iter().fold(0_u32, |score, term| {
                    score
                        + title.matches(term).count() as u32 * 24
                        + path.matches(term).count() as u32 * 10
                        + tags.matches(term).count() as u32 * 8
                        + body.matches(term).count().min(12) as u32 * 2
                });

                if lexical_score == 0 {
                    return None;
                }

                Some(SearchHit {
                    note_id: note.id,
                    score: lexical_score + note.authority.weight(),
                    snippet: best_snippet(note, &terms),
                })
            })
            .collect::<Vec<_>>();

        hits.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| {
                    let left_note = &self.notes[left.note_id];
                    let right_note = &self.notes[right.note_id];
                    right_note.authority.cmp(&left_note.authority)
                })
                .then_with(|| {
                    self.notes[left.note_id]
                        .title
                        .cmp(&self.notes[right.note_id].title)
                })
        });
        hits.truncate(limit);
        hits
    }
}

fn meaningful_terms(text: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "about", "already", "also", "and", "any", "are", "does", "from", "have", "into",
        "know", "more", "that", "the", "their", "there", "these", "this", "what", "when",
        "where", "which", "with", "would",
    ];

    text.split(|character: char| !character.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|term| term.len() >= 4 && !STOP_WORDS.contains(&term.as_str()))
        .collect()
}

fn best_snippet(note: &KnowledgeNote, terms: &[String]) -> String {
    let best = note
        .body
        .lines()
        .filter_map(|line| {
            let clean = clean_markdown(line);
            if clean.len() < 24 {
                return None;
            }

            let lower = clean.to_lowercase();
            let matches = terms
                .iter()
                .filter(|term| lower.contains(term.as_str()))
                .count();
            (matches > 0).then_some((matches, clean))
        })
        .max_by_key(|(matches, _)| *matches)
        .map(|(_, line)| line)
        .unwrap_or_else(|| note.excerpt.clone());

    truncate(&best, 260)
}

pub fn clean_markdown(line: &str) -> String {
    line.trim()
        .trim_start_matches(['#', '-', '>', '*', ' '])
        .replace("**", "")
        .replace("[[", "")
        .replace("]]", "")
        .replace('`', "")
}

pub fn truncate(text: &str, max_chars: usize) -> String {
    let mut value = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        value.push('…');
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_prefers_a_title_match() {
        let index = VaultIndex {
            root: PathBuf::from("/vault"),
            notes: vec![
                note(0, "Other", "The eastern wall is mentioned once."),
                note(1, "Eastern Mountain Wall", "A focused source."),
            ],
        };

        let hits = index.search("eastern mountain wall", &[], 5);
        assert_eq!(hits[0].note_id, 1);
    }

    fn note(id: usize, title: &str, body: &str) -> KnowledgeNote {
        KnowledgeNote {
            id,
            title: title.to_owned(),
            relative_path: format!("{title}.md"),
            absolute_path: PathBuf::from(format!("/vault/{title}.md")),
            excerpt: body.to_owned(),
            body: body.to_owned(),
            tags: Vec::new(),
            canon_state: CanonState::Archive,
            authority: SourceAuthority::Archive,
        }
    }
}
