#[derive(Clone, Copy, PartialEq)]
enum NativeApplicationPhase {
    Idle,
    Starting,
    Ready,
    Retired,
    Stopped,
}
