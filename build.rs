use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

fn files(root: &Path, dir: &Path, inputs: bool, result: &mut BTreeMap<String, String>) {
    for item in fs::read_dir(dir).expect("UI directory missing; run node scripts/build.mjs") {
        let item = item.unwrap();
        let name = item.file_name().to_string_lossy().into_owned();
        if inputs
            && (matches!(
                name.as_str(),
                "dist"
                    | "node_modules"
                    | ".nuxt"
                    | ".output"
                    | ".cache"
                    | "test-results"
                    | "playwright-report"
                    | ".DS_Store"
            ) || name.starts_with(".env")
                || name.ends_with(".log"))
        {
            continue;
        }
        let path = item.path();
        assert!(
            !item.file_type().unwrap().is_symlink(),
            "UI symlinks are unsupported"
        );
        if path.is_dir() {
            files(root, &path, inputs, result);
        } else {
            println!("cargo:rerun-if-changed={}", path.display());
            let key = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .replace('\\', "/");
            result.insert(
                key,
                format!("{:x}", Sha256::digest(fs::read(&path).unwrap())),
            );
        }
    }
}
fn main() {
    // Watch the input root as well as existing files: newly added Nuxt config,
    // public assets or directories must invalidate an earlier successful build.
    println!("cargo:rerun-if-changed=frontend");
    println!("cargo:rerun-if-changed=frontend/app");
    println!("cargo:rerun-if-changed=frontend/tests");
    println!("cargo:rerun-if-changed=frontend/.output");
    println!("cargo:rerun-if-changed=build.rs");
    let manifest_path = "frontend/.output/ui-manifest.json";
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(manifest_path).expect("UI manifest missing: run node scripts/build.mjs"),
    )
    .unwrap();
    assert_eq!(manifest["version"], 1, "Unsupported UI manifest");
    assert!(
        Path::new("frontend/.output/public/index.html").is_file(),
        "UI index.html missing"
    );
    for (key, directory, inputs) in [
        ("inputs", "frontend", true),
        ("outputs", "frontend/.output/public", false),
    ] {
        let mut actual = BTreeMap::new();
        files(
            Path::new(directory),
            Path::new(directory),
            inputs,
            &mut actual,
        );
        let expected: BTreeMap<String, String> =
            serde_json::from_value(manifest[key].clone()).unwrap();
        assert_eq!(
            actual, expected,
            "UI {key} changed: run node scripts/build.mjs again"
        );
    }
}
