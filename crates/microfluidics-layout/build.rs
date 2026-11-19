//! Bundle the processes: every file in the workspace's `processes/` directory is built into the
//! crate, so a binary has them without the repository.

// A build script stops the build when the library's files cannot be read.
#![allow(clippy::unwrap_used, clippy::panic)]

use std::{env, fs, path::Path};

fn main() {
    let root = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let mut source = String::new();
    for (constant, folder, extension) in [("PROCESSES", "processes", "toml")] {
        let dir = root.join(folder);
        println!("cargo:rerun-if-changed={}", dir.display());
        let mut files: Vec<_> = fs::read_dir(&dir)
            .unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|found| found == extension))
            .collect();
        files.sort();
        source.push_str(&format!("pub(crate) const {constant}: &[(&str, &str)] = &[\n"));
        for file in files {
            let name = file.file_name().unwrap().to_string_lossy().into_owned();
            let path = file.canonicalize().unwrap();
            source.push_str(&format!(
                "    ({name:?}, include_str!({:?})),\n",
                path.display().to_string()
            ));
        }
        source.push_str("];\n");
    }
    fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("bundled.rs"), source).unwrap();
}
