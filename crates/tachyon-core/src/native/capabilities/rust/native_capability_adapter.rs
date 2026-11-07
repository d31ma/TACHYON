type NativeCapabilityCall =
    extern "C" fn(*const core::ffi::c_char, *const core::ffi::c_char, *mut usize) -> *mut u8;
type NativeCapabilityFree = extern "C" fn(*mut u8, usize);

#[derive(Clone, Copy, Debug)]
enum NativeCapabilityAdapter {
    Unsupported,
    Local(fn(&str, &str) -> String),
    Host(NativeCapabilityCall, NativeCapabilityFree),
}

impl NativeCapabilityAdapter {
    const REQUEST_LIMIT: usize = 65_536;
    const REPLY_LIMIT: usize = 1_048_576;

    fn invoke(self, capability: &str, payload: &str) -> String {
        match self {
            Self::Unsupported => String::from("{\"status\":\"unsupported\"}"),
            Self::Local(call) => call(capability, payload),
            Self::Host(call, free) => Self::host(call, free, capability, payload),
        }
    }

    fn registered(call: Option<NativeCapabilityCall>, free: Option<NativeCapabilityFree>) -> Self {
        match (call, free) {
            (Some(call), Some(free)) => Self::Host(call, free),
            _ => Self::Unsupported,
        }
    }

    fn host(
        call: NativeCapabilityCall,
        free: NativeCapabilityFree,
        capability: &str,
        payload: &str,
    ) -> String {
        Self::request(capability, payload)
            .map_or_else(Self::unavailable, |(capability, payload)| {
                Self::reply(call, free, &capability, &payload)
            })
    }

    fn request(capability: &str, payload: &str) -> Option<(std::ffi::CString, std::ffi::CString)> {
        if capability.len() > 64 || payload.len() > Self::REQUEST_LIMIT {
            return None;
        }
        Some((
            std::ffi::CString::new(capability).ok()?,
            std::ffi::CString::new(payload).ok()?,
        ))
    }

    fn reply(
        call: NativeCapabilityCall,
        free: NativeCapabilityFree,
        capability: &std::ffi::CStr,
        payload: &std::ffi::CStr,
    ) -> String {
        let mut length = 0;
        let answer = call(capability.as_ptr(), payload.as_ptr(), &mut length);
        core::ptr::NonNull::new(answer).map_or_else(Self::unavailable, |answer| {
            Self::copy_reply(answer.as_ptr(), length, free)
        })
    }

    fn copy_reply(answer: *mut u8, length: usize, free: NativeCapabilityFree) -> String {
        if length == 0 || length > Self::REPLY_LIMIT {
            free(answer, length);
            return Self::unavailable();
        }
        let bytes = unsafe { std::slice::from_raw_parts(answer, length) };
        let text = String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| Self::unavailable());
        free(answer, length);
        text
    }

    fn unavailable() -> String {
        String::from("{\"status\":\"unavailable\"}")
    }
}
