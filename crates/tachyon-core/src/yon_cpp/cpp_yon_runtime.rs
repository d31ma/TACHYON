use crate::tac_cpp::CppPrelude;

pub(crate) struct CppYonRuntime;

impl CppYonRuntime {
    const PIECES: &'static [&'static str] = &[
        include_str!("tachyon/yon_frame.hpp"),
        include_str!("tachyon/yon_answer.hpp"),
        include_str!("tachyon/yon_events.hpp"),
        include_str!("tachyon/yon_needs.hpp"),
        include_str!("tachyon/yon_main.hpp"),
    ];

    pub(crate) fn source() -> String {
        format!(
            "{}{}{}",
            CppPrelude::yon(),
            Self::PIECES.concat(),
            CppPrelude::services()
        )
    }
}
