use super::ordinary_browser_fixture::OrdinaryBrowserFixture;

struct DotnetHttpCacheBrowserFixture;

impl DotnetHttpCacheBrowserFixture {
    const ENVIRONMENT: [&'static str; 3] = [
        "TAC_DOTNET_HTTP_FIXTURE_OUT",
        "TAC_BIN",
        "TAC_DOTNET_HTTP_SOURCE_COMMIT",
    ];
    const FILES: [(&'static str, &'static str); 7] = [
        (
            "client/remotes/CachedProbeClient.cs",
            include_str!(
                "../../tests/fixtures/tac-dotnet-http-cache/client/remotes/CachedProbeClient.cs"
            ),
        ),
        (
            "client/services/CachedHttpService.cs",
            include_str!(
                "../../tests/fixtures/tac-dotnet-http-cache/client/services/CachedHttpService.cs"
            ),
        ),
        (
            "client/setups/CachedHttpSetup.cs",
            include_str!(
                "../../tests/fixtures/tac-dotnet-http-cache/client/setups/CachedHttpSetup.cs"
            ),
        ),
        (
            "client/pages/tac.html",
            include_str!("../../tests/fixtures/tac-dotnet-http-cache/client/pages/tac.html"),
        ),
        (
            "client/components/cached/http/tac.html",
            include_str!(
                "../../tests/fixtures/tac-dotnet-http-cache/client/components/cached/http/tac.html"
            ),
        ),
        (
            "client/manifest.json",
            include_str!("../../tests/fixtures/tac-dotnet-http-cache/client/manifest.json"),
        ),
        (
            "server/controllers/CachedProbeController.js",
            include_str!(
                "../../tests/fixtures/tac-dotnet-http-cache/server/controllers/CachedProbeController.js"
            ),
        ),
    ];

    async fn run() {
        let environment = OrdinaryBrowserFixture::read(Self::ENVIRONMENT);
        environment.validate();
        environment.stage(Self::FILES);
        environment.build().await;
        environment
            .browser(
                Self::ENVIRONMENT,
                "scripts/dotnet-http-cache-browser-gate.mjs",
            )
            .await;
    }
}

#[tokio::test]
#[ignore = "requires explicit matching compiler, isolated fixture/cache, .NET wasm-tools and installed browser"]
async fn dotnet_cached_http_client_uses_public_yon_and_revalidates_after_clear() {
    DotnetHttpCacheBrowserFixture::run().await;
}

#[test]
fn csharp_cached_http_client_keeps_canonical_request_and_cache_lowering() {
    let (name, source) = DotnetHttpCacheBrowserFixture::FILES[0];
    let path = std::path::Path::new(name);
    let concrete = crate::declared_contract::DeclaredContract::concrete(path, source, "tac")
        .unwrap_or_else(|| panic!("canonical C# HTTPClient requires concrete generation"));
    assert!(concrete.contains("typeof(CachedHttpSetup)"));
    let lowered = crate::behavior::lower(path, &concrete, "tac")
        .unwrap_or_else(|error| panic!("canonical HTTPClient/cache lowering: {error}"))
        .unwrap_or_else(|| panic!("canonical HTTPClient requires behavior lowering"));
    assert!(lowered.contains("typeof(CachedHttpSetup)"));
    assert!(lowered.contains("\"cacheable\""));
    assert!(lowered.contains("\"cacheEvict\""));
    assert!(lowered.contains("\"httpExchange\""));
    assert!(lowered.contains("System.Threading.Tasks.Task<BehaviorJSON>"));
}
