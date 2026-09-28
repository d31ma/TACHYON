pub struct Companion {
    pub runtime: String,
    pub target: String,
    pub cores: usize,
}

impl Companion {
    pub fn new() -> Self {
        Self {
            runtime: format!("Rust on {}", std::env::consts::OS),
            target: format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
            cores: std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
        }
    }

    pub fn home(&self) -> String {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_default()
    }
}
