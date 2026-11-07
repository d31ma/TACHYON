use super::FrameLimits;
use crate::HandlerSource;
use std::path::Path;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::handler) enum FramePrefix {
    #[default]
    Binary,
    Hex,
}

impl FramePrefix {
    const HEX_WIDTH: usize = 8;
    const HEX_RADIX: u32 = 16;

    pub(in crate::handler) fn of(source: &HandlerSource) -> Self {
        let lua = crate::lua_source::LuaSource::is_lua(Path::new(source.relative_path()));
        [Self::Binary, Self::Hex][usize::from(lua)]
    }

    pub(in crate::handler) fn width(self) -> usize {
        [FrameLimits::PREFIX_BYTES, Self::HEX_WIDTH][self as usize]
    }

    pub(in crate::handler) fn encode(self, length: usize) -> Vec<u8> {
        let binary = u32::try_from(length).unwrap_or(u32::MAX).to_be_bytes();
        [binary.to_vec(), format!("{length:08x}").into_bytes()][self as usize].clone()
    }

    pub(in crate::handler) fn decode(self, prefix: &[u8]) -> Option<usize> {
        [Self::binary_length(prefix), Self::hex_length(prefix)][self as usize]
    }

    pub(in crate::handler) fn reframed(self, binary_frame: &[u8]) -> Vec<u8> {
        let payload = binary_frame
            .get(FrameLimits::PREFIX_BYTES..)
            .unwrap_or_default();
        [self.encode(payload.len()), payload.to_vec()].concat()
    }

    fn binary_length(prefix: &[u8]) -> Option<usize> {
        let bytes = <[u8; FrameLimits::PREFIX_BYTES]>::try_from(prefix).ok()?;
        usize::try_from(u32::from_be_bytes(bytes)).ok()
    }

    fn hex_length(prefix: &[u8]) -> Option<usize> {
        let text = std::str::from_utf8(prefix).ok()?;
        let digits = text.bytes().all(|byte| byte.is_ascii_hexdigit());
        digits
            .then(|| usize::from_str_radix(text, Self::HEX_RADIX).ok())
            .flatten()
            .filter(|_| prefix.len() == Self::HEX_WIDTH)
    }
}

#[cfg(test)]
mod tests {
    use super::FramePrefix;

    #[test]
    fn hex_prefixes_never_hold_the_bytes_windows_text_mode_changes() {
        for length in [0x0a, 0x0d, 0x1a, 0x0a0d, 0x1a1a_0a0d, 26, 282] {
            let prefix = FramePrefix::Hex.encode(length);
            assert_eq!(prefix.len(), FramePrefix::Hex.width());
            assert!(prefix.iter().all(u8::is_ascii_hexdigit), "{prefix:?}");
            assert_eq!(FramePrefix::Hex.decode(&prefix), Some(length));
        }
        assert_eq!(FramePrefix::Hex.encode(26), b"0000001a");
        assert_eq!(FramePrefix::Hex.decode(b"0000001g"), None);
        assert_eq!(FramePrefix::Hex.decode(b"+000001a"), None);
        assert_eq!(FramePrefix::Hex.decode(b"001a"), None);
    }

    #[test]
    fn binary_prefixes_keep_the_four_byte_big_endian_length() {
        assert_eq!(FramePrefix::Binary.encode(26), [0, 0, 0, 0x1a]);
        assert_eq!(FramePrefix::Binary.decode(&[0, 0, 1, 0x1a]), Some(282));
        assert_eq!(FramePrefix::Binary.decode(&[0, 1]), None);
        assert_eq!(FramePrefix::Binary.width(), 4);
    }

    #[test]
    fn a_binary_frame_is_reframed_with_the_same_payload() {
        let binary = [0, 0, 0, 2, b'{', b'}'];
        assert_eq!(FramePrefix::Hex.reframed(&binary), b"00000002{}");
        assert_eq!(FramePrefix::Binary.reframed(&binary), binary);
    }
}
