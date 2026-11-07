#[cfg(windows)]
#[link(name = "winhttp")]
unsafe extern "system" {
    fn WinHttpOpen(
        agent: *const u16,
        access: u32,
        proxy: *const u16,
        bypass: *const u16,
        flags: u32,
    ) -> *mut core::ffi::c_void;
    fn WinHttpConnect(
        session: *mut core::ffi::c_void,
        server: *const u16,
        port: u16,
        reserved: u32,
    ) -> *mut core::ffi::c_void;
    fn WinHttpOpenRequest(
        connection: *mut core::ffi::c_void,
        verb: *const u16,
        object: *const u16,
        version: *const u16,
        referrer: *const u16,
        accept: *const *const u16,
        flags: u32,
    ) -> *mut core::ffi::c_void;
    fn WinHttpSetTimeouts(
        handle: *mut core::ffi::c_void,
        resolve: i32,
        connect: i32,
        send: i32,
        receive: i32,
    ) -> i32;
    fn WinHttpSetOption(
        handle: *mut core::ffi::c_void,
        option: u32,
        buffer: *const core::ffi::c_void,
        length: u32,
    ) -> i32;
    fn WinHttpSendRequest(
        request: *mut core::ffi::c_void,
        headers: *const u16,
        headers_length: u32,
        optional: *const core::ffi::c_void,
        optional_length: u32,
        total_length: u32,
        context: usize,
    ) -> i32;
    fn WinHttpReceiveResponse(request: *mut core::ffi::c_void, reserved: *mut core::ffi::c_void) -> i32;
    fn WinHttpQueryHeaders(
        request: *mut core::ffi::c_void,
        level: u32,
        name: *const u16,
        buffer: *mut core::ffi::c_void,
        length: *mut u32,
        index: *mut u32,
    ) -> i32;
    fn WinHttpReadData(
        request: *mut core::ffi::c_void,
        buffer: *mut core::ffi::c_void,
        wanted: u32,
        read: *mut u32,
    ) -> i32;
    fn WinHttpCloseHandle(handle: *mut core::ffi::c_void) -> i32;
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetLastError() -> u32;
}

#[cfg(windows)]
pub struct TacWinHttp;

#[cfg(windows)]
impl TacWinHttp {
    const AGENT: &'static str = "Tachyon";
    const AUTOMATIC_PROXY: u32 = 4;
    const SECURE_FLAG: u32 = 0x0080_0000;
    const DISABLE_FEATURE: u32 = 63;
    const DISABLED_FEATURES: u32 = 0x1 | 0x2 | 0x4;
    const STATUS_CODE: u32 = 19;
    const CONTENT_LENGTH: u32 = 5;
    const NUMBER_FLAG: u32 = 0x2000_0000;
    const TIMEOUT_ERROR: u32 = 12002;
    const CHUNK: usize = 16_384;
    const NETWORK: &'static str = "HTTP_NETWORK";
    const DEADLINE: &'static str = "DEADLINE";
    const LIMIT: &'static str = "HTTP_LIMIT";

