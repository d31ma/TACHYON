use super::ResourceAccess;
use axum::http::{HeaderMap, StatusCode, header};

pub(super) struct ResourceRequest;

impl ResourceRequest {
    const REQUEST_LINE: &str = "POST / HTTP/1.1";

    pub(super) fn headers(bytes: &[u8]) -> Result<(HeaderMap, usize), StatusCode> {
        let text = std::str::from_utf8(bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
        let mut lines = text.split("\r\n");
        Self::check_request_line(lines.next())?;
        let headers = Self::header_map(lines)?;
        Self::check_no_transfer_encoding(&headers)?;
        let length = Self::declared_content_length(&headers).ok_or(StatusCode::LENGTH_REQUIRED)?;
        Self::check_length(length)?;
        Ok((headers, length))
    }

    fn check_request_line(line: Option<&str>) -> Result<(), StatusCode> {
        if line != Some(Self::REQUEST_LINE) {
            return Err(StatusCode::BAD_REQUEST);
        }
        Ok(())
    }

    fn header_map<'a>(lines: impl Iterator<Item = &'a str>) -> Result<HeaderMap, StatusCode> {
        let mut headers = HeaderMap::new();
        for line in lines.take_while(|line| !line.is_empty()) {
            Self::insert_header(&mut headers, line)?;
        }
        Ok(headers)
    }

    fn insert_header(headers: &mut HeaderMap, line: &str) -> Result<(), StatusCode> {
        let (name, value) = line.split_once(':').ok_or(StatusCode::BAD_REQUEST)?;
        let name =
            header::HeaderName::from_bytes(name.as_bytes()).map_err(|_| StatusCode::BAD_REQUEST)?;
        let value =
            header::HeaderValue::from_str(value.trim()).map_err(|_| StatusCode::BAD_REQUEST)?;
        if headers.insert(name, value).is_some() {
            return Err(StatusCode::BAD_REQUEST);
        }
        Ok(())
    }

    fn check_no_transfer_encoding(headers: &HeaderMap) -> Result<(), StatusCode> {
        if headers.contains_key(header::TRANSFER_ENCODING) {
            return Err(StatusCode::BAD_REQUEST);
        }
        Ok(())
    }

    fn check_length(length: usize) -> Result<(), StatusCode> {
        if length > ResourceAccess::MAX_BYTES {
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }
        Ok(())
    }

    fn declared_content_length(headers: &HeaderMap) -> Option<usize> {
        let value = headers.get(header::CONTENT_LENGTH)?.to_str().ok()?;
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        value.parse::<usize>().ok()
    }
}
