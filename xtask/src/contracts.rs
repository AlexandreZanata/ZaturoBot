use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use std::{collections::BTreeSet, fs, path::Path};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    version: u32,
    tools: Vec<Tool>,
    integrations: Vec<Integration>,
    features: Vec<Feature>,
    migrations: Vec<Migration>,
    configurations: Vec<Configuration>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Tool {
    id: String,
    source: String,
    status: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Integration {
    id: String,
    status: String,
    documentation: String,
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Feature {
    id: String,
    phase: u8,
    status: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Migration {
    path: String,
    documentation: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    name: String,
    source: String,
    examples: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Prompts {
    prompts: Vec<Prompt>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Prompt {
    id: String,
    purpose: String,
    owner: String,
    version: u32,
    path: String,
    variables: Vec<String>,
    output_schema: String,
}

pub fn check(root: &Path) -> Result<()> {
    let registry: Registry =
        toml::from_str(&fs::read_to_string(root.join("config/registry.toml"))?)
            .context("invalid config/registry.toml")?;
    ensure!(registry.version == 1, "unsupported registry version");
    check_tools(root, &registry.tools).context("tool registration check failed")?;
    check_integrations(root, &registry.integrations)
        .context("integration registration check failed")?;
    check_features(root, &registry.features).context("feature status check failed")?;
    check_task_states(root).context("task state documentation check failed")?;
    check_migrations(root, &registry.migrations).context("migration documentation check failed")?;
    check_configurations(root, &registry.configurations)
        .context("configuration contract check failed")?;
    check_prompts(root).context("prompts/registry.toml declaration check failed")
}

fn unique<'a>(
    values: impl IntoIterator<Item = &'a str>,
    category: &str,
) -> Result<BTreeSet<String>> {
    let mut seen = BTreeSet::new();
    for value in values {
        ensure!(
            !value.is_empty()
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._/-".contains(&byte)),
            "invalid {category} identifier: {value:?}"
        );
        ensure!(
            seen.insert(value.to_owned()),
            "duplicate {category}: {value}"
        );
    }
    Ok(seen)
}

fn check_tools(root: &Path, tools: &[Tool]) -> Result<()> {
    let registered = unique(tools.iter().map(|tool| tool.id.as_str()), "tool")?;
    for tool in tools {
        ensure!(
            tool.status == "implemented",
            "registered executable tool must be implemented: {}",
            tool.id
        );
        ensure!(
            root.join(&tool.source).is_file(),
            "tool {} source missing",
            tool.id
        );
    }
    let source = fs::read_to_string(root.join("crates/tools/src/lib.rs"))?;
    let declaration = source
        .split_once("pub const TOOL_IDS")
        .context("tool implementation must expose canonical TOOL_IDS")?
        .1
        .split_once('=')
        .context("invalid TOOL_IDS declaration")?
        .1
        .split_once(';')
        .context("unterminated TOOL_IDS declaration")?
        .0;
    let implemented = unique(
        string_literals(declaration).iter().map(String::as_str),
        "tool implementation",
    )?;
    ensure!(
        registered == implemented,
        "unregistered or missing executable tools: registry and TOOL_IDS differ"
    );
    Ok(())
}

fn check_integrations(root: &Path, integrations: &[Integration]) -> Result<()> {
    unique(
        integrations
            .iter()
            .map(|integration| integration.id.as_str()),
        "integration",
    )?;
    let mut implementations = BTreeSet::new();
    for integration in integrations {
        ensure!(
            matches!(integration.status.as_str(), "planned" | "implemented"),
            "invalid integration status"
        );
        let documentation = fs::read_to_string(root.join(&integration.documentation))?;
        ensure!(
            documentation.to_lowercase().contains(&integration.id),
            "integration {} missing documentation",
            integration.id
        );
        match (&integration.source, integration.status.as_str()) {
            (Some(source), "implemented") => {
                ensure!(
                    root.join(source).exists(),
                    "missing integration source: {source}"
                );
                implementations.insert(source.clone());
            }
            (None, "planned") => {}
            _ => bail!("integration {} source/status conflict", integration.id),
        }
    }
    for entry in fs::read_dir(root.join("integrations"))? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let relative = format!("integrations/{}", entry.file_name().to_string_lossy());
            ensure!(
                implementations.contains(&relative),
                "unregistered integration directory: {relative}"
            );
        }
    }
    Ok(())
}

fn check_features(root: &Path, features: &[Feature]) -> Result<()> {
    unique(
        features.iter().map(|feature| feature.id.as_str()),
        "feature",
    )?;
    let roadmap = fs::read_to_string(root.join("docs/ROADMAP.md"))?;
    let markers: BTreeSet<_> = roadmap
        .lines()
        .filter(|line| line.starts_with("<!-- feature:"))
        .map(str::to_owned)
        .collect();
    let expected: BTreeSet<_> = features
        .iter()
        .map(|feature| format!("<!-- feature:{} status:{} -->", feature.id, feature.status))
        .collect();
    ensure!(
        markers == expected,
        "roadmap feature status markers disagree with config/registry.toml"
    );
    for feature in features {
        ensure!(
            feature.phase <= 8,
            "unknown roadmap phase for {}",
            feature.id
        );
        ensure!(
            matches!(
                feature.status.as_str(),
                "planned" | "implemented" | "partial"
            ),
            "unknown feature status"
        );
    }
    Ok(())
}

fn check_task_states(root: &Path) -> Result<()> {
    let text = fs::read_to_string(root.join("crates/core/src/lib.rs"))?;
    let declaration = text
        .split_once("pub const TASK_STATES")
        .context("core must declare canonical TASK_STATES")?
        .1
        .split_once('=')
        .context("invalid TASK_STATES declaration")?
        .1
        .split_once(';')
        .context("unterminated TASK_STATES declaration")?
        .0;
    let implemented = unique(
        string_literals(declaration).iter().map(String::as_str),
        "task state",
    )?;
    let documentation = fs::read_to_string(root.join("docs/TASK_LIFECYCLE.md"))?;
    let declared = documentation
        .split_once("<!-- task-states:")
        .context("missing task-states documentation marker")?
        .1
        .split_once("-->")
        .context("unterminated task-states marker")?
        .0;
    let documented = unique(declared.split_whitespace(), "documented task state")?;
    ensure!(
        implemented == documented,
        "task state documentation conflicts with core TASK_STATES"
    );
    Ok(())
}

fn check_migrations(root: &Path, migrations: &[Migration]) -> Result<()> {
    let registered = unique(
        migrations.iter().map(|migration| migration.path.as_str()),
        "migration",
    )?;
    let actual: BTreeSet<_> = fs::read_dir(root.join("crates/db/migrations"))?
        .map(|entry| {
            let entry = entry?;
            Ok(entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "sql")
                .then(|| {
                    format!(
                        "crates/db/migrations/{}",
                        entry.file_name().to_string_lossy()
                    )
                }))
        })
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    ensure!(
        actual == registered,
        "undocumented or missing database migrations"
    );
    for migration in migrations {
        let documentation = fs::read_to_string(root.join(&migration.documentation))?;
        let name = Path::new(&migration.path)
            .file_name()
            .context("migration missing filename")?
            .to_string_lossy();
        ensure!(
            documentation.contains(name.as_ref()),
            "missing migration documentation: {}",
            migration.path
        );
    }
    Ok(())
}

fn check_configurations(root: &Path, configurations: &[Configuration]) -> Result<()> {
    unique(
        configurations
            .iter()
            .map(|configuration| configuration.name.as_str()),
        "configuration",
    )?;
    let sources = rust_sources(root)?;
    for configuration in configurations {
        let text = fs::read_to_string(root.join(&configuration.source))?;
        let fields = config_fields(&text, &configuration.name)?;
        ensure!(
            !fields.is_empty(),
            "configuration {} has no canonical fields",
            configuration.name
        );
        let declaration = format!("struct {}", configuration.name);
        let definitions: Vec<_> = sources
            .iter()
            .filter(|(_, text)| text.contains(&declaration))
            .map(|(path, _)| path)
            .collect();
        ensure!(
            definitions.len() == 1,
            "duplicate or missing {} definitions: {definitions:?}",
            configuration.name
        );
        for example in &configuration.examples {
            let text = fs::read_to_string(root.join(example))?;
            let value: toml::Table = toml::from_str(&text)
                .with_context(|| format!("invalid configuration example {example}"))?;
            for key in value.keys() {
                ensure!(
                    fields.contains(key),
                    "{example}: unknown configuration key {key}"
                );
            }
            ensure!(!value.is_empty(), "empty configuration example: {example}");
        }
    }
    Ok(())
}

fn config_fields(text: &str, name: &str) -> Result<BTreeSet<String>> {
    let body = text
        .split_once(&format!("struct {name}"))
        .with_context(|| format!("missing canonical {name} definition"))?
        .1
        .split_once('{')
        .context("invalid configuration struct")?
        .1
        .split_once('}')
        .context("unterminated configuration struct")?
        .0;
    Ok(body
        .lines()
        .filter_map(|line| {
            let line = line.trim().strip_prefix("pub ")?;
            Some(line.split_once(':')?.0.trim().to_owned())
        })
        .collect())
}

fn rust_sources(root: &Path) -> Result<Vec<(String, String)>> {
    fn visit(directory: &Path, output: &mut Vec<(String, String)>) -> Result<()> {
        if !directory.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                visit(&entry.path(), output)?;
            } else if entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "rs")
            {
                output.push((
                    entry.path().display().to_string(),
                    fs::read_to_string(entry.path())?,
                ));
            }
        }
        Ok(())
    }
    let mut output = Vec::new();
    for directory in ["apps", "crates"] {
        visit(&root.join(directory), &mut output)?;
    }
    Ok(output)
}

