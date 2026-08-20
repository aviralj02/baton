//! The `Context` domain type and its markdown rendering.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ContextBody {
    pub description: Option<String>,
    pub goal: Option<String>,
    pub current_state: Option<String>,
    pub decisions: Vec<String>,
    pub tried: Vec<String>,
    pub relevant_files: Vec<String>,
    pub constraints: Vec<String>,
    pub open_issues: Vec<String>,
    pub next_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    pub id: String,
    pub name: String,
    #[serde(flatten)]
    pub body: ContextBody,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSummary {
    pub id: String,
    pub name: String,
    pub updated_at: String,
}

impl Context {
    pub fn to_markdown(&self) -> String {
        let mut out = String::with_capacity(512);
        out.push_str(&format!("# {}\n", self.name.trim()));

        let b = &self.body;
        push_para(&mut out, "Goal", b.goal.as_deref());
        push_para(&mut out, "Current State", b.current_state.as_deref());
        push_list(&mut out, "Decisions", &b.decisions);
        push_list(&mut out, "Things Tried", &b.tried);
        push_list(&mut out, "Constraints", &b.constraints);
        push_list(&mut out, "Relevant Files", &b.relevant_files);
        push_list(&mut out, "Open Issues", &b.open_issues);
        push_list(&mut out, "Next Steps", &b.next_steps);

        out
    }
}

fn push_para(out: &mut String, heading: &str, value: Option<&str>) {
    let Some(text) = value.map(str::trim).filter(|s| !s.is_empty()) else {
        return;
    };
    out.push_str(&format!("\n## {heading}\n{text}\n"));
}

fn push_list(out: &mut String, heading: &str, items: &[String]) {
    let items: Vec<&str> = items
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if items.is_empty() {
        return;
    }
    out.push_str(&format!("\n## {heading}\n"));
    for item in items {
        out.push_str(&format!("- {item}\n"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Context {
        Context {
            id: "1".into(),
            name: "Auth migration".into(),
            body: ContextBody {
                goal: Some("Replace auth with OAuth.".into()),
                decisions: vec!["Don't use NextAuth.".into()],
                open_issues: vec!["  ".into()], // whitespace-only: must be dropped
                ..Default::default()
            },
            created_at: "2026-08-20T00:00:00Z".into(),
            updated_at: "2026-08-20T00:00:00Z".into(),
        }
    }

    #[test]
    fn renders_only_populated_sections() {
        let md = sample().to_markdown();
        assert!(md.starts_with("# Auth migration\n"));
        assert!(md.contains("## Goal\nReplace auth with OAuth."));
        assert!(md.contains("## Decisions\n- Don't use NextAuth."));
        // Empty and whitespace-only sections are omitted entirely.
        assert!(!md.contains("## Current State"));
        assert!(!md.contains("## Open Issues"));
        assert!(!md.contains("## Next Steps"));
    }

    #[test]
    fn body_flattens_into_one_object() {
        let json = serde_json::to_value(sample()).unwrap();
        // The frontend must see a flat object, not { body: {...} }.
        assert!(json.get("body").is_none());
        assert_eq!(json["goal"], "Replace auth with OAuth.");
        assert_eq!(json["updatedAt"], "2026-08-20T00:00:00Z");
    }

    #[test]
    fn missing_json_fields_default_rather_than_fail() {
        // A row written before a field existed must still load.
        let body: ContextBody = serde_json::from_str(r#"{"goal":"g"}"#).unwrap();
        assert_eq!(body.goal.as_deref(), Some("g"));
        assert!(body.decisions.is_empty());
    }
}
