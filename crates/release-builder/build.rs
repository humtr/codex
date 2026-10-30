use std::{env, fs, path::PathBuf};

fn append(output: &mut Vec<u8>, name: &str, bytes: &[u8]) {
    output.extend_from_slice(&(name.len() as u64).to_be_bytes());
    output.extend_from_slice(name.as_bytes());
    output.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    output.extend_from_slice(bytes);
}

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let manifest = root.join("release/runtime-build-inputs-v1");
    println!("cargo:rerun-if-changed={}", manifest.display());
    let inventory = fs::read_to_string(&manifest).expect("runtime input inventory");
    assert_eq!(
        inventory.lines().next(),
        Some("codex-runtime-build-inputs-v1")
    );
    let mut result = b"codex-local-runtime-inputs-v1\n".to_vec();
    for line in inventory.lines().skip(1) {
        let (role, name) = line.split_once('\t').expect("runtime input record");
        if role != "recipe" {
            continue;
        }
        let path = root.join(name);
        println!("cargo:rerun-if-changed={}", path.display());
        append(
            &mut result,
            name,
            &fs::read(path).expect("runtime producer input"),
        );
    }
    // This local identity is conservative: the complete wrapper lock also binds
    // compiler-side parser dependencies. Remote component reuse retains its
    // existing exact dependency-closure fingerprint.
    let lock = root.join("Cargo.lock");
    println!("cargo:rerun-if-changed={}", lock.display());
    append(
        &mut result,
        "Cargo.lock",
        &fs::read(lock).expect("producer lock"),
    );
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("runtime-inputs"),
        result,
    )
    .expect("embedded local runtime inputs");
}