fn check_prompts(root: &Path) -> Result<()> {
    let declaration: Prompts =
        toml::from_str(&fs::read_to_string(root.join("prompts/registry.toml"))?)?;
    unique(
        declaration.prompts.iter().map(|prompt| prompt.id.as_str()),
        "prompt",
    )?;
    let registered = unique(
        declaration
            .prompts
            .iter()
            .map(|prompt| prompt.path.as_str()),
        "prompt path",
    )?;
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(root.join("prompts"))? {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "md")
            && !matches!(entry.file_name().to_str(), Some("README.md" | "AGENTS.md"))
        {
            actual.insert(format!("prompts/{}", entry.file_name().to_string_lossy()));
        }
    }
    ensure!(
        actual == registered,
        "unregistered or missing prompt templates"
    );
    for prompt in declaration.prompts {
        ensure!(
            prompt.purpose.split_whitespace().count() >= 4 && prompt.purpose.len() <= 256,
            "prompt needs a bounded meaningful purpose: {}",
            prompt.id
        );
        let architecture: toml::Value =
            toml::from_str(&fs::read_to_string(root.join("architecture.toml"))?)?;
        ensure!(
            architecture
                .get("crates")
                .and_then(|crates| crates.get(&prompt.owner))
                .is_some(),
            "prompt {} has an unknown owning component",
            prompt.id
        );
        ensure!(
            prompt.version > 0,
            "prompt version must be positive: {}",
            prompt.id
        );
        let variables = unique(
            prompt.variables.iter().map(String::as_str),
            "prompt variable",
        )?;
        let text = fs::read_to_string(root.join(&prompt.path))?;
        ensure!(
            placeholders(&text)? == variables,
            "undeclared or unused prompt variables: {}",
            prompt.id
        );
        let schema: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(root.join(&prompt.output_schema))?)?;
        ensure!(schema.is_object(), "prompt output schema must be an object");
    }
    Ok(())
}