    pub fn exchange(request: &TacNativeHttpRequest) -> Result<TacNativeHttpResponse, &'static str> {
        let agent = Self::wide(Self::AGENT);
        let session = TacWinHttpHandle::opened(unsafe {
            WinHttpOpen(agent.as_ptr(), Self::AUTOMATIC_PROXY, core::ptr::null(), core::ptr::null(), 0)
        })?;
        let host = Self::wide(request.host());
        let connection =
            TacWinHttpHandle::opened(unsafe { WinHttpConnect(session.raw, host.as_ptr(), request.port(), 0) })?;
        let opened = Self::open(&connection, request)?;
        Self::send(&opened, request)?;
        let status = Self::number(&opened, Self::STATUS_CODE).ok_or(Self::NETWORK)?;
        Self::require_length(&opened, request.limit)?;
        let body = Self::read(&opened, request)?;
        Ok(TacNativeHttpResponse {
            status: u16::try_from(status).map_err(|_| Self::NETWORK)?,
            body,
        })
    }

    fn open(
        connection: &TacWinHttpHandle,
        request: &TacNativeHttpRequest,
    ) -> Result<TacWinHttpHandle, &'static str> {
        let verb = Self::wide(request.method);
        let target = Self::wide(&request.target);
        let flags = [0, Self::SECURE_FLAG][usize::from(request.secure())];
        let opened = TacWinHttpHandle::opened(unsafe {
            WinHttpOpenRequest(
                connection.raw,
                verb.as_ptr(),
                target.as_ptr(),
                core::ptr::null(),
                core::ptr::null(),
                core::ptr::null(),
                flags,
            )
        })?;
        let features = Self::DISABLED_FEATURES;
        let pointer = (&raw const features).cast::<core::ffi::c_void>();
        let size = u32::try_from(core::mem::size_of::<u32>()).unwrap_or(4);
        Self::checked(unsafe { WinHttpSetOption(opened.raw, Self::DISABLE_FEATURE, pointer, size) })?;
        Self::limit_time(&opened, request)?;
        Ok(opened)
    }

    fn limit_time(opened: &TacWinHttpHandle, request: &TacNativeHttpRequest) -> Result<(), &'static str> {
        let remaining = i32::try_from(request.remaining_ms()).unwrap_or(i32::MAX).max(1);
        Self::checked(unsafe { WinHttpSetTimeouts(opened.raw, remaining, remaining, remaining, remaining) })
    }

    fn send(opened: &TacWinHttpHandle, request: &TacNativeHttpRequest) -> Result<(), &'static str> {
        let headers = Self::wide(&Self::header_lines(&request.headers));
        let header_length = u32::try_from(headers.len().saturating_sub(1)).map_err(|_| Self::NETWORK)?;
        let body = request.body.as_deref().unwrap_or_default().as_bytes();
        let body_length = u32::try_from(body.len()).map_err(|_| Self::NETWORK)?;
        let body_pointer = body.as_ptr().cast::<core::ffi::c_void>();
        Self::checked(unsafe {
            WinHttpSendRequest(
                opened.raw,
                headers.as_ptr(),
                header_length,
                body_pointer,
                body_length,
                body_length,
                0,
            )
        })?;
        Self::checked(unsafe { WinHttpReceiveResponse(opened.raw, core::ptr::null_mut()) })
    }

    fn header_lines(headers: &[(String, String)]) -> String {
        headers
            .iter()
            .map(|(name, value)| format!("{name}: {value}\r\n"))
            .collect()
    }

    fn number(opened: &TacWinHttpHandle, level: u32) -> Option<u32> {
        let mut value = 0u32;
        let mut length = u32::try_from(core::mem::size_of::<u32>()).unwrap_or(4);
        let found = unsafe {
            WinHttpQueryHeaders(
                opened.raw,
                level | Self::NUMBER_FLAG,
                core::ptr::null(),
                (&raw mut value).cast::<core::ffi::c_void>(),
                &raw mut length,
                core::ptr::null_mut(),
            )
        };
        (found != 0).then_some(value)
    }

    fn require_length(opened: &TacWinHttpHandle, limit: usize) -> Result<(), &'static str> {
        let declared = Self::number(opened, Self::CONTENT_LENGTH).map_or(0, |length| length as usize);
        (declared <= limit).then_some(()).ok_or(Self::LIMIT)
    }

    fn read(opened: &TacWinHttpHandle, request: &TacNativeHttpRequest) -> Result<Vec<u8>, &'static str> {
        let mut body = Vec::new();
        while Self::read_chunk(opened, &mut body)? {
            Self::within(&body, request)?;
        }
        Ok(body)
    }

    fn read_chunk(opened: &TacWinHttpHandle, body: &mut Vec<u8>) -> Result<bool, &'static str> {
        let mut chunk = vec![0u8; Self::CHUNK];
        let mut read = 0u32;
        let wanted = u32::try_from(Self::CHUNK).unwrap_or(u32::MAX);
        Self::checked(unsafe {
            WinHttpReadData(opened.raw, chunk.as_mut_ptr().cast::<core::ffi::c_void>(), wanted, &raw mut read)
        })?;
        chunk.truncate(read as usize);
        body.extend_from_slice(&chunk);
        Ok(read > 0)
    }

    fn within(body: &[u8], request: &TacNativeHttpRequest) -> Result<(), &'static str> {
        let failed = [
            (body.len() > request.limit).then_some(Self::LIMIT),
            (request.remaining_ms() == 0).then_some(Self::DEADLINE),
        ];
        failed.into_iter().flatten().next().map_or(Ok(()), Err)
    }

    fn checked(succeeded: i32) -> Result<(), &'static str> {
        (succeeded != 0).then_some(()).ok_or_else(Self::failure)
    }

    fn failure() -> &'static str {
        let error = unsafe { GetLastError() };
        [Self::NETWORK, Self::DEADLINE][usize::from(error == Self::TIMEOUT_ERROR)]
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(core::iter::once(0)).collect()
    }
}
