#![allow(clippy::expect_used)]

use super::WebEngine;
use crate::compiler::ContentDigest;
use crate::node_driver::NodeDriver;

struct RustEmbedding;

impl RustEmbedding {
    const PLACEHOLDER: &str = "__WASM__";
    const DECODER_START: &str = "const wasmText";
    const DECODER_END: &str = "const { instance }";
    const SAMPLE: [u8; 10] = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0xff, 0x0a, 0x27, 0x5c];

    fn node_available() -> bool {
        NodeDriver::available()
    }

    fn decoder(bytes: &[u8]) -> String {
        let start = WebEngine::RUST_INSTANCE
            .find(Self::DECODER_START)
            .expect("the engine names its embedded module text");
        let end = WebEngine::RUST_INSTANCE
            .find(Self::DECODER_END)
            .expect("the engine instantiates the module");
        WebEngine::RUST_INSTANCE[start..end].replace(Self::PLACEHOLDER, &ContentDigest::hex(bytes))
    }
}

#[test]
fn the_rust_module_is_embedded_as_hexadecimal_text_and_decodes_to_the_same_bytes() {
    if !RustEmbedding::node_available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let program = format!(
        "{}console.log(JSON.stringify([...binary]))",
        RustEmbedding::decoder(&RustEmbedding::SAMPLE)
    );
    let output =
        NodeDriver::output(NodeDriver::command().args(["--input-type=module", "--eval", &program]))
            .expect("node");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let decoded: Vec<u8> = serde_json::from_slice(&output.stdout).expect("decoded bytes");
    assert_eq!(decoded, RustEmbedding::SAMPLE);
}
