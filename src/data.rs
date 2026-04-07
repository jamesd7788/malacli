use flate2::read::GzDecoder;
use std::io::Read;
use std::sync::OnceLock;

static KJV_XML: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/kjv.xml.gz"));
static CROSS_REFS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cross_references.txt.gz"));

static KJV_TEXT: OnceLock<String> = OnceLock::new();
static CROSS_REFS_TEXT: OnceLock<String> = OnceLock::new();

fn decompress(compressed: &[u8]) -> String {
    let mut decoder = GzDecoder::new(compressed);
    let mut text = String::new();
    decoder
        .read_to_string(&mut text)
        .expect("failed to decompress embedded data");
    text
}

pub fn kjv_xml() -> &'static str {
    KJV_TEXT.get_or_init(|| decompress(KJV_XML))
}

pub fn cross_references() -> &'static str {
    CROSS_REFS_TEXT.get_or_init(|| decompress(CROSS_REFS))
}
