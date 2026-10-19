#![allow(clippy::expect_used)]

use super::helper_tests::HelperPage;

struct NamedPage;

impl NamedPage {
    const HELPERS: [(&'static str, &'static str); 2] = [
        (
            "client/helpers/GreeterHelper.lua",
            "-- @Helper\n---@class GreeterHelper\nGreeterHelper = {}\n\n---@return string\nfunction GreeterHelper:greet()\n  return \"hi\"\nend\n",
        ),
        (
            "client/helpers/LoudGreeterHelper.lua",
            "-- @Helper\n---@class LoudGreeterHelper\nLoudGreeterHelper = {}\n\n---@return string\nfunction LoudGreeterHelper:greet()\n  return \"HI\"\nend\n",
        ),
    ];

    const SERVICE: &'static str = "-- @Service\n---@class WelcomeService\nWelcomeService = {\n  -- @Needs\n  -- @Named(\"LoudGreeterHelper\")\n  ---@type GreeterHelper\n  greeter = nil,\n  -- @Needs\n  ---@type GreeterHelper\n  plain = nil,\n}\n\n\
         ---@return string\nfunction WelcomeService:welcome()\n  return self.greeter:greet()\nend\n\n\
         ---@return string\nfunction WelcomeService:hello()\n  return self.plain:greet()\nend\n";

    const DRIVER: &'static str = "const { default: WelcomeService } = await import('./WelcomeService.js')\n\
         const welcome = new WelcomeService({})\n\
         console.log(JSON.stringify({ greeted: [welcome.welcome(), welcome.hello()] }))\n\
         process.exit(0)\n";
}

#[tokio::test]
async fn a_lua_needs_field_gets_the_helper_that_named_names() {
    if let Some(reason) = HelperPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in NamedPage::HELPERS {
        HelperPage::write(root.path(), relative, text);
    }
    HelperPage::publish(root.path(), "welcome", "WelcomeService", NamedPage::SERVICE).await;
    let report = HelperPage::run(root.path(), NamedPage::DRIVER);
    assert_eq!(
        report["greeted"],
        serde_json::json!(["HI", "hi"]),
        "{report}"
    );
}
