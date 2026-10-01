struct NativeServiceDefinition<T> {
    name: &'static str,
    prototype: bool,
    create: fn(&NativeServiceScope) -> Result<T, String>,
    start: fn(&T) -> Result<(), String>,
    stop: fn(&T) -> Result<(), String>,
}
