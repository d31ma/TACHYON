use crate::tac_cpp::CppPrelude;

pub(crate) struct CppYonRuntime;

impl CppYonRuntime {
    const HOST_STORAGE: &'static str = include_str!("tachyon/yon_host_storage.hpp");
    const HOST: &'static str = include_str!("tachyon/yon_host.hpp");
    const TEST_HOST: &'static str = include_str!("tachyon/yon_test_host.hpp");
    const FAKES: &'static str = include_str!("tachyon/yon_fakes.hpp");
    const PIECES: &'static [&'static str] = &[
        include_str!("tachyon/yon_frame.hpp"),
        include_str!("tachyon/yon_request.hpp"),
        include_str!("tachyon/yon_answer.hpp"),
        include_str!("tachyon/yon_events.hpp"),
        include_str!("tachyon/yon_delivery.hpp"),
        include_str!("tachyon/yon_needs.hpp"),
        include_str!("tachyon/yon_frames.hpp"),
        include_str!("tachyon/yon_stream.hpp"),
        include_str!("tachyon/yon_socket.hpp"),
        include_str!("tachyon/yon_gateway.hpp"),
        include_str!("tachyon/yon_main.hpp"),
        include_str!("tachyon/yon_application.hpp"),
        include_str!("tachyon/yon_storage.hpp"),
        include_str!("tachyon/yon_remote.hpp"),
    ];

    pub(crate) fn source() -> String {
        Self::composed(Self::HOST, "")
    }

    pub(crate) fn tested() -> String {
        let harness = format!("{}{}", CppPrelude::test_harness(), Self::FAKES);
        Self::composed(Self::TEST_HOST, &harness)
    }

    fn composed(host: &str, harness: &str) -> String {
        format!(
            "{}{}{}{host}{}{}{}{harness}",
            CppPrelude::yon(),
            CppPrelude::MEMORY_ENTITIES,
            Self::HOST_STORAGE,
            CppPrelude::context(),
            Self::PIECES.concat(),
            CppPrelude::services()
        )
    }
}
