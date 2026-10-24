mod frame_limits;
mod frame_prefix;

pub(super) use frame_limits::FrameLimits;
pub(super) use frame_prefix::FramePrefix;

use crate::Failure;
use crate::failure::diagnostic;
use serde::Serialize;
use std::cmp::Ordering;
use tachyon_contracts::{
    HandlerBody, HandlerCancel, HandlerHeaders, HandlerProtocolError, HandlerRequest,
    HandlerResponse,
};

pub(crate) struct HandlerFrame;

impl HandlerFrame {
    const HEADER_NAME_SYMBOLS: &'static [u8] = b"!#$%&'*+-.^_`|~";
    const IDENTIFIER_SYMBOLS: &'static [u8] = b"_-";
    const OPERATION_SYMBOLS: &'static [u8] = b"_.-";

    pub(in crate::handler) fn request(request: &HandlerRequest) -> Result<Vec<u8>, Failure> {
        Self::check_request(request)?;
        Self::encode(request)
    }

    pub(in crate::handler) fn cancel(request_id: &str) -> Result<Vec<u8>, Failure> {
        Self::encode(&HandlerCancel::v1(request_id))
    }

    pub(crate) fn response(bytes: &[u8], request_id: &str) -> Result<HandlerResponse, Failure> {
        Self::check_prefix(bytes)?;
        let length = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        Self::check_response_length(length)?;
        let expected = FrameLimits::PREFIX_BYTES.saturating_add(length);
        Self::check_exact_size(bytes.len(), expected)?;
        let json = std::str::from_utf8(&bytes[FrameLimits::PREFIX_BYTES..]).map_err(|_| {
            Self::protocol_failure(2102, "Handler response frame is not valid UTF-8.")
        })?;
        let response = serde_json::from_str::<HandlerResponse>(json).map_err(|_| {
            Self::protocol_failure(
                2102,
                "Handler response is not a valid Handler Protocol v1 response.",
            )
        })?;
        Self::check_response(&response, request_id)?;
        Ok(response)
    }

    pub(in crate::handler) fn protocol_failure(number: u16, message: &str) -> Failure {
        Failure::one(diagnostic(
            number,
            message,
            Some(String::from(
                "Fix or replace the handler adapter before retrying.",
            )),
            None,
        ))
    }

    fn check_prefix(bytes: &[u8]) -> Result<(), Failure> {
        if bytes.len() < FrameLimits::PREFIX_BYTES {
            return Err(Self::protocol_failure(
                2102,
                "Handler stdout ended before a frame prefix was received.",
            ));
        }
        Ok(())
    }

    fn check_response_length(length: usize) -> Result<(), Failure> {
        if length > FrameLimits::MAX_BYTES {
            return Err(Self::protocol_failure(
                2103,
                "Handler response frame exceeds the 16 MiB protocol limit.",
            ));
        }
        Ok(())
    }

    fn check_exact_size(actual: usize, expected: usize) -> Result<(), Failure> {
        match actual.cmp(&expected) {
            Ordering::Equal => Ok(()),
            Ordering::Greater => Err(Self::protocol_failure(
                2102,
                "Handler wrote trailing or multiple data to protocol stdout.",
            )),
            Ordering::Less => Err(Self::protocol_failure(
                2102,
                "Handler stdout ended before the declared frame was complete.",
            )),
        }
    }

    fn encode<T: Serialize>(envelope: &T) -> Result<Vec<u8>, Failure> {
        let json = serde_json::to_vec(envelope).map_err(|_| {
            Failure::one(diagnostic(
                2005,
                "Handler request cannot be serialized.",
                Some(String::from(
                    "Use Handler Protocol v1-compatible request values.",
                )),
                None,
            ))
        })?;
        Self::check_request_size(&json)?;
        let length = u32::try_from(json.len()).map_err(|_| {
            Failure::one(diagnostic(
                2005,
                "Handler request frame length cannot be represented.",
                None,
                None,
            ))
        })?;
        let mut frame = Vec::with_capacity(FrameLimits::PREFIX_BYTES + json.len());
        frame.extend_from_slice(&length.to_be_bytes());
        frame.extend_from_slice(&json);
        Ok(frame)
    }

    fn check_request_size(json: &[u8]) -> Result<(), Failure> {
        if json.len() > FrameLimits::MAX_BYTES {
            return Err(Failure::one(diagnostic(
                2005,
                "Handler request frame exceeds the 16 MiB protocol limit.",
                Some(String::from("Reduce request headers or body size.")),
                None,
            )));
        }
        Ok(())
    }

    fn check_request(request: &HandlerRequest) -> Result<(), Failure> {
        if !Self::valid_request(request) {
            return Err(Failure::one(diagnostic(
                2005,
                "Handler request does not satisfy Handler Protocol v1.",
                Some(String::from(
                    "Check the request ID, operation, route, deadline, headers, and body limits.",
                )),
                None,
            )));
        }
        Ok(())
    }

    fn valid_request(request: &HandlerRequest) -> bool {
        Self::all_pass(&[
            request.protocol_version == 1,
            Self::valid_identifier(&request.request_id),
            Self::valid_operation(&request.operation),
            Self::valid_route(&request.route),
            request
                .deadline_ms
                .is_none_or(|deadline| (1..=300_000).contains(&deadline)),
            Self::valid_headers(&request.headers),
            request.body.as_ref().is_none_or(Self::valid_body),
        ])
    }

    fn check_response(response: &HandlerResponse, request_id: &str) -> Result<(), Failure> {
        if !Self::valid_response(response, request_id) {
            return Err(Self::protocol_failure(
                2102,
                "Handler response violates Handler Protocol v1 or does not match the request.",
            ));
        }
        Ok(())
    }

    fn valid_response(response: &HandlerResponse, request_id: &str) -> bool {
        Self::all_pass(&[
            response.protocol_version == 1,
            response.request_id == request_id,
            Self::valid_identifier(&response.request_id),
            (100..=599).contains(&response.status),
            Self::valid_headers(&response.headers),
            response.body.as_ref().is_none_or(Self::valid_body),
            response.body.is_some() != response.error.is_some(),
            response.error.as_ref().is_none_or(Self::valid_error),
            response
                .error
                .as_ref()
                .is_none_or(|error| Self::valid_rejection(response.status, error)),
        ])
    }

    fn valid_rejection(status: u16, error: &HandlerProtocolError) -> bool {
        if !error.is_check_rejection() {
            return error.violations.is_empty();
        }
        Self::valid_check_rejection(status, error)
    }

    fn valid_check_rejection(status: u16, error: &HandlerProtocolError) -> bool {
        Self::all_pass(&[
            status == crate::check_rejection::CheckProblem::STATUS,
            crate::check_rejection::CheckProblem::accepts(&error.violations),
        ])
    }

    fn valid_error(error: &HandlerProtocolError) -> bool {
        Self::all_pass(&[
            Self::valid_code(&error.code),
            !error.message.is_empty(),
            error.message.chars().count() <= 2_048,
        ])
    }

    fn valid_identifier(value: &str) -> bool {
        (1..=128).contains(&value.len()) && value.bytes().all(Self::identifier_byte)
    }

    fn identifier_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || Self::IDENTIFIER_SYMBOLS.contains(&byte)
    }

    fn valid_operation(value: &str) -> bool {
        let mut bytes = value.bytes();
        bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
            && value.len() <= 128
            && bytes.all(Self::operation_byte)
    }

    fn operation_byte(byte: u8) -> bool {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || Self::OPERATION_SYMBOLS.contains(&byte)
    }

    fn valid_route(value: &str) -> bool {
        value.starts_with('/') && value.chars().count() <= 2_048
    }

    fn valid_headers(headers: &HandlerHeaders) -> bool {
        Self::all_pass(&[
            headers.len() <= 128,
            headers
                .iter()
                .all(|(name, values)| Self::valid_header(name, values)),
        ])
    }

    fn valid_header(name: &str, values: &[String]) -> bool {
        Self::all_pass(&[
            Self::valid_header_name(name),
            values.len() <= 64,
            values.iter().all(|value| value.chars().count() <= 8_192),
        ])
    }

    fn valid_header_name(name: &str) -> bool {
        !name.is_empty() && name.bytes().all(Self::header_name_byte)
    }

    fn header_name_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || Self::HEADER_NAME_SYMBOLS.contains(&byte)
    }

    fn valid_body(body: &HandlerBody) -> bool {
        body.data.chars().count() <= FrameLimits::MAX_BYTES
    }

    fn valid_code(code: &str) -> bool {
        code.len() == 6
            && code.starts_with("TY")
            && code[2..].bytes().all(|byte| byte.is_ascii_digit())
    }

    fn all_pass(checks: &[bool]) -> bool {
        !checks.contains(&false)
    }
}

#[cfg(test)]
mod tests {
    use super::{FrameLimits, HandlerFrame};
    use tachyon_contracts::{
        HandlerBody, HandlerBodyEncoding, HandlerHeaders, HandlerRequest, HandlerResponse,
        HttpMethod,
    };

    #[test]
    fn frames_round_trip_and_reject_trailing_or_mismatched_output() {
        let request = HandlerRequest::route("req_1", "/", HttpMethod::Get);
        let request_bytes = HandlerFrame::request(&request).unwrap_or_else(|_| unreachable!());
        assert_eq!(
            u32::from_be_bytes([
                request_bytes[0],
                request_bytes[1],
                request_bytes[2],
                request_bytes[3]
            ]) as usize,
            request_bytes.len() - 4
        );

        let response = HandlerResponse::success(
            "req_1",
            200,
            HandlerHeaders::new(),
            HandlerBody {
                encoding: HandlerBodyEncoding::Utf8,
                data: String::from("ok"),
            },
        );
        let json = serde_json::to_vec(&response).unwrap_or_else(|_| unreachable!());
        let length = u32::try_from(json.len()).unwrap_or_else(|_| unreachable!());
        let mut frame = Vec::from(length.to_be_bytes());
        frame.extend_from_slice(&json);
        assert_eq!(
            HandlerFrame::response(&frame, "req_1").unwrap_or_else(|_| unreachable!()),
            response
        );
        frame.push(0);
        assert!(HandlerFrame::response(&frame, "req_1").is_err());
        frame.pop();
        assert!(HandlerFrame::response(&frame, "other").is_err());
    }

    #[test]
    fn invalid_requests_and_frames_fail_closed() {
        let mut request = HandlerRequest::route("", "missing-slash", HttpMethod::Get);
        request.deadline_ms = Some(0);
        assert!(HandlerFrame::request(&request).is_err());

        assert!(HandlerFrame::response(&[], "req").is_err());
        assert!(HandlerFrame::response(&[0, 0, 0, 4, b'{', b'}'], "req").is_err());
        assert!(HandlerFrame::response(&[0, 0, 0, 3, 0xff, 0xff, 0xff], "req").is_err());
        assert!(HandlerFrame::response(&[0, 0, 0, 3, b'x', b'x', b'x'], "req").is_err());
        let oversized = u32::try_from(FrameLimits::MAX_BYTES + 1)
            .unwrap_or_else(|_| unreachable!())
            .to_be_bytes();
        assert!(HandlerFrame::response(&oversized, "req").is_err());
        assert!(HandlerFrame::cancel(&"x".repeat(FrameLimits::MAX_BYTES + 1)).is_err());
    }

    #[test]
    fn invalid_response_fields_are_rejected_after_deserialization() {
        let baseline = HandlerResponse::success(
            "req",
            200,
            HandlerHeaders::new(),
            HandlerBody {
                encoding: HandlerBodyEncoding::Utf8,
                data: String::from("ok"),
            },
        );
        let mut invalid = Vec::new();
        let mut protocol = baseline.clone();
        protocol.protocol_version = 2;
        invalid.push(protocol);
        let mut status = baseline.clone();
        status.status = 99;
        invalid.push(status);
        let mut header = baseline.clone();
        header
            .headers
            .insert(String::from("bad header"), vec![String::new()]);
        invalid.push(header);
        let mut neither = baseline;
        neither.body = None;
        invalid.push(neither);

        for response in invalid {
            let json = serde_json::to_vec(&response).unwrap_or_else(|_| unreachable!());
            let length = u32::try_from(json.len()).unwrap_or_else(|_| unreachable!());
            let mut frame = Vec::from(length.to_be_bytes());
            frame.extend_from_slice(&json);
            assert!(HandlerFrame::response(&frame, "req").is_err());
        }
    }

    fn framed(response: &HandlerResponse) -> Vec<u8> {
        let json = serde_json::to_vec(response).unwrap_or_else(|_| unreachable!());
        let length = u32::try_from(json.len()).unwrap_or_else(|_| unreachable!());
        let mut frame = Vec::from(length.to_be_bytes());
        frame.extend_from_slice(&json);
        frame
    }

    #[test]
    fn check_rejection_frames_must_be_well_formed() {
        use tachyon_contracts::{HandlerProtocolError, HandlerViolation};
        let violations = vec![HandlerViolation::new("items[2].name", "NotEmpty")];
        let accepted = HandlerResponse::error(
            "req",
            422,
            HandlerProtocolError::check_rejected(violations.clone()),
        );
        assert!(HandlerFrame::response(&framed(&accepted), "req").is_ok());
        let wrong_status = HandlerResponse::error(
            "req",
            500,
            HandlerProtocolError::check_rejected(violations.clone()),
        );
        assert!(HandlerFrame::response(&framed(&wrong_status), "req").is_err());
        let empty =
            HandlerResponse::error("req", 422, HandlerProtocolError::check_rejected(Vec::new()));
        assert!(HandlerFrame::response(&framed(&empty), "req").is_err());
        let mut other_code = HandlerProtocolError::new("TY2201", "Handler failed.");
        other_code.violations = violations;
        let smuggled = HandlerResponse::error("req", 500, other_code);
        assert!(HandlerFrame::response(&framed(&smuggled), "req").is_err());
        let echoed = HandlerResponse::error(
            "req",
            422,
            HandlerProtocolError::check_rejected(vec![HandlerViolation::new(
                "secret value",
                "Required",
            )]),
        );
        assert!(HandlerFrame::response(&framed(&echoed), "req").is_err());
    }
}
