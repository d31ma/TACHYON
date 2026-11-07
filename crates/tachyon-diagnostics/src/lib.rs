use core::fmt;
use core::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DiagnosticCode([u8; 4]);

impl DiagnosticCode {
    pub const PREFIX: &str = "TY";

    #[must_use]
    pub const fn from_number(number: u16) -> Option<Self> {
        if number > 9_999 {
            return None;
        }

        Some(Self([
            b'0' + ((number / 1_000) % 10) as u8,
            b'0' + ((number / 100) % 10) as u8,
            b'0' + ((number / 10) % 10) as u8,
            b'0' + (number % 10) as u8,
        ]))
    }

    #[must_use]
    pub const fn number(self) -> u16 {
        ((self.0[0] - b'0') as u16 * 1_000)
            + ((self.0[1] - b'0') as u16 * 100)
            + ((self.0[2] - b'0') as u16 * 10)
            + (self.0[3] - b'0') as u16
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = core::str::from_utf8(&self.0).map_err(|_| fmt::Error)?;
        write!(formatter, "{}{digits}", Self::PREFIX)
    }
}

impl FromStr for DiagnosticCode {
    type Err = InvalidDiagnosticCode;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let bytes = value.as_bytes();
        if bytes.len() != 6 || &bytes[..2] != Self::PREFIX.as_bytes() {
            return Err(InvalidDiagnosticCode);
        }

        let digits: [u8; 4] = bytes[2..].try_into().map_err(|_| InvalidDiagnosticCode)?;
        if !digits.iter().all(u8::is_ascii_digit) {
            return Err(InvalidDiagnosticCode);
        }

        Ok(Self(digits))
    }
}

impl From<DiagnosticCode> for String {
    fn from(code: DiagnosticCode) -> Self {
        code.to_string()
    }
}

impl TryFrom<String> for DiagnosticCode {
    type Error = InvalidDiagnosticCode;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidDiagnosticCode;

impl fmt::Display for InvalidDiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("diagnostic code must match TY followed by four ASCII digits")
    }
}

impl std::error::Error for InvalidDiagnosticCode {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Information,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub file: String,
    pub start: u64,
    pub end: u64,
}

impl SourceSpan {
    pub fn new(file: impl Into<String>, start: u64, end: u64) -> Result<Self, InvalidSourceSpan> {
        let file = file.into();
        if file.is_empty() || file.starts_with('/') || file.contains('\\') || start > end {
            return Err(InvalidSourceSpan);
        }

        Ok(Self { file, start, end })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidSourceSpan;

impl fmt::Display for InvalidSourceSpan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("source span needs a project-relative forward-slash path and start <= end")
    }
}

impl std::error::Error for InvalidSourceSpan {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    pub spans: Vec<SourceSpan>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub contract_version: u8,
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticReport {
    #[must_use]
    pub const fn v1(diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            contract_version: 1,
            diagnostics,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, DiagnosticCode, DiagnosticReport, Severity, SourceSpan};

    #[test]
    fn diagnostic_codes_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let code: DiagnosticCode = "TY0042".parse()?;
        assert_eq!(code.number(), 42);
        assert_eq!(code.to_string(), "TY0042");
        Ok(())
    }

    #[test]
    fn diagnostic_codes_reject_unstable_shapes() {
        for invalid in ["42", "TY42", "ty0042", "TY-042", "TY00420", "TY00A2"] {
            assert!(invalid.parse::<DiagnosticCode>().is_err(), "{invalid}");
        }
    }

    #[test]
    fn diagnostic_codes_construct_and_serialize() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            DiagnosticCode::from_number(7).map(DiagnosticCode::number),
            Some(7)
        );
        assert_eq!(
            DiagnosticCode::from_number(9_999).map(String::from),
            Some(String::from("TY9999"))
        );
        assert_eq!(DiagnosticCode::from_number(10_000), None);

        let serialized = serde_json::to_string(&"TY0042".parse::<DiagnosticCode>()?)?;
        assert_eq!(serialized, r#""TY0042""#);
        let deserialized: DiagnosticCode = serde_json::from_str(&serialized)?;
        assert_eq!(deserialized.number(), 42);
        Ok(())
    }

    #[test]
    fn validation_errors_are_actionable() {
        let code_error = "bad"
            .parse::<DiagnosticCode>()
            .err()
            .map(|error| error.to_string());
        assert_eq!(
            code_error.as_deref(),
            Some("diagnostic code must match TY followed by four ASCII digits")
        );

        let span_error = SourceSpan::new("", 0, 0)
            .err()
            .map(|error| error.to_string());
        assert_eq!(
            span_error.as_deref(),
            Some("source span needs a project-relative forward-slash path and start <= end")
        );
    }

    #[test]
    fn source_spans_require_portable_relative_paths() {
        assert!(SourceSpan::new("client/pages/tac.html", 4, 8).is_ok());
        assert!(SourceSpan::new("", 4, 8).is_err());
        assert!(SourceSpan::new("/client/pages/tac.html", 4, 8).is_err());
        assert!(SourceSpan::new(r"client\pages\tac.html", 4, 8).is_err());
        assert!(SourceSpan::new("client/pages/tac.html", 8, 4).is_err());
    }

    #[test]
    fn complete_diagnostics_have_a_stable_wire_shape() -> Result<(), Box<dyn std::error::Error>> {
        let diagnostic = Diagnostic {
            code: "TY0042".parse()?,
            severity: Severity::Warning,
            message: String::from("A fallback is required."),
            help: Some(String::from("Register a native adapter.")),
            spans: vec![SourceSpan::new("client/pages/tac.html", 4, 8)?],
        };

        let serialized = serde_json::to_string(&diagnostic)?;
        let decoded: Diagnostic = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, diagnostic);
        assert!(serde_json::to_string(&Severity::Error)?.contains("error"));
        assert!(serde_json::to_string(&Severity::Information)?.contains("information"));
        let report = DiagnosticReport::v1(vec![diagnostic]);
        assert_eq!(serde_json::to_value(report)?["contract_version"], 1);
        Ok(())
    }
}
