//! Offline provider imports. Input is JSON already on disk, never a URL.
use std::{collections::HashSet, io::Read, path::Path};

use serde::Deserialize;
use serde_json::Value;

use super::{FetchedIssues, ImportSummary, StatusMap, github, jira, linear};
use crate::{
    db::{DbPool, queries},
    error::LificError,
};

pub const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_ISSUES: usize = 10_000;

#[derive(Debug, Clone, Copy, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Github,
    Linear,
    Jira,
}

impl Source {
    fn name(self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Linear => "linear",
            Self::Jira => "jira",
        }
    }
}

fn bad(message: impl Into<String>) -> LificError {
    LificError::BadRequest(message.into())
}

pub fn read(path: &Path) -> Result<Vec<u8>, LificError> {
    let file =
        std::fs::File::open(path).map_err(|e| bad(format!("Cannot open import file: {e}")))?;
    if !file.metadata().map_err(|e| bad(e.to_string()))?.is_file() {
        return Err(bad("Choose a regular JSON file"));
    }
    let mut bytes = Vec::new();
    file.take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| bad(e.to_string()))?;
    check_size(bytes.len())?;
    Ok(bytes)
}

fn check_size(size: usize) -> Result<(), LificError> {
    if size > MAX_BYTES {
        return Err(LificError::PayloadTooLarge(
            "Import files must be at most 4 MiB".into(),
        ));
    }
    Ok(())
}

pub fn parse(
    source: Source,
    namespace: Option<&str>,
    bytes: &[u8],
) -> Result<FetchedIssues, LificError> {
    check_size(bytes.len())?;
    let document: Value =
        serde_json::from_slice(bytes).map_err(|e| bad(format!("Invalid JSON: {e}")))?;
    let rows = document
        .as_array()
        .or_else(|| document.get("issues").and_then(Value::as_array))
        .or_else(|| {
            document
                .pointer("/data/issues/nodes")
                .and_then(Value::as_array)
        })
        .or_else(|| document.pointer("/issues/nodes").and_then(Value::as_array))
        .ok_or_else(|| {
            bad("Expected an issue array, {\"issues\": [...]}, or a Linear issues.nodes export")
        })?;
    if rows.len() > MAX_ISSUES {
        return Err(bad("Import files may contain at most 10,000 issues"));
    }
    let namespace = namespace.unwrap_or("").trim();
    match source {
        Source::Github => {
            github::parse_repo(namespace).map_err(bad)?;
        }
        Source::Jira
            if namespace.is_empty()
                || namespace.len() > 120
                || !namespace
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)) =>
        {
            return Err(bad(
                "Jira needs a site label using letters, numbers, dots, hyphens or underscores",
            ));
        }
        _ => {}
    }
    let mut fetched = FetchedIssues::default();
    let mut seen = HashSet::new();
    for (index, row) in rows.iter().enumerate() {
        let invalid = |e| bad(format!("Issue {}: {e}", index + 1));
        let issue = match source {
            Source::Github => {
                let mut issue: github::GithubIssue =
                    serde_json::from_value(row.clone()).map_err(invalid)?;
                if issue.pull_request.is_some() {
                    fetched.skipped_non_issues += 1;
                    continue;
                }
                if issue.number <= 0 {
                    return Err(bad("GitHub issue numbers must be positive"));
                }
                issue.state.make_ascii_lowercase();
                if !matches!(issue.state.as_str(), "open" | "closed") {
                    return Err(bad("GitHub state must be open or closed"));
                }
                let comments = row
                    .get("comments")
                    .filter(|v| v.is_array())
                    .cloned()
                    .unwrap_or_else(|| Value::Array(vec![]));
                let comments: Vec<github::GithubComment> =
                    serde_json::from_value(comments).map_err(invalid)?;
                fetched.skipped_assignees += issue.assignees.len();
                github::map_issue(namespace, &issue, &comments, &StatusMap::default())
            }
            Source::Linear => {
                let issue: linear::LinearIssue =
                    serde_json::from_value(row.clone()).map_err(invalid)?;
                if issue.identifier.trim().is_empty() {
                    return Err(bad("Linear issue identifiers cannot be empty"));
                }
                fetched.skipped_assignees += usize::from(issue.assignee.is_some());
                linear::map_issue(&issue, &linear::LinearStatusMap::default())
            }
            Source::Jira => {
                let issue: jira::JiraIssue =
                    serde_json::from_value(row.clone()).map_err(invalid)?;
                if issue.key.trim().is_empty() {
                    return Err(bad("Jira issue keys cannot be empty"));
                }
                let comments = row
                    .pointer("/fields/comment/comments")
                    .or_else(|| row.get("comments"))
                    .filter(|v| v.is_array())
                    .cloned()
                    .unwrap_or_else(|| Value::Array(vec![]));
                let comments: Vec<jira::JiraComment> =
                    serde_json::from_value(comments).map_err(invalid)?;
                fetched.skipped_assignees += usize::from(issue.fields.assignee.is_some());
                jira::map_issue(
                    namespace,
                    &issue,
                    &comments,
                    &jira::JiraStatusMap::default(),
                )
            }
        };
        if issue.title.trim().is_empty() {
            return Err(bad(format!("Issue {} has no title", index + 1)));
        }
        if issue.comments.len() > 1000 {
            return Err(bad("At most 1,000 embedded comments per issue"));
        }
        if !seen.insert(issue.source.clone()) {
            return Err(bad(format!("Duplicate source in file: {}", issue.source)));
        }
        fetched.issues.push(issue);
    }
    Ok(fetched)
}

