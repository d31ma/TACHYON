pub(crate) struct CppPrelude;

impl CppPrelude {
    const INCLUDES: &'static str = include_str!("tachyon/includes.hpp");
    const JSON: &'static str = include_str!("tachyon/json.hpp");
    const TASK: &'static str = include_str!("tachyon/task.hpp");
    const PAGE: &'static str = include_str!("tachyon/page.hpp");
    const TICKETS: &'static str = include_str!("tachyon/tickets.hpp");
    const PAGE_TASKS: &'static str = include_str!("tachyon/page_tasks.hpp");
    const TEST_HOST: &'static str = include_str!("tachyon/test_host.hpp");
    const TEST_EVENTS: &'static str = include_str!("tachyon/test_events.hpp");
    const HTTP: &'static str = include_str!("tachyon/http.hpp");
    const SERVICES: &'static str = include_str!("tachyon/services.hpp");
    const TEST: &'static str = include_str!("tachyon/test.hpp");
    pub(crate) const EXPORTS: &'static str = include_str!("tachyon/exports.hpp");

    pub(crate) fn json() -> String {
        [Self::INCLUDES, Self::JSON].concat()
    }

    pub(crate) fn web() -> String {
        [
            Self::INCLUDES,
            Self::JSON,
            Self::TASK,
            Self::PAGE,
            Self::TICKETS,
            Self::PAGE_TASKS,
            Self::HTTP,
            Self::SERVICES,
        ]
        .concat()
    }

    pub(crate) fn test() -> String {
        [
            Self::INCLUDES,
            Self::JSON,
            Self::TASK,
            Self::TEST_HOST,
            Self::TICKETS,
            Self::TEST_EVENTS,
            Self::HTTP,
            Self::SERVICES,
            Self::TEST,
        ]
        .concat()
    }
}
