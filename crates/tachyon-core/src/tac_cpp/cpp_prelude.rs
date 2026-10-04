pub(crate) struct CppPrelude;

impl CppPrelude {
    pub(crate) const CORE: &'static str = concat!(
        include_str!("Includes.hpp"),
        include_str!("json/JsonValueKind.hpp"),
        include_str!("json/JsonException.hpp"),
        include_str!("json/JsonNode.hpp"),
        include_str!("json/JsonObject.hpp"),
        include_str!("json/JsonArray.hpp"),
        include_str!("json/JsonValue.hpp"),
        include_str!("json/JsonWriter.hpp"),
        include_str!("json/JsonReader.hpp"),
        include_str!("json/JsonCodec.hpp"),
        include_str!("json/JsonModelFields.hpp"),
        include_str!("json/JsonSerializer.hpp"),
    );
    pub(crate) const WEB_HOST: &'static str = include_str!("web/TacHost.hpp");
    pub(crate) const PAGE: &'static str = concat!(
        include_str!("page/TacServiceType.hpp"),
        include_str!("page/TacBind.hpp"),
        include_str!("page/TacOwner.hpp"),
        include_str!("page/TacPage.hpp"),
        include_str!("web/TacEventSink.hpp"),
    );
    pub(crate) const SERVICES: &'static str = concat!(
        include_str!("page/ApplicationEventPublisher.hpp"),
        include_str!("page/TacNeeds.hpp"),
        include_str!("Globals.hpp"),
    );
    pub(crate) const TEST: &'static str = concat!(
        include_str!("test/TacHost.hpp"),
        include_str!("test/TacEventSink.hpp"),
        include_str!("test/Assert.hpp"),
        include_str!("test/TacTestRunner.hpp"),
        include_str!("test/TestGlobals.hpp"),
    );
    pub(crate) const WEB_EXPORTS: &'static str = include_str!("web/TacExports.hpp");
}