pub fn run(
    pool: &DbPool,
    project: &str,
    source: Source,
    namespace: Option<&str>,
    bytes: &[u8],
    dry_run: bool,
) -> Result<ImportSummary, LificError> {
    // Parse the entire file before creating a bot or writing any issue.
    let fetched = parse(source, namespace, bytes)?;
    let (project_id, operator) = {
        let conn = pool.read()?;
        (
            queries::resolve_project_identifier(&conn, project)?,
            queries::users::first_admin(&conn)?
                .ok_or_else(|| bad("Initialize the local workspace first"))?,
        )
    };
    let bot = if dry_run {
        None
    } else {
        Some(super::ensure_import_bot(
            pool,
            operator.id,
            source.name(),
            &format!("{} file import", source.name()),
        )?)
    };
    super::run_import(pool, project_id, bot, &fetched, dry_run)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_exports_preserve_content_without_fetching_references() {
        let github = parse(Source::Github, Some("sample/repo"), br#"[{"number":1,"title":"Local issue","state":"OPEN","comments":[{"body":"Saved comment"}],"comments_url":"https://never-contact.invalid/comments"},{"number":2,"title":"PR","state":"open","pull_request":{}}]"#).unwrap();
        assert_eq!(github.issues.len(), 1);
        assert_eq!(github.issues[0].comments[0].body, "Saved comment");
        assert_eq!(github.skipped_non_issues, 1);
        let linear = parse(Source::Linear, None, br#"{"data":{"issues":{"nodes":[{"identifier":"ENG-1","title":"Local Linear","comments":{"nodes":[{"body":"Note"}]}}]}}}"#).unwrap();
        assert_eq!(linear.issues[0].source, "linear:ENG-1");
        assert_eq!(linear.issues[0].comments.len(), 1);
        let jira = parse(Source::Jira, Some("company"), br#"{"issues":[{"key":"ENG-1","fields":{"summary":"Local Jira","comment":{"comments":[{"body":{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Saved"}]}]}}]}}}]}"#).unwrap();
        assert_eq!(jira.issues[0].source, "jira:company:ENG-1");
        assert!(jira.issues[0].comments[0].body.contains("Saved"));
    }

    #[test]
    fn rejects_invalid_and_duplicate_rows_before_writes() {
        assert!(parse(Source::Github, Some("sample/repo"), br#"[{"number":1,"title":"A","state":"open"},{"number":1,"title":"B","state":"open"}]"#).is_err());
        assert!(parse(Source::Linear, None, br#"[{"identifier":"","title":"A"}]"#).is_err());
        assert!(parse(Source::Jira, Some("https://external.invalid"), b"[]").is_err());
        assert!(parse(Source::Linear, None, &vec![b' '; MAX_BYTES + 1]).is_err());
    }
}
