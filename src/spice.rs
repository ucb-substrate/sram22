//! Portable circuit netlists with included cell definitions inlined.
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

pub(crate) fn is_external_or_model_directive(line: &str) -> bool {
    matches!(
        line.split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        ".include" | ".inc" | ".lib" | ".model"
    )
}

fn expand(path: &Path, stack: &mut Vec<PathBuf>) -> Result<String> {
    let path = path
        .canonicalize()
        .with_context(|| format!("missing SPICE input {}", path.display()))?;
    if stack.contains(&path) {
        bail!("cyclic SPICE include at {}", path.display());
    }
    stack.push(path.clone());
    let source = fs::read_to_string(&path)?;
    let mut output = String::new();
    for line in source.lines() {
        let command = line
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if matches!(command.as_str(), ".lib" | ".model") {
            bail!(
                "Device-model directives belong in the simulation testbench: {}: {line}",
                path.display()
            );
        }
        if !matches!(command.as_str(), ".include" | ".inc") {
            output.push_str(line);
            output.push('\n');
            continue;
        }
        let tokens = words(line)?;
        if tokens.len() != 2 {
            bail!(
                "unsupported SPICE include syntax in {}: {line}",
                path.display()
            );
        }
        let child = path.parent().unwrap().join(&tokens[1]);
        output.push_str(&expand(&child, stack)?);
    }
    stack.pop();
    Ok(output)
}

/// Inline circuit definitions; the simulation testbench supplies device models.
pub fn make_portable(path: &Path) -> Result<()> {
    let output = expand(path, &mut Vec::new())?;
    fs::write(path, output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_relative_circuit_includes() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("nested")).unwrap();
        fs::write(
            dir.path().join("top"),
            ".INCLUDE 'nested/cell with space'\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("nested/cell with space"),
            ".include '../leaf'\n",
        )
        .unwrap();
        fs::write(dir.path().join("leaf"), ".subckt test a b\n.ends test\n").unwrap();
        let result = expand(&dir.path().join("top"), &mut Vec::new()).unwrap();
        assert_eq!(result, ".subckt test a b\n.ends test\n");
    }

    #[test]
    fn reject_cycles_missing_files_and_device_models() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a");
        fs::write(&path, ".include a\n").unwrap();
        assert!(expand(&path, &mut Vec::new()).is_err());
        fs::write(&path, ".include absent\n").unwrap();
        assert!(expand(&path, &mut Vec::new()).is_err());
        for directive in [".lib models tt", ".model nfet nmos level=1"] {
            fs::write(&path, directive).unwrap();
            assert!(expand(&path, &mut Vec::new()).is_err());
        }
        assert!(words(".include \"unterminated").is_err());
    }
}
