use super::csharp_browser_bindings::CsharpBrowserBindings;

#[test]
fn csharp_http_bindings_capture_only_qualified_runtime_namespaces_and_typed_errors() {
    let authored = "namespace TachyonService0 { internal static class BehaviorBrowserHttp {} }\nnamespace TachyonService1 { class PlainService {} }\nnamespace TachyonService2 { internal static class BehaviorBrowserHttp {} }";
    let bindings = CsharpBrowserBindings::read(authored);
    assert!(
        bindings
            .contains("TachyonService0.BehaviorBrowserHttp.Capture ??= Tac.CaptureHttpTransport")
    );
    assert!(
        bindings
            .contains("TachyonService2.BehaviorBrowserHttp.Capture ??= Tac.CaptureHttpTransport")
    );
    assert!(!bindings.contains("TachyonService1.BehaviorBrowserHttp"));
    assert!(bindings.contains("TachyonService0.BehaviorError failure0 => failure0.Code"));
    assert!(!bindings.contains("GetProperty"));
    assert!(!bindings.contains("GetType"));
}
