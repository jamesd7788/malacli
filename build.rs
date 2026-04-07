use flate2::Compression;
use flate2::write::GzEncoder;
use std::io::Write;
use std::{env, fs, path::Path};

fn compress(src: &Path, dst: &Path) {
    let data = fs::read(src).unwrap_or_else(|e| panic!("failed to read {}: {e}", src.display()));
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&data).unwrap();
    let compressed = encoder.finish().unwrap();
    fs::write(dst, compressed).unwrap();
    println!("cargo::rerun-if-changed={}", src.display());
}

fn main() {
    let out = env::var("OUT_DIR").unwrap();
    let out = Path::new(&out);

    compress(
        Path::new("data/raw/eng-kjv.osis.xml"),
        &out.join("kjv.xml.gz"),
    );
    compress(
        Path::new("data/raw/cross_references.txt"),
        &out.join("cross_references.txt.gz"),
    );
}
