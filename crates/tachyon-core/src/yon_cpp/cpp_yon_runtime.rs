use crate::tac_cpp::CppPrelude;

pub(crate) struct CppYonRuntime;

impl CppYonRuntime {
    const HOST: &'static str = include_str!("tachyon/yon_host.hpp");
    const PIECES: &'static [&'static str] = &[
        include_str!("tachyon/yon_frame.hpp"),
        include_str!("tachyon/yon_request.hpp"),
        include_str!("tachyon/yon_answer.hpp"),
        include_str!("tachyon/yon_events.hpp"),
        include_str!("tachyon/yon_needs.hpp"),
        include_str!("tachyon/yon_main.hpp"),
        include_str!("tachyon/yon_application.hpp"),
    ];

    pub(crate) fn source() -> String {
        format!(
            "{}{}{}{}{}",
            CppPrelude::yon(),
            Self::HOST,
            CppPrelude::context(),
            Self::PIECES.concat(),
            CppPrelude::services()
        )
    }
}
