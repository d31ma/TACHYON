#[cfg(target_os = "android")]
type TacJniTable = *mut *const *const core::ffi::c_void;

#[cfg(target_os = "android")]
type TacJniObject = *mut core::ffi::c_void;

#[cfg(target_os = "android")]
pub struct TacAndroidBridge {
    machine: usize,
    class: usize,
    publish: usize,
    dispatch: usize,
    capability: usize,
    http: usize,
}

#[cfg(target_os = "android")]
impl TacAndroidBridge {
    const JNI_VERSION: i32 = 0x0001_0006;
    const DETACHED: i32 = -2;
    const GET_ENV: usize = 6;
    const ATTACH_AS_DAEMON: usize = 7;
    const EXCEPTION_CLEAR: usize = 17;
    const NEW_GLOBAL_REF: usize = 21;
    const DELETE_LOCAL_REF: usize = 23;
    const GET_STATIC_METHOD_ID: usize = 113;
    const CALL_STATIC_OBJECT_METHOD_A: usize = 116;
    const CALL_STATIC_VOID_METHOD_A: usize = 143;
    const GET_ARRAY_LENGTH: usize = 171;
    const NEW_BYTE_ARRAY: usize = 176;
    const GET_BYTE_ARRAY_REGION: usize = 200;
    const SET_BYTE_ARRAY_REGION: usize = 208;
    const GET_JAVA_VM: usize = 219;
    const EXCEPTION_CHECK: usize = 228;
    const BYTE_LIMIT: usize = 1_048_576;
    const UNSUPPORTED: &'static str = "{\"status\":\"unsupported\"}";
    const HTTP_MISSING: &'static str = "HTTP_MISSING";
    const HTTP_NETWORK: &'static str = "HTTP_NETWORK";
    const HTTP_CODES: [&'static str; 4] = ["DEADLINE", "HTTP_LIMIT", "HTTP_REQUEST", "HTTP_NETWORK"];
    const STATUS_BYTES: usize = 4;

    fn installed() -> &'static std::sync::OnceLock<Self> {
        static BRIDGE: std::sync::OnceLock<TacAndroidBridge> = std::sync::OnceLock::new();
        &BRIDGE
    }

