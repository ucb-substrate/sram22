use std::path::Path;
use std::{env, fs};

fn collect(root: &Path, path: &Path, entries: &mut String) {
    if path.is_dir() {
        let mut paths: Vec<_> = fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        paths.sort();
        for path in paths {
            collect(root, &path, entries);
        }
    } else {
        let relative = path.strip_prefix(root).unwrap().to_str().unwrap();
        entries.push_str(&format!("({relative:?}, include_bytes!({path:?})),\n"));
    }
}

fn main() {
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = Path::new(&root);
    let mut entries = String::from("&[\n");
    for input in [
        "tech/sky130/gds",
        "tech/sky130/spice",
        "tech/sky130/LICENSE.skywater",
        "tech/sky130/LICENSE.sky130_fd_sc_hd",
        "tech/sky130/LICENSE.sky130_fd_sc_hs",
        "tech/sky130/AUTHORS.skywater",
        "templates",
    ] {
        println!("cargo:rerun-if-changed={input}");
        collect(root, &root.join(input), &mut entries);
    }
    entries.push_str("]\n");
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("assets.rs"),
        entries,
    )
    .unwrap();
}
