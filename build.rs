use std::path::Path;
use std::{env, fs};

fn collect(root: &Path, directory: &Path, entries: &mut String) {
    let mut paths: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect(root, &path, entries);
        } else {
            let relative = path.strip_prefix(root).unwrap().to_str().unwrap();
            entries.push_str(&format!("({relative:?}, include_bytes!({:?})),\n", path));
        }
    }
}

fn main() {
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = Path::new(&root);
    let mut entries = String::from("&[\n");
    for directory in ["tech/sky130/gds", "tech/sky130/spice", "templates"] {
        println!("cargo:rerun-if-changed={directory}");
        collect(root, &root.join(directory), &mut entries);
    }
    entries.push_str("]\n");
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("assets.rs"),
        entries,
    )
    .unwrap();
}
