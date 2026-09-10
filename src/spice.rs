//! Portable SPICE export: inline circuit includes and package selectable models.
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

fn words(line: &str) -> Result<Vec<String>> {
    let mut result = Vec::new();
    let mut chars = line.trim().chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            continue;
        }
        if c == '$' || c == ';' {
            break;
        }
        let mut word = String::new();
        if c == '"' || c == '\'' {
            let mut closed = false;
            for next in chars.by_ref() {
                if next == c {
                    closed = true;
                    break;
                }
                word.push(next);
            }
            if !closed {
                bail!("unterminated SPICE filename quote");
            }
        } else {
            word.push(c);
            while chars.peek().is_some_and(|c| !c.is_whitespace()) {
                word.push(chars.next().unwrap());
            }
        }
        result.push(word);
    }
    Ok(result)
}

fn expand(
    path: &Path,
    section: Option<&str>,
    stack: &mut Vec<(PathBuf, Option<String>)>,
) -> Result<String> {
    let path = path
        .canonicalize()
        .with_context(|| format!("missing SPICE input {}", path.display()))?;
    let key = (path.clone(), section.map(str::to_ascii_lowercase));
    if stack.contains(&key) {
        bail!("cyclic SPICE include at {}", path.display());
    }
    stack.push(key);
    let source = fs::read_to_string(&path)?;
    let mut output = String::new();
    let mut active = section.is_none();
    let mut found = section.is_none();
    for line in source.lines() {
        let command = line
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(command.as_str(), ".include" | ".inc" | ".lib" | ".endl") {
            if active {
                output.push_str(line);
                output.push('\n');
            }
            continue;
        }
        let tokens = words(line)?;
        if command == ".lib" && tokens.len() == 2 {
            if let Some(wanted) = section {
                active = tokens[1].eq_ignore_ascii_case(wanted);
                found |= active;
            } else {
                output.push_str(line);
                output.push('\n');
            }
        } else if command == ".endl" {
            if section.is_some() {
                active = false;
            } else {
                output.push_str(line);
                output.push('\n');
            }
        } else if active {
            let expected = if command == ".lib" { 3 } else { 2 };
            if tokens.len() != expected {
                bail!(
                    "unsupported SPICE include syntax in {}: {line}",
                    path.display()
                );
            }
            let child = path.parent().unwrap().join(&tokens[1]);
            let child_section = (command == ".lib").then(|| tokens[2].as_str());
            output.push_str(&expand(&child, child_section, stack)?);
        }
    }
    stack.pop();
    if !found {
        bail!(
            "SPICE library {} has no section {}",
            path.display(),
            section.unwrap()
        );
    }
    Ok(output)
}

/// Inline the complete circuit and one selected set of open device models.
pub fn make_portable(path: &Path, corner: &str) -> Result<()> {
    let mut output = expand(path, None, &mut Vec::new())?;
    let models = crate::tech::sky130::open_model_library()?;
    output.push_str(&format!("\n* Self-contained open SKY130 device models: {corner} corner.\n* Do not load another PDK model library alongside this file.\n"));
    output.push_str(&expand(&models, Some(corner), &mut Vec::new())?);
    fs::write(path, output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_relative_includes_and_selected_library() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("nested")).unwrap();
        fs::write(
            dir.path().join("top"),
            ".INCLUDE 'nested/cell with space'\n.lib nested/models ss\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("nested/cell with space"),
            ".include '../leaf'\n",
        )
        .unwrap();
        fs::write(dir.path().join("leaf"), ".subckt test a b\n.ends test\n").unwrap();
        fs::write(
            dir.path().join("nested/models"),
            ".lib tt\nwrong\n.endl\n.lib ss\nright\n.endl\n",
        )
        .unwrap();
        let result = expand(&dir.path().join("top"), None, &mut Vec::new()).unwrap();
        assert_eq!(result, ".subckt test a b\n.ends test\nright\n");
    }

    #[test]
    fn reject_cycles_missing_files_and_missing_sections() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a");
        fs::write(&path, ".include a\n").unwrap();
        assert!(expand(&path, None, &mut Vec::new()).is_err());
        fs::write(&path, ".include absent\n").unwrap();
        assert!(expand(&path, None, &mut Vec::new()).is_err());
        fs::write(&path, ".lib tt\n.endl\n").unwrap();
        assert!(expand(&path, Some("ss"), &mut Vec::new()).is_err());
        assert!(words(".include \"unterminated").is_err());
    }
}
