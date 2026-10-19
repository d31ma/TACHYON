#[cfg(windows)]
pub struct TacWinHttpHandle {
    raw: *mut core::ffi::c_void,
}

#[cfg(windows)]
impl TacWinHttpHandle {
    fn opened(raw: *mut core::ffi::c_void) -> Result<Self, &'static str> {
        (!raw.is_null())
            .then(|| Self { raw })
            .ok_or_else(TacWinHttp::failure)
    }
}

#[cfg(windows)]
impl Drop for TacWinHttpHandle {
    fn drop(&mut self) {
        unsafe { WinHttpCloseHandle(self.raw) };
    }
}
