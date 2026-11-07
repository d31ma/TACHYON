use crate::BrotliEncoder;
use std::io::Read;

struct Sample;

impl Sample {
    const TEXT: &'static str = "Tachyon compresses every bundle file with Brotli. ";

    fn bytes() -> Vec<u8> {
        Self::TEXT.repeat(200).into_bytes()
    }

    fn decoded(encoded: &[u8]) -> Vec<u8> {
        let mut decoded = Vec::new();
        let read = brotli::Decompressor::new(encoded, 4096).read_to_end(&mut decoded);
        assert!(read.is_ok(), "the encoded bytes decode");
        decoded
    }
}

#[test]
fn compressed_bytes_decode_to_the_original_bytes() {
    let source = Sample::bytes();
    let encoded = BrotliEncoder::new(11, 22).compress(&source);
    let encoded = encoded.unwrap_or_default();
    assert!(encoded.len() < source.len() / 10);
    assert_eq!(Sample::decoded(&encoded), source);
}

#[test]
fn the_same_input_gives_the_same_bytes_every_time() {
    let encoder = BrotliEncoder::new(11, 22);
    let first = encoder.compress(&Sample::bytes()).unwrap_or_default();
    let second = encoder.compress(&Sample::bytes()).unwrap_or_default();
    assert!(!first.is_empty());
    assert_eq!(first, second);
}
