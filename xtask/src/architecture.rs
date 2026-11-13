use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Architecture {
    version: u32,
    crates: BTreeMap<String, Rule>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    path: String,
    may_depend_on: Vec<String>,
    #[serde(default)]
    may_dev_depend_on: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Package {
    id: String,
    name: String,
    manifest_path: PathBuf,
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
struct Dependency {
    name: String,
    kind: Option<String>,
}

pub fn check(root: &Path, locked: bool) -> Result<()> {
    let text = fs::read_to_string(root.join("architecture.toml"))?;
    let architecture: Architecture = toml::from_str(&text).context("invalid architecture.toml")?;
    let mut command = Command::new("cargo");
    command.args(["metadata", "--format-version", "1", "--no-deps"]);
    if locked {
        command.arg("--locked");
    }
    let output = command.current_dir(root).output()?;
    ensure!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Metadata = serde_json::from_slice(&output.stdout)?;
    validate(root, &architecture, &metadata)
}

fn validate(root: &Path, architecture: &Architecture, metadata: &Metadata) -> Result<()> {
    ensure!(
        architecture.version == 1,
        "unsupported architecture version"
    );
    let members: BTreeMap<_, _> = metadata
        .packages
        .iter()
        .filter(|package| metadata.workspace_members.contains(&package.id))
        .map(|package| (package.name.clone(), package))
        .collect();
    ensure!(
        members.keys().collect::<Vec<_>>() == architecture.crates.keys().collect::<Vec<_>>(),
        "architecture.toml must declare exactly the workspace package names"
    );
    let components = fs::read_to_string(root.join("docs/COMPONENTS.md"))?;
    let mut edges = BTreeMap::new();
    for (name, package) in &members {
        let rule = &architecture.crates[name];
        ensure!(
            package.manifest_path == root.join(&rule.path).join("Cargo.toml"),
            "{name}: architecture path does not match Cargo metadata"
        );
        ensure!(
            root.join(&rule.path).join("AGENTS.md").is_file(),
            "{name}: missing crate-level AGENTS.md"
        );
        ensure!(
            components.contains(&format!("`{name}`")),
            "{name}: missing package entry in docs/COMPONENTS.md"
        );
        let allowed: BTreeSet<_> = rule.may_depend_on.iter().cloned().collect();
        let dev_allowed: BTreeSet<_> = rule.may_dev_depend_on.iter().cloned().collect();
        ensure!(
            allowed.len() == rule.may_depend_on.len(),
            "{name}: duplicate allowed dependencies"
        );
        ensure!(
            dev_allowed.len() == rule.may_dev_depend_on.len(),
            "{name}: duplicate allowed development dependencies"
        );
        for dependency in allowed.union(&dev_allowed) {
            ensure!(
                members.contains_key(dependency),
                "{name}: unknown dependency {dependency}"
            );
            ensure!(dependency != name, "{name}: self dependency is prohibited");
        }
        let dependencies: Vec<_> = package
            .dependencies
            .iter()
            .filter(|dependency| members.contains_key(&dependency.name))
            .collect();
        for dependency in &dependencies {
            ensure!(
                allowed.contains(&dependency.name)
                    || (dependency.kind.as_deref() == Some("dev")
                        && dev_allowed.contains(&dependency.name)),
                "{name}: forbidden {:?} dependency on {}",
                dependency.kind,
                dependency.name
            );
        }
        let actual = dependencies
            .into_iter()
            .map(|dependency| dependency.name.clone())
            .collect();
        edges.insert(name.clone(), actual);
        check_central_dependencies(&package.manifest_path)?;
    }
    reject_cycles(&edges)
}

fn check_central_dependencies(manifest_path: &Path) -> Result<()> {
    let manifest: toml::Value = toml::from_str(&fs::read_to_string(manifest_path)?)?;
    fn visit(value: &toml::Value, manifest: &Path) -> Result<()> {
        let Some(table) = value.as_table() else {
            return Ok(());
        };
        for (key, value) in table {
            if matches!(
                key.as_str(),
                "dependencies" | "dev-dependencies" | "build-dependencies"
            ) {
                let dependencies = value
                    .as_table()
                    .context("dependency section must be a table")?;
                for (name, declaration) in dependencies {
                    ensure!(
                        declaration.get("workspace").and_then(toml::Value::as_bool) == Some(true),
                        "{}: dependency {name} must inherit its central workspace declaration",
                        manifest.display()
                    );
                }
            } else if key == "target" || key.starts_with("cfg(") || value.is_table() {
                visit(value, manifest)?;
            }
        }
        Ok(())
    }
    visit(&manifest, manifest_path)
}

fn reject_cycles(edges: &BTreeMap<String, BTreeSet<String>>) -> Result<()> {
    fn walk(
        name: &str,
        edges: &BTreeMap<String, BTreeSet<String>>,
        pending: &mut BTreeSet<String>,
        complete: &mut BTreeSet<String>,
    ) -> Result<()> {
        if complete.contains(name) {
            return Ok(());
        }
        if !pending.insert(name.to_owned()) {
            bail!("cyclic internal dependency involving {name}");
        }
        for dependency in &edges[name] {
            walk(dependency, edges, pending, complete)?;
        }
        pending.remove(name);
        complete.insert(name.to_owned());
        Ok(())
    }
    let mut complete = BTreeSet::new();
    for name in edges.keys() {
        walk(name, edges, &mut BTreeSet::new(), &mut complete)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_cycles_including_indirect_dependencies() {
        let mut edges = BTreeMap::from([
            ("a".into(), BTreeSet::from(["b".into()])),
            ("b".into(), BTreeSet::from(["c".into()])),
            ("c".into(), BTreeSet::new()),
        ]);
        assert!(reject_cycles(&edges).is_ok());
        edges.get_mut("c").unwrap().insert("a".into());
        assert!(reject_cycles(&edges).is_err());
    }

    #[test]
    fn architecture_rejects_unknown_contract_fields() {
        let contract = "version = 1\nextra = true\n[crates]\n";
        assert!(toml::from_str::<Architecture>(contract).is_err());
    }

    #[test]
    fn forbidden_dependency_and_missing_crate_instructions_fail() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/COMPONENTS.md"), "`a` and `b`").unwrap();
        let mut packages = Vec::new();
        for name in ["a", "b"] {
            let directory = root.join(name);
            fs::create_dir_all(&directory).unwrap();
            fs::write(directory.join("AGENTS.md"), "Instructions").unwrap();
            fs::write(directory.join("Cargo.toml"), "[package]\nname='fixture'\n").unwrap();
            packages.push(Package {
                id: name.into(),
                name: name.into(),
                manifest_path: directory.join("Cargo.toml"),
                dependencies: if name == "a" {
                    vec![Dependency {
                        name: "b".into(),
                        kind: None,
                    }]
                } else {
                    Vec::new()
                },
            });
        }
        let mut architecture = Architecture {
            version: 1,
            crates: ["a", "b"]
                .into_iter()
                .map(|name| {
                    (
                        name.into(),
                        Rule {
                            path: name.into(),
                            may_depend_on: Vec::new(),
                            may_dev_depend_on: Vec::new(),
                        },
                    )
                })
                .collect(),
        };
        let mut metadata = Metadata {
            packages,
            workspace_members: vec!["a".into(), "b".into()],
        };
        assert!(validate(root, &architecture, &metadata).is_err());
        architecture
            .crates
            .get_mut("a")
            .unwrap()
            .may_depend_on
            .push("b".into());
        assert!(validate(root, &architecture, &metadata).is_ok());
        let rule = architecture.crates.get_mut("a").unwrap();
        rule.may_depend_on.clear();
        rule.may_dev_depend_on.push("b".into());
        assert!(validate(root, &architecture, &metadata).is_err());
        metadata.packages[0].dependencies[0].kind = Some("dev".into());
        assert!(validate(root, &architecture, &metadata).is_ok());
        fs::remove_file(root.join("b/AGENTS.md")).unwrap();
        assert!(validate(root, &architecture, &metadata).is_err());
    }
}
