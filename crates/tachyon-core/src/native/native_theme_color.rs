#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct NativeThemeColor {
    pub(super) red: u8,
    pub(super) green: u8,
    pub(super) blue: u8,
}

impl NativeThemeColor {
    const LIGHT_THRESHOLD: u32 = 128_000;
    const CHANNEL_SCALES: [u8; 2] = [17, 1];

    pub(super) fn from_manifest(value: Option<&serde_json::Value>) -> Option<Self> {
        let digits = value?.as_str()?.trim().strip_prefix('#')?;
        let hexadecimal = digits.bytes().all(|byte| byte.is_ascii_hexdigit());
        Self::from_digits(hexadecimal.then_some(digits)?)
    }

    fn from_digits(digits: &str) -> Option<Self> {
        let width = Self::channel_width(digits.len())?;
        let scale = Self::CHANNEL_SCALES[width - 1];
        let channel = |index: usize| {
            let start = index * width;
            let value = u8::from_str_radix(digits.get(start..start + width)?, 16).ok()?;
            Some(value * scale)
        };
        Some(Self {
            red: channel(0)?,
            green: channel(1)?,
            blue: channel(2)?,
        })
    }

    fn channel_width(length: usize) -> Option<usize> {
        match length {
            3 => Some(1),
            6 => Some(2),
            _ => None,
        }
    }

    pub(super) fn is_light(self) -> bool {
        let weighted =
            299 * u32::from(self.red) + 587 * u32::from(self.green) + 114 * u32::from(self.blue);
        weighted >= Self::LIGHT_THRESHOLD
    }
}

#[cfg(test)]
mod tests {
    use super::NativeThemeColor;
    use serde_json::json;

    fn parsed(value: &serde_json::Value) -> Option<NativeThemeColor> {
        NativeThemeColor::from_manifest(Some(value))
    }

    #[test]
    fn short_and_long_hex_colors_become_channels() {
        assert_eq!(
            parsed(&json!(" #0F6cbd ")),
            Some(NativeThemeColor {
                red: 0x0f,
                green: 0x6c,
                blue: 0xbd
            })
        );
        assert_eq!(
            parsed(&json!("#fa0")),
            Some(NativeThemeColor {
                red: 0xff,
                green: 0xaa,
                blue: 0x00
            })
        );
    }

    #[test]
    fn other_color_forms_and_values_give_no_native_color() {
        for value in [
            json!("rebeccapurple"),
            json!("rgb(1, 2, 3)"),
            json!("#12345"),
            json!("#11223344"),
            json!("#ggg"),
            json!("#"),
            json!(""),
            json!(12),
            json!(null),
        ] {
            assert_eq!(parsed(&value), None, "{value}");
        }
        assert_eq!(NativeThemeColor::from_manifest(None), None);
    }

    #[test]
    fn light_colors_take_dark_caption_text() {
        assert!(parsed(&json!("#ffffff")).is_some_and(NativeThemeColor::is_light));
        assert!(parsed(&json!("#ffd700")).is_some_and(NativeThemeColor::is_light));
        assert!(!parsed(&json!("#000000")).is_some_and(NativeThemeColor::is_light));
        assert!(!parsed(&json!("#0f6cbd")).is_some_and(NativeThemeColor::is_light));
    }
}