    fn store_folder() -> &'static std::sync::OnceLock<std::path::PathBuf> {
        static FOLDER: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
        &FOLDER
    }

    fn store_base() -> Option<std::path::PathBuf> {
        Self::store_folder().get().cloned()
    }

    #[allow(non_snake_case)]
    #[unsafe(no_mangle)]
    pub unsafe extern "system" fn Java_dev_tachyon_host_TachyonRust_install(
        environment: TacJniTable,
        class: TacJniObject,
        store: TacJniObject,
    ) {
        let folder = String::from_utf8_lossy(&Self::bytes(environment, store)).into_owned();
        let _ = Self::store_folder().set(std::path::PathBuf::from(folder));
        let _ = Self::bind(environment, class).map(Self::install_bridge);
    }

    fn install_bridge(bridge: Self) {
        let _ = Self::installed().set(bridge).map(|()| Self::register());
    }

    fn register() {
        unsafe { tac_native_set_emit(Some(Self::emit)) };
        unsafe { TacTaskDispatch::tac_native_set_dispatch(Some(Self::dispatch)) };
        NativeCapabilities::register(Self::capability);
    }

    #[allow(non_snake_case)]
    #[unsafe(no_mangle)]
    pub unsafe extern "system" fn Java_dev_tachyon_host_TachyonRust_call(
        environment: TacJniTable,
        _class: TacJniObject,
        request: TacJniObject,
    ) -> TacJniObject {
        let request = String::from_utf8_lossy(&Self::bytes(environment, request)).into_owned();
        let answer = tac_invoke_owned(&request);
        Self::array(environment, answer.as_bytes())
    }

    #[allow(non_snake_case)]
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_tachyon_host_TachyonRust_start(
        _environment: TacJniTable,
        _class: TacJniObject,
    ) -> u8 {
        u8::from(tac_native_application_start())
    }

    #[allow(non_snake_case)]
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_tachyon_host_TachyonRust_stop(
        _environment: TacJniTable,
        _class: TacJniObject,
    ) {
        tac_native_application_shutdown();
    }

    pub fn text(environment: TacJniTable, array: TacJniObject) -> std::ffi::CString {
        std::ffi::CString::new(Self::bytes(environment, array)).unwrap_or_default()
    }

    fn bind(environment: TacJniTable, class: TacJniObject) -> Option<Self> {
        let new_global: extern "system" fn(TacJniTable, TacJniObject) -> TacJniObject =
            unsafe { core::mem::transmute(Self::function(environment, Self::NEW_GLOBAL_REF)) };
        let global = new_global(environment, class);
        let machine = (!global.is_null())
            .then(|| Self::java_machine(environment))
            .flatten()?;
        Some(Self {
            machine: machine as usize,
            class: global as usize,
            publish: Self::method(environment, global, c"publish", c"([B)V")?,
            dispatch: Self::method(environment, global, c"dispatch", c"()V")?,
            capability: Self::method(environment, global, c"capability", c"([B[B)[B")?,
            http: Self::method(environment, global, c"http", c"([B[B[B[BJJ)[B")?,
        })
    }

    fn java_machine(environment: TacJniTable) -> Option<TacJniTable> {
        let java_machine: extern "system" fn(TacJniTable, *mut TacJniTable) -> i32 =
            unsafe { core::mem::transmute(Self::function(environment, Self::GET_JAVA_VM)) };
        let mut machine: TacJniTable = core::ptr::null_mut();
        let found = java_machine(environment, &mut machine) == 0;
        found.then_some(machine)
    }

    fn method(
        environment: TacJniTable,
        class: TacJniObject,
        name: &core::ffi::CStr,
        signature: &core::ffi::CStr,
    ) -> Option<usize> {
        let find: extern "system" fn(
            TacJniTable,
            TacJniObject,
            *const core::ffi::c_char,
            *const core::ffi::c_char,
        ) -> TacJniObject =
            unsafe { core::mem::transmute(Self::function(environment, Self::GET_STATIC_METHOD_ID)) };
        let method = find(environment, class, name.as_ptr(), signature.as_ptr());
        Self::clear(environment);
        (!method.is_null()).then_some(method as usize)
    }

    extern "C" fn emit(payload: *const core::ffi::c_char) {
        let payload = (!payload.is_null())
            .then(|| unsafe { core::ffi::CStr::from_ptr(payload) }.to_bytes());
        let _ = payload
            .zip(Self::installed().get())
            .map(|(payload, bridge)| bridge.send(bridge.publish, &[payload]));
    }

    extern "C" fn dispatch() {
        let _ = Self::installed()
            .get()
            .map(|bridge| bridge.send(bridge.dispatch, &[]));
    }

    fn capability(capability: &str, payload: &str) -> String {
        Self::installed()
            .get()
            .and_then(|bridge| bridge.ask(capability, payload))
            .unwrap_or_else(|| String::from(Self::UNSUPPORTED))
    }

    pub fn exchange(request: &TacNativeHttpRequest) -> Result<TacNativeHttpResponse, &'static str> {
        let bridge = Self::installed().get().ok_or(Self::HTTP_MISSING)?;
        let answer = bridge.http_answer(request).ok_or(Self::HTTP_NETWORK)?;
        Self::http_response(&answer)
    }

    fn http_answer(&self, request: &TacNativeHttpRequest) -> Option<Vec<u8>> {
        let environment = self.environment()?;
        let call: extern "system" fn(TacJniTable, TacJniObject, TacJniObject, *const u64) -> TacJniObject =
            unsafe { core::mem::transmute(Self::function(environment, Self::CALL_STATIC_OBJECT_METHOD_A)) };
        let url = format!("{}{}", request.origin, request.target);
        let headers = Self::header_lines(&request.headers);
        let mut arrays = Self::arrays(environment, &[request.method.as_bytes(), url.as_bytes(), headers.as_bytes()]);
        let body = request.body.as_deref().map(|body| Self::array(environment, body.as_bytes()));
        arrays.push(body.unwrap_or(core::ptr::null_mut()));
        let ready = Self::present(&arrays[..3]) & body.is_none_or(|body| !body.is_null());
        let mut arguments: Vec<u64> = arrays.iter().map(|array| *array as u64).collect();
        arguments.extend([request.remaining_ms(), request.limit as u64]);
        let answer = ready
            .then(|| call(environment, self.class as TacJniObject, self.http as TacJniObject, arguments.as_ptr()))
            .unwrap_or(core::ptr::null_mut());
        Self::clear(environment);
        let bytes = (!answer.is_null()).then(|| Self::bytes(environment, answer));
        arrays.into_iter().chain([answer]).for_each(|object| Self::release(environment, object));
        bytes
    }

    fn http_response(answer: &[u8]) -> Result<TacNativeHttpResponse, &'static str> {
        let (status, body) = answer
            .split_first_chunk::<{ Self::STATUS_BYTES }>()
            .ok_or(Self::HTTP_NETWORK)?;
        let status = u32::from_be_bytes(*status);
        if status == 0 {
            return Err(Self::http_code(body));
        }
        u16::try_from(status)
            .map(|status| TacNativeHttpResponse { status, body: body.to_vec() })
            .map_err(|_| Self::HTTP_NETWORK)
    }

    fn http_code(named: &[u8]) -> &'static str {
        Self::HTTP_CODES
            .into_iter()
            .find(|code| code.as_bytes() == named)
            .unwrap_or(Self::HTTP_NETWORK)
    }

    fn header_lines(headers: &[(String, String)]) -> String {
        headers
            .iter()
            .map(|(name, value)| format!("{name}\n{value}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn send(&self, method: usize, values: &[&[u8]]) {
        let _ = self
            .environment()
            .map(|environment| self.send_in(environment, method, values));
    }

    fn send_in(&self, environment: TacJniTable, method: usize, values: &[&[u8]]) {
        let call: extern "system" fn(TacJniTable, TacJniObject, TacJniObject, *const TacJniObject) =
            unsafe { core::mem::transmute(Self::function(environment, Self::CALL_STATIC_VOID_METHOD_A)) };
        let arguments = Self::arrays(environment, values);
        let _ = Self::present(&arguments).then(|| {
            call(environment, self.class as TacJniObject, method as TacJniObject, arguments.as_ptr())
        });
        Self::clear(environment);
        arguments.into_iter().for_each(|argument| Self::release(environment, argument));
    }

    fn ask(&self, capability: &str, payload: &str) -> Option<String> {
        let environment = self.environment()?;
        let call: extern "system" fn(
            TacJniTable,
            TacJniObject,
            TacJniObject,
            *const TacJniObject,
        ) -> TacJniObject =
            unsafe { core::mem::transmute(Self::function(environment, Self::CALL_STATIC_OBJECT_METHOD_A)) };
        let arguments = Self::arrays(environment, &[capability.as_bytes(), payload.as_bytes()]);
        let answer = Self::present(&arguments)
            .then(|| {
                call(environment, self.class as TacJniObject, self.capability as TacJniObject, arguments.as_ptr())
            })
            .unwrap_or(core::ptr::null_mut());
        Self::clear(environment);
        let text = (!answer.is_null())
            .then(|| String::from_utf8_lossy(&Self::bytes(environment, answer)).into_owned());
        arguments.into_iter().chain([answer]).for_each(|object| Self::release(environment, object));
        text
    }

    fn environment(&self) -> Option<TacJniTable> {
        let machine = self.machine as TacJniTable;
        let current: extern "system" fn(TacJniTable, *mut TacJniTable, i32) -> i32 =
            unsafe { core::mem::transmute(Self::function(machine, Self::GET_ENV)) };
        let attach: extern "system" fn(TacJniTable, *mut TacJniTable, TacJniObject) -> i32 =
            unsafe { core::mem::transmute(Self::function(machine, Self::ATTACH_AS_DAEMON)) };
        let mut environment: TacJniTable = core::ptr::null_mut();
        let state = current(machine, &mut environment, Self::JNI_VERSION);
        let attached = (state == Self::DETACHED)
            .then(|| attach(machine, &mut environment, core::ptr::null_mut()) == 0);
        attached
            .unwrap_or(state == 0)
            .then_some(environment)
            .filter(|environment| !environment.is_null())
    }

    fn present(objects: &[TacJniObject]) -> bool {
        objects.iter().all(|object| !object.is_null())
    }

    fn arrays(environment: TacJniTable, values: &[&[u8]]) -> Vec<TacJniObject> {
        values.iter().map(|value| Self::array(environment, value)).collect()
    }

    fn array(environment: TacJniTable, bytes: &[u8]) -> TacJniObject {
        i32::try_from(bytes.len()).map_or(core::ptr::null_mut(), |length| {
            Self::filled(environment, bytes, length)
        })
    }

    fn filled(environment: TacJniTable, bytes: &[u8], length: i32) -> TacJniObject {
        let create: extern "system" fn(TacJniTable, i32) -> TacJniObject =
            unsafe { core::mem::transmute(Self::function(environment, Self::NEW_BYTE_ARRAY)) };
        let fill: extern "system" fn(TacJniTable, TacJniObject, i32, i32, *const i8) =
            unsafe { core::mem::transmute(Self::function(environment, Self::SET_BYTE_ARRAY_REGION)) };
        let array = create(environment, length);
        let _ = (!array.is_null()).then(|| fill(environment, array, 0, length, bytes.as_ptr().cast()));
        Self::clear(environment);
        array
    }

    fn bytes(environment: TacJniTable, array: TacJniObject) -> Vec<u8> {
        (!array.is_null())
            .then(|| Self::read(environment, array))
            .unwrap_or_default()
    }

    fn read(environment: TacJniTable, array: TacJniObject) -> Vec<u8> {
        let length: extern "system" fn(TacJniTable, TacJniObject) -> i32 =
            unsafe { core::mem::transmute(Self::function(environment, Self::GET_ARRAY_LENGTH)) };
        let read: extern "system" fn(TacJniTable, TacJniObject, i32, i32, *mut i8) =
            unsafe { core::mem::transmute(Self::function(environment, Self::GET_BYTE_ARRAY_REGION)) };
        let size = usize::try_from(length(environment, array))
            .ok()
            .filter(|size| *size <= Self::BYTE_LIMIT)
            .unwrap_or(0);
        let mut buffer = vec![0_u8; size];
        read(environment, array, 0, size as i32, buffer.as_mut_ptr().cast());
        buffer
    }

    fn clear(environment: TacJniTable) {
        let pending: extern "system" fn(TacJniTable) -> u8 =
            unsafe { core::mem::transmute(Self::function(environment, Self::EXCEPTION_CHECK)) };
        let clear: extern "system" fn(TacJniTable) =
            unsafe { core::mem::transmute(Self::function(environment, Self::EXCEPTION_CLEAR)) };
        if pending(environment) != 0 {
            clear(environment);
        }
    }

    fn release(environment: TacJniTable, object: TacJniObject) {
        let delete: extern "system" fn(TacJniTable, TacJniObject) =
            unsafe { core::mem::transmute(Self::function(environment, Self::DELETE_LOCAL_REF)) };
        if !object.is_null() {
            delete(environment, object);
        }
    }

    fn function(table: TacJniTable, index: usize) -> *const core::ffi::c_void {
        unsafe { *(*table).add(index) }
    }
}
