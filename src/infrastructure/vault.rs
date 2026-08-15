use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::domain::knowledge::{
    CanonState, KnowledgeNote, SourceAuthority, VaultIndex, clean_markdown, truncate,
};

pub const DEFAULT_VAULT_PATH: &str =
    "/home/rio/Documents/Kovia_Complete_Obsidian_Archive_2026-08-14";

pub fn configured_root() -> PathBuf {
    env::var_os("KOVIA_VAULT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_VAULT_PATH))
}

pub fn load_configured() -> Result<VaultIndex, String> {
    let root = configured_root();
    load(&root).map_err(|error| format!("Could not index {}: {error}", root.display()))
}

pub fn load(root: &Path) -> io::Result<VaultIndex> {
    let mut paths = Vec::new();
    collect_markdown(root, &mut paths)?;
    paths.sort();

    let notes = paths
        .into_iter()
        .enumerate()
        .filter_map(|(id, path)| parse_note(id, root, path).transpose())
        .collect::<io::Result<Vec<_>>>()?;

    Ok(VaultIndex {
        root: root.to_path_buf(),
        notes,
    })
}

fn collect_markdown(directory: &Path, output: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            collect_markdown(&path, output)?;
        } else if file_type.is_file() && path.extension().is_some_and(|value| value == "md") {
            output.push(path);
        }
    }
    Ok(())
}

fn parse_note(id: usize, root: &Path, path: PathBuf) -> io::Result<Option<KnowledgeNote>> {
    let body = fs::read_to_string(&path)?;
    if body.trim().is_empty() {
        return Ok(None);
    }

    let metadata = Frontmatter::parse(&body);
    let title = body
        .lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_owned)
        .or_else(|| path.file_stem().map(|stem| stem.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "Untitled note".to_owned());

    let excerpt = body
        .lines()
        .map(clean_markdown)
        .find(|line| line.len() >= 32 && !line.starts_with("type:") && !line.starts_with("status:"))
        .map(|line| truncate(&line, 180))
        .unwrap_or_else(|| "No excerpt available.".to_owned());

    let relative_path = path
        .strip_prefix(root)
        .unwrap_or(&path)
        .to_string_lossy()
        .into_owned();
    let authority = authority_for(&relative_path, &body);
    let canon_state = canon_state_for(metadata.status.as_deref(), &relative_path, &body);

    Ok(Some(KnowledgeNote {
        id,
        title,
        relative_path,
        absolute_path: path,
        excerpt,
        body,
        tags: metadata.tags,
        canon_state,
        authority,
    }))
}

fn authority_for(relative_path: &str, body: &str) -> SourceAuthority {
    if relative_path.starts_with("94 - Canon Synthesis/") {
        SourceAuthority::CanonSynthesis
    } else if body.contains("[USER-CORRECTED]") {
        SourceAuthority::UserCorrected
    } else if body.contains("[USER-STATED]") {
        SourceAuthority::UserStated
    } else if body.contains("[RECOVERED-NONVERBATIM]")
        || body.contains("[RECOVERED-UNCERTAIN]")
    {
        SourceAuthority::RecoveredUserContext
    } else if body.contains("[STRUCTURED-VAULT]") || relative_path.starts_with("0") {
        SourceAuthority::StructuredVault
    } else if body.contains("[ASSISTANT-SYNTHESIS]") {
        SourceAuthority::AssistantSynthesis
    } else if body.contains("[ASSISTANT-RECONSTRUCTION]") {
        SourceAuthority::AssistantReconstruction
    } else {
        SourceAuthority::Archive
    }
}

fn canon_state_for(status: Option<&str>, relative_path: &str, body: &str) -> CanonState {
    let status = status.unwrap_or_default().to_lowercase();
    if body.contains("[UNRESOLVED]") || status.contains("unresolved") {
        CanonState::Unresolved
    } else if status.contains("contradiction") || relative_path.to_lowercase().contains("conflict") {
        CanonState::Contradiction
    } else if relative_path.starts_with("94 - Canon Synthesis/")
        || status.contains("active-canon")
        || status.contains("current-working-canon")
    {
        CanonState::Canon
    } else if status.contains("working") || status.contains("mixed") {
        CanonState::Working
    } else {
        CanonState::Archive
    }
}

#[derive(Default)]
struct Frontmatter {
    status: Option<String>,
    tags: Vec<String>,
}

impl Frontmatter {
    fn parse(body: &str) -> Self {
        let mut lines = body.lines();
        if lines.next() != Some("---") {
            return Self::default();
        }

        let mut result = Self::default();
        let mut reading_tags = false;
        for line in lines {
            if line == "---" {
                break;
            }
            if let Some(status) = line.strip_prefix("status:") {
                result.status = Some(status.trim().to_owned());
                reading_tags = false;
            } else if line == "tags:" {
                reading_tags = true;
            } else if reading_tags {
                if let Some(tag) = line.trim().strip_prefix("- ") {
                    result.tags.push(tag.trim().to_owned());
                } else if !line.starts_with(' ') {
                    reading_tags = false;
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn indexes_markdown_and_preserves_provenance() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let root = env::temp_dir().join(format!("kovia-vault-test-{nonce}"));
        fs::create_dir_all(root.join("91 - Recovered Conversation Memory"))
            .expect("fixture directory");
        let path = root
            .join("91 - Recovered Conversation Memory")
            .join("Eastern Wall.md");
        fs::write(
            &path,
            "---\nstatus: mixed-recovery\ntags:\n  - geography\n---\n# Eastern Wall\n\n- **[USER-STATED]** The wall is a retained fact with enough text for an excerpt.\n",
        )
        .expect("fixture note");

        let index = load(&root).expect("vault index");
        assert_eq!(index.notes.len(), 1);
        assert_eq!(index.notes[0].title, "Eastern Wall");
        assert_eq!(index.notes[0].authority, SourceAuthority::UserStated);
        assert_eq!(index.notes[0].canon_state, CanonState::Working);
        assert_eq!(index.notes[0].tags, vec!["geography"]);

        fs::remove_dir_all(root).expect("fixture cleanup");
    }
}
