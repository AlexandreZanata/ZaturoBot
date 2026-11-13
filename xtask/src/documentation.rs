use anyhow::{Context, Result, ensure};
use std::{collections::BTreeMap, fs, path::Path};

const REQUIRED: &[&str] = &[
    "AGENTS.md",
    "CLAUDE.md",
    "README.md",
    "docs/README.md",
    "docs/VISION.md",
    "docs/REQUIREMENTS.md",
    "docs/ARCHITECTURE.md",
    "docs/COMPONENTS.md",
    "docs/CONVENTIONS.md",
    "docs/DECISIONS.md",
    "docs/SECURITY.md",
    "docs/THREAT_MODEL.md",
    "docs/AGENT_ARCHITECTURE.md",
    "docs/AGENT_WORKFLOW.md",
    "docs/COMPUTER_USE.md",
    "docs/TASK_LIFECYCLE.md",
    "docs/SCHEDULER.md",
    "docs/LLM_PROVIDERS.md",
    "docs/VAULT_ARCHITECTURE.md",
    "docs/INTEGRATIONS.md",
    "docs/COMMUNICATION_PROTOCOL.md",
    "docs/RESOURCE_LIMITS.md",
    "docs/DEPLOYMENT.md",
    "docs/TESTING.md",
    "docs/OBSERVABILITY.md",
    "docs/ROADMAP.md",
    "docs/OPEN_QUESTIONS.md",
    "docs/GLOSSARY.md",
    "docs/MIGRATIONS.md",
    "docs/recipes/README.md",
    "docs/recipes/add-crate.md",
    "docs/recipes/add-integration.md",
    "docs/recipes/add-agent-tool.md",
    "docs/recipes/add-llm-provider.md",
    "docs/recipes/add-api-endpoint.md",
    "docs/recipes/add-database-migration.md",
    "docs/recipes/add-task-type.md",
    "docs/recipes/add-permission.md",
    "docs/recipes/change-agent-prompt.md",
    "docs/recipes/debug-failed-task.md",
    "docs/recipes/recover-interrupted-job.md",
    "docs/recipes/finish-phase.md",
    "integrations/README.md",
    "prompts/README.md",
];

pub fn check(root: &Path) -> Result<()> {
    for relative in REQUIRED {
        let path = root.join(relative);
        ensure!(path.is_file(), "missing required documentation: {relative}");
        let text = fs::read_to_string(&path)?;
        ensure!(
            text.split_whitespace().count() >= 25,
            "documentation is too sparse to be useful: {relative}"
        );
        ensure!(
            !text
                .lines()
                .any(|line| line.trim() == "TODO" || line.trim() == "TODO."),
            "placeholder documentation: {relative}"
        );
    }
    walk(root, root)
}

fn walk(root: &Path, directory: &Path) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            if !matches!(
                entry.file_name().to_str(),
                Some(
                    ".git"
                        | ".secrets"
                        | ".agents"
                        | ".codex"
                        | ".aws"
                        | "target"
                        | "data"
                        | "node_modules"
                )
            ) {
                walk(root, &path)?;
            }
        } else if path.extension().is_some_and(|extension| extension == "md") {
            let text = fs::read_to_string(&path)?;
            for link in links(&text) {
                validate_link(root, &path, &link)
                    .with_context(|| format!("{}: broken link {link:?}", path.display()))?;
            }
        }
    }
    Ok(())
}

fn links(text: &str) -> Vec<String> {
    let mut output = Vec::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let mut rest = line;
        while let Some(start) = rest.find("](") {
            rest = &rest[start + 2..];
            if let Some(end) = rest.find(')') {
                let target = rest[..end].trim();
                let target = if let Some(angle) = target.strip_prefix('<') {
                    angle.split('>').next().unwrap_or(angle)
                } else {
                    target.split_whitespace().next().unwrap_or("")
                };
                output.push(target.to_owned());
                rest = &rest[end + 1..];
            } else {
                break;
            }
        }
    }
    output
}

fn validate_link(root: &Path, source: &Path, link: &str) -> Result<()> {
    if link.starts_with("https://")
        || link.starts_with("http://")
        || link.starts_with("mailto:")
        || link.starts_with("codex://")
    {
        return Ok(());
    }
    ensure!(!link.is_empty(), "empty documentation link");
    let (target, anchor) = link
        .split_once('#')
        .map_or((link, None), |(file, anchor)| (file, Some(anchor)));
    let decoded = target.replace("%20", " ");
    let path = if decoded.is_empty() {
        source.to_owned()
    } else {
        source
            .parent()
            .context("source has no parent")?
            .join(decoded)
    };
    let resolved = path.canonicalize().context("target does not exist")?;
    ensure!(resolved.starts_with(root), "link escapes the repository");
    if let Some(anchor) = anchor
        && !anchor.is_empty()
        && resolved.is_file()
    {
        let text = fs::read_to_string(resolved)?;
        ensure!(
            anchors(&text).contains_key(anchor),
            "missing heading anchor #{anchor}"
        );
    }
    Ok(())
}

fn anchors(text: &str) -> BTreeMap<String, usize> {
    let mut output = BTreeMap::new();
    let mut counts = BTreeMap::new();
    for line in text.lines() {
        let Some(heading) = line.strip_prefix('#') else {
            continue;
        };
        let heading = heading.trim_start_matches('#').trim();
        let slug: String = heading
            .chars()
            .flat_map(char::to_lowercase)
            .filter_map(|character| {
                if character.is_alphanumeric() || character == '-' || character == '_' {
                    Some(character)
                } else if character.is_whitespace() {
                    Some('-')
                } else {
                    None
                }
            })
            .collect();
        let count = counts.entry(slug.clone()).or_insert(0);
        let unique = if *count == 0 {
            slug
        } else {
            format!("{slug}-{count}")
        };
        *count += 1;
        output.insert(unique, *count);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_extraction_skips_fenced_examples_and_preserves_anchors() {
        let text = "[contract](../docs/README.md#start)\n```md\n[missing](ghost.md)\n```\n[space](<some file.md>)";
        assert_eq!(links(text), ["../docs/README.md#start", "some file.md"]);
    }

    #[test]
    fn heading_anchors_support_repeated_headings() {
        let text = "# Getting Started!\n## Getting Started!\n";
        let found = anchors(text);
        assert!(found.contains_key("getting-started"));
        assert!(found.contains_key("getting-started-1"));
        assert!(!found.contains_key("missing"));
    }

    #[test]
    fn missing_link_is_an_error() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        assert!(validate_link(root, &root.join("README.md"), "certainly-missing.md").is_err());
    }

    #[test]
    fn required_document_removal_fails() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        let content = "Useful documentation content ".repeat(12);
        for relative in REQUIRED {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, &content).unwrap();
        }
        assert!(check(root).is_ok());
        fs::remove_file(root.join("docs/SECURITY.md")).unwrap();
        assert!(check(root).is_err());
    }

    #[test]
    fn existing_file_with_broken_anchor_still_fails() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        let source = root.join("README.md");
        fs::write(&source, "# Existing heading\n").unwrap();
        assert!(validate_link(root, &source, "#existing-heading").is_ok());
        assert!(validate_link(root, &source, "#missing-heading").is_err());
    }
}