fn placeholders(text: &str) -> Result<BTreeSet<String>> {
    let mut remaining = text;
    let mut result = BTreeSet::new();
    while let Some((_, tail)) = remaining.split_once("{{") {
        let (variable, rest) = tail
            .split_once("}}")
            .context("unterminated prompt variable")?;
        ensure!(!variable.trim().is_empty(), "empty prompt variable");
        result.insert(variable.trim().to_owned());
        remaining = rest;
    }
    ensure!(
        !remaining.contains("}}"),
        "unmatched closing prompt variable"
    );
    Ok(result)
}

fn string_literals(text: &str) -> Vec<String> {
    text.split('"')
        .enumerate()
        .filter(|(index, _)| index % 2 == 1)
        .map(|(_, value)| value.to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_ids_and_invalid_declarations_fail() {
        assert!(unique(["diagnostic", "diagnostic"], "tool").is_err());
        assert!(unique(["diagnostic", "read-only"], "tool").is_ok());
        assert!(unique(["bad id"], "tool").is_err());
    }

    #[test]
    fn malformed_prompt_declarations_fail() {
        assert_eq!(
            placeholders("Read {{request}} and {{context}}.").unwrap(),
            BTreeSet::from(["request".into(), "context".into()])
        );
        assert!(placeholders("Read {{request").is_err());
        assert!(placeholders("Read {{ }}").is_err());
        assert!(placeholders("Read text}}").is_err());
        assert!(toml::from_str::<Prompts>("prompts=[]\nunknown=true").is_err());
    }

    #[test]
    fn prompt_purpose_and_owner_are_required_and_checked() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        fs::create_dir(root.join("prompts")).unwrap();
        fs::write(
            root.join("architecture.toml"),
            "[crates.planner]\npath='crates/planner'\n",
        )
        .unwrap();
        fs::write(
            root.join("prompts/plan.md"),
            "Return one bounded structured action.",
        )
        .unwrap();
        fs::write(root.join("prompts/schema.json"), r#"{"type":"object"}"#).unwrap();
        let valid = "[[prompts]]\nid='plan'\npurpose='Choose one bounded diagnostic action'\nowner='planner'\nversion=1\npath='prompts/plan.md'\nvariables=[]\noutput_schema='prompts/schema.json'\n";
        fs::write(root.join("prompts/registry.toml"), valid).unwrap();
        check_prompts(root).unwrap();
        fs::write(
            root.join("prompts/registry.toml"),
            valid.replace("owner='planner'", "owner='missing'"),
        )
        .unwrap();
        assert!(check_prompts(root).is_err());
        fs::write(
            root.join("prompts/registry.toml"),
            valid.replace("Choose one bounded diagnostic action", ""),
        )
        .unwrap();
        assert!(check_prompts(root).is_err());
        assert!(toml::from_str::<Prompts>(&valid.replace("owner='planner'\n", "")).is_err());
    }

    #[test]
    fn canonical_configuration_fields_are_discovered() {
        let fields = config_fields(
            "pub struct Config {\n pub bind: String,\n pub limit: u32,\n}",
            "Config",
        )
        .unwrap();
        assert_eq!(fields, BTreeSet::from(["bind".into(), "limit".into()]));
        assert!(config_fields("", "Config").is_err());
    }

    #[test]
    fn tool_inventory_reads_the_implementation_constant() {
        assert_eq!(
            string_literals("&[\"diagnostic.v1\", \"safe.v1\"]"),
            ["diagnostic.v1", "safe.v1"]
        );
    }

    #[test]
    fn unregistered_tool_fails_against_actual_source() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        fs::create_dir_all(root.join("crates/tools/src")).unwrap();
        fs::write(
            root.join("crates/tools/src/lib.rs"),
            "pub const TOOL_IDS: &[&str] = &[\"diagnostic.v1\"];",
        )
        .unwrap();
        let registered = vec![Tool {
            id: "diagnostic.v1".into(),
            source: "crates/tools/src/lib.rs".into(),
            status: "implemented".into(),
        }];
        assert!(check_tools(root, &registered).is_ok());
        fs::write(
            root.join("crates/tools/src/lib.rs"),
            "pub const TOOL_IDS: &[&str] = &[\"diagnostic.v1\", \"unregistered.v1\"];",
        )
        .unwrap();
        assert!(check_tools(root, &registered).is_err());
    }

    #[test]
    fn undocumented_migration_fails_against_actual_directory() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        fs::create_dir_all(root.join("crates/db/migrations")).unwrap();
        fs::write(root.join("crates/db/migrations/001.sql"), "SELECT 1;").unwrap();
        fs::write(
            root.join("MIGRATIONS.md"),
            "001.sql creates the task schema.",
        )
        .unwrap();
        let migrations = vec![Migration {
            path: "crates/db/migrations/001.sql".into(),
            documentation: "MIGRATIONS.md".into(),
        }];
        assert!(check_migrations(root, &migrations).is_ok());
        fs::write(root.join("crates/db/migrations/002.sql"), "SELECT 2;").unwrap();
        assert!(check_migrations(root, &migrations).is_err());
    }

    #[test]
    fn inconsistent_feature_status_fails_against_roadmap() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        fs::create_dir_all(root.join("docs")).unwrap();
        let features = vec![Feature {
            id: "foundation".into(),
            phase: 1,
            status: "implemented".into(),
        }];
        fs::write(
            root.join("docs/ROADMAP.md"),
            "<!-- feature:foundation status:implemented -->",
        )
        .unwrap();
        assert!(check_features(root, &features).is_ok());
        fs::write(
            root.join("docs/ROADMAP.md"),
            "<!-- feature:foundation status:planned -->",
        )
        .unwrap();
        assert!(check_features(root, &features).is_err());
    }

    #[test]
    fn conflicting_configurations_and_unknown_fields_fail() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        fs::create_dir_all(root.join("apps/test/src")).unwrap();
        fs::write(
            root.join("apps/test/src/config.rs"),
            "pub struct HubConfig {\n pub bind: String,\n}",
        )
        .unwrap();
        fs::write(root.join("hub.toml"), "bind='127.0.0.1:0'").unwrap();
        let configurations = vec![Configuration {
            name: "HubConfig".into(),
            source: "apps/test/src/config.rs".into(),
            examples: vec!["hub.toml".into()],
        }];
        assert!(check_configurations(root, &configurations).is_ok());
        fs::write(root.join("hub.toml"), "unrecognized=1").unwrap();
        assert!(check_configurations(root, &configurations).is_err());
        fs::write(root.join("hub.toml"), "bind='127.0.0.1:0'").unwrap();
        fs::write(
            root.join("apps/test/src/duplicate.rs"),
            "pub struct HubConfig {} ",
        )
        .unwrap();
        assert!(check_configurations(root, &configurations).is_err());
    }

    #[test]
    fn unregistered_integration_directory_fails() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        fs::create_dir_all(root.join("integrations")).unwrap();
        assert!(check_integrations(root, &[]).is_ok());
        fs::create_dir_all(root.join("integrations/unknown")).unwrap();
        assert!(check_integrations(root, &[]).is_err());
    }

    #[test]
    fn unregistered_prompt_template_fails() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        fs::create_dir_all(root.join("prompts")).unwrap();
        fs::write(root.join("prompts/registry.toml"), "prompts=[]").unwrap();
        assert!(check_prompts(root).is_ok());
        fs::write(root.join("prompts/rogue.md"), "An undeclared template.").unwrap();
        assert!(check_prompts(root).is_err());
    }
}
