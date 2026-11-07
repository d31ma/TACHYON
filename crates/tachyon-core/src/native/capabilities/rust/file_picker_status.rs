#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilePickerStatus {
    Available,
    Selected,
    Cancelled,
    Read,
    Released,
    Denied,
    Unsupported,
    Unavailable,
    Invalid,
    Disposed,
    TooLarge,
}

impl FilePickerStatus {
    pub fn as_str(self) -> &'static str {
        self.spelling()
    }

    fn spelling(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Selected => "selected",
            Self::Cancelled => "cancelled",
            Self::Read => "read",
            Self::Released => "released",
            Self::Denied => "denied",
            Self::Unsupported => "unsupported",
            Self::Unavailable => "unavailable",
            Self::Invalid => "invalid",
            Self::Disposed => "disposed",
            Self::TooLarge => "tooLarge",
        }
    }

    fn read(value: &str) -> Self {
        match value {
            "available" => Self::Available,
            "selected" => Self::Selected,
            "cancelled" => Self::Cancelled,
            "read" => Self::Read,
            "released" => Self::Released,
            "denied" => Self::Denied,
            "unsupported" => Self::Unsupported,
            "invalid" => Self::Invalid,
            "disposed" => Self::Disposed,
            "tooLarge" => Self::TooLarge,
            _ => Self::Unavailable,
        }
    }
}
