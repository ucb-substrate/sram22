//! Extracted-netlist blocks use the native Substrate 2 SPICE schema.
pub use crate::netlist::Pex;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Resolves Calibre probes by source net and instance hierarchy.
/// Exact extracted names take precedence. A changed terminal name must resolve to a
/// unique nearest segment; choosing arbitrarily would change an RC timing measurement.
#[derive(Default)]
pub(crate) struct ExtractedNodes(HashSet<String>);
impl ExtractedNodes {
    pub fn read(paths: &[PathBuf]) -> anyhow::Result<Self> {
        fn read(
            path: &Path,
            seen: &mut HashSet<PathBuf>,
            nodes: &mut HashSet<String>,
            include_pattern: &regex::Regex,
        ) -> anyhow::Result<()> {
            let path = path.canonicalize()?;
            if !seen.insert(path.clone()) {
                return Ok(());
            }
            let text = std::fs::read_to_string(&path)?;
            for line in text.lines().map(str::trim).filter(|s| !s.starts_with('*')) {
                if let Some(captures) = include_pattern.captures(line) {
                    if let Some(include) = captures.iter().skip(1).flatten().next() {
                        read(
                            &path.parent().unwrap().join(include.as_str()),
                            seen,
                            nodes,
                            include_pattern,
                        )?;
                    }
                }
                nodes.extend(
                    line.split_whitespace()
                        .map(|s| s.trim_matches(['\'', '"', '(', ')']))
                        .filter(|s| s.to_ascii_lowercase().starts_with("n_"))
                        .map(str::to_ascii_lowercase),
                );
            }
            Ok(())
        }
        let mut out = Self::default();
        let mut seen = HashSet::new();
        let include_pattern =
            regex::Regex::new(r#"(?i)^\.(?:include|inc)\s+(?:"([^"]+)"|'([^']+)'|(\S+))"#)?;
        for path in paths {
            read(path, &mut seen, &mut out.0, &include_pattern)?;
        }
        Ok(out)
    }

    pub fn resolve(&self, path: &str) -> anyhow::Result<String> {
        let lower = path.to_ascii_lowercase();
        let Some((instance, node)) = lower.split_once(".n_") else {
            return Ok(path.to_owned());
        };
        let node = format!("n_{node}");
        if node.contains('*') || self.0.contains(&node) {
            return Ok(path.to_owned());
        }
        let Some((source, terminal)) = node[2..].split_once("_x0/") else {
            return Ok(path.to_owned());
        };
        let prefix = format!("n_{source}_x0/");
        let target: Vec<_> = terminal
            .split('/')
            .filter(|s| *s != "x0" && *s != "m0")
            .collect();
        let mut candidates: Vec<_> = self
            .0
            .iter()
            .filter(|n| n.starts_with(&prefix))
            .map(|n| {
                let score = n[prefix.len()..]
                    .split('/')
                    .filter(|s| *s != "x0" && *s != "m0")
                    .zip(&target)
                    .take_while(|(a, b)| a == *b)
                    .count();
                (score, n)
            })
            .collect();
        candidates.sort();
        let (score, best) = candidates.pop().ok_or_else(|| {
            anyhow::anyhow!("no extracted segment for source net {source} (requested {path})")
        })?;
        anyhow::ensure!(
            candidates.last().is_none_or(|(other, _)| *other < score),
            "ambiguous extracted segments for {path}; use an exact extracted node name"
        );
        Ok(format!("{instance}.{best}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn probes_follow_the_source_net_and_instance_hierarchy() {
        let map = ExtractedNodes(
            [
                "n_x0/bl[0]_x0/xcol/xgroup_0/mpull_d".into(),
                "n_x0/bl[0]_x0/xarray/xcell_0_0/m2_g".into(),
                "n_x0/bl[1]_x0/xcol/xgroup_0/mpull_d".into(),
            ]
            .into_iter()
            .collect(),
        );
        assert_eq!(
            map.resolve("Xdut_xinst0.N_X0/bl[0]_X0/Xcol/Xgroup_0/Xpull/M0_d")
                .unwrap(),
            "xdut_xinst0.n_x0/bl[0]_x0/xcol/xgroup_0/mpull_d"
        );
        assert!(map.resolve("Xdut.N_X0/missing_X0/Xm/M0_d").is_err());
        let mut ambiguous = map;
        ambiguous
            .0
            .insert("n_x0/bl[0]_x0/xcol/xgroup_0/mpull_s".into());
        assert!(ambiguous
            .resolve("Xdut.N_X0/bl[0]_X0/Xcol/Xgroup_0/Xpull/M0_d")
            .is_err());
    }

    #[test]
    fn extracted_includes_accept_spaces_and_cycles() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("parent.spice");
        let child = dir.path().join("child netlist.spice");
        std::fs::write(&parent, ".include \"child netlist.spice\"\n").unwrap();
        std::fs::write(&child, ".inc 'parent.spice'\nR0 N_NET_X0/XCELL/M1_d 0 10\n").unwrap();
        let nodes = ExtractedNodes::read(&[parent]).unwrap();
        assert_eq!(
            nodes.resolve("Xdut.N_NET_X0/XCELL/M0_d").unwrap(),
            "xdut.n_net_x0/xcell/m1_d"
        );
    }
}
