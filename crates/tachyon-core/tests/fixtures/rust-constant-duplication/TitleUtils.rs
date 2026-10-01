pub struct TitleUtils;

impl TitleUtils {
    const CONNECTING: &'static str = "State connecting";
    const WAITING: &'static str = "State waiting";
    const ACTIVE: &'static str = "State active";
    const FINISHED: &'static str = "State finished";
    const FAILED: &'static str = "State failed";
    const STALE: &'static str = "State stale";
    const MISSING: &'static str = "State missing";
    const REPORTED: &'static str = "State reported";
    const RECOVERED: &'static str = "State recovered";
    const LINKED: &'static str = "State linked";
    const UNLINKED: &'static str = "State unlinked";

    pub fn text() -> String {
        Self::CONNECTING.to_owned()
    }
}
