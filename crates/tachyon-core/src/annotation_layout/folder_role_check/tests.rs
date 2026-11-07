use super::*;

#[test]
fn nested_services_require_native_markers_in_each_supported_language() {
    for (extension, marked, plain) in [
        (
            "js",
            "@Service\nexport class MoneyService {}",
            "export class MoneyService {}",
        ),
        (
            "ts",
            "@Service\nexport class MoneyService {}",
            "export class MoneyService {}",
        ),
        (
            "java",
            "@Service\nclass MoneyService {}",
            "class MoneyService {}",
        ),
        (
            "kt",
            "@Service\nclass MoneyService {}",
            "class MoneyService {}",
        ),
        (
            "cs",
            "[Service]\nclass MoneyService {}",
            "class MoneyService {}",
        ),
        (
            "rs",
            "#[yon::Service]\npub struct MoneyService;",
            "pub struct MoneyService;",
        ),
        (
            "dart",
            "@Service\nclass MoneyService {}",
            "class MoneyService {}",
        ),
        (
            "swift",
            "@Service\nclass MoneyService {}",
            "class MoneyService {}",
        ),
        (
            "py",
            "@service\nclass MoneyService:\n    pass",
            "class MoneyService:\n    pass",
        ),
        (
            "php",
            "<?php\n#[Service]\nclass MoneyService {}",
            "<?php\nclass MoneyService {}",
        ),
        (
            "groovy",
            "@Service\nclass MoneyService {}",
            "class MoneyService {}",
        ),
        (
            "go",
            "// @yon::Service\ntype MoneyService struct{}",
            "type MoneyService struct{}",
        ),
        (
            "rb",
            "#[Service]\nclass MoneyService\nend",
            "class MoneyService\nend",
        ),
        (
            "fs",
            "[<Service>]\ntype MoneyService() =\n    member this.Count : int = 1",
            "type MoneyService() =\n    member this.Count : int = 1",
        ),
        (
            "lua",
            "-- @Service\nMoneyService = {}\nfunction MoneyService:Count()\n  return 1\nend\n",
            "MoneyService = {}\nfunction MoneyService:Count()\n  return 1\nend\n",
        ),
    ] {
        let realm = if extension == "swift" {
            "client"
        } else {
            "server"
        };
        let name = format!("{realm}/services/finance/nested/MoneyService.{extension}");
        let path = Path::new(&name);
        FolderRoleCheck::check(path, marked)
            .unwrap_or_else(|failure| panic!("{extension}: {failure}"));
        let failure = FolderRoleCheck::check(path, plain).expect_err(extension);
        assert_eq!(
            failure.diagnostics()[0].code.number(),
            FolderRoleCheck::CODE,
            "{extension}"
        );
        assert!(failure.to_string().contains("MoneyService"), "{failure}");
        assert!(failure.to_string().contains("Service"), "{failure}");
    }
}

#[test]
fn java_and_fsharp_tac_services_need_only_their_marker() {
    for (name, marked, plain) in [
        (
            "client/services/MoneyService.java",
            "@Service\nclass MoneyService {\n    int count = 1;\n}\n",
            "class MoneyService {\n    int count = 1;\n}\n",
        ),
        (
            "client/services/MoneyService.fs",
            "[<Service>]\ntype MoneyService() =\n    member val Count = 1 with get, set\n",
            "type MoneyService() =\n    member val Count = 1 with get, set\n",
        ),
    ] {
        let path = Path::new(name);
        FolderRoleCheck::check(path, marked).unwrap_or_else(|failure| panic!("{name}: {failure}"));
        let failure = FolderRoleCheck::check(path, plain).expect_err(name);
        let message = &failure.diagnostics()[0].message;
        assert_eq!(
            failure.diagnostics()[0].code.number(),
            FolderRoleCheck::CODE,
            "{name}"
        );
        assert!(message.contains("required `"), "{message}");
        assert!(message.contains("Service"), "{message}");
        assert!(!message.contains("is not available"), "{message}");
    }
}

#[test]
fn a_lua_service_without_its_marker_is_told_the_lua_spelling() {
    let failure = FolderRoleCheck::check(
        Path::new("client/services/BasketService.lua"),
        "BasketService = {}\nfunction BasketService:Count()\n  return 1\nend\n",
    )
    .expect_err("an unmarked Lua service");
    let message = &failure.diagnostics()[0].message;
    assert!(message.contains("`-- @Service`"), "{message}");
}

#[test]
fn rust_and_go_services_without_their_marker_are_told_a_spelling_that_compiles() {
    for (path, source, spelling) in [
        (
            "client/services/BasketService.rs",
            "struct BasketService {}\n",
            "`#[Service]`",
        ),
        (
            "client/services/BasketService.go",
            "type BasketService struct{}\n",
            "`// @Service`",
        ),
    ] {
        let failure = FolderRoleCheck::check(Path::new(path), source).expect_err(path);
        let message = &failure.diagnostics()[0].message;
        assert!(message.contains(spelling), "{message}");
    }
}

#[test]
fn actual_role_folders_and_the_legacy_remote_alias_require_their_own_markers() {
    for realm in ["client", "server"] {
        for (folder, marker, suffix) in [
            ("services", "Service", "Service"),
            ("stores", "Store", "Store"),
            ("remotes", "HTTPClient", "Client"),
            ("clients", "HTTPClient", "Client"),
            ("setups", "Setup", "Setup"),
            ("entities", "Entity", "Entity"),
            ("delegates", "Delegate", "Delegate"),
            ("converters", "Converter", "Converter"),
        ] {
            let name = format!("{realm}/{folder}/one/two/Money{suffix}.ts");
            let path = Path::new(&name);
            let plain = format!("class Money{suffix} {{}}");
            assert!(FolderRoleCheck::check(path, &plain).is_err(), "{name}");
            FolderRoleCheck::check(path, &format!("@{marker}\n{plain}")).expect(&name);
        }
    }
    for marker in [
        "RequestRoute",
        "SocketRoute",
        "ErrorHandler",
        "IdentityProvider",
    ] {
        let suffix = if marker.ends_with("Route") {
            "Controller"
        } else {
            marker
        };
        let name = format!("server/controllers/one/Money{suffix}.ts");
        FolderRoleCheck::check(
            Path::new(&name),
            &format!("@{marker}\nclass Money{suffix} {{}}"),
        )
        .expect(marker);
    }
    assert!(
        FolderRoleCheck::check(
            Path::new("server/controllers/one/MoneyController.ts"),
            "class MoneyController {}"
        )
        .is_err()
    );
}

#[test]
fn contracts_assets_and_barrels_are_not_fake_providers() {
    for source in [
        "export interface Contract {}",
        "export type Contract = string",
        "export { Other } from './Other'",
        "abstract class Contract {}",
        "const note = 'class Service {}'",
        "// class Service {}",
    ] {
        FolderRoleCheck::check(Path::new("client/services/contracts/Contract.ts"), source)
            .expect(source);
    }
    for path in [
        "client/models/Money.rs",
        "server/shared/Money.rs",
        "client/shared/utils/MoneyUtils.rs",
        ".tachyon/services/MoneyService.js",
        "corpus/client/services/MoneyService.js",
    ] {
        FolderRoleCheck::check(Path::new(path), "pub struct Money;").expect(path);
    }
    assert!(
        FolderRoleCheck::check(
            Path::new("client/services/MoneyService.cs"),
            "namespace Money { public class MoneyService {} }"
        )
        .is_err()
    );
}

#[test]
fn nested_folders_cannot_hide_service_classes_and_formal_delegates_need_their_marker() {
    for realm in ["client", "server"] {
        let path = format!("{realm}/delegates/nested/MoneyService.rs");
        let error = FolderRoleCheck::check(Path::new(&path), "pub struct MoneyService;")
            .expect_err("misplaced service");
        assert!(error.to_string().contains("role class suffix"), "{error}");
        let path = format!("{realm}/delegates/nested/MoneyDelegate.rs");
        let error = FolderRoleCheck::check(Path::new(&path), "pub struct MoneyDelegate;")
            .expect_err("unmarked delegate");
        assert_eq!(error.diagnostics()[0].code.number(), FolderRoleCheck::CODE);
    }
}

#[test]
fn a_missing_marker_points_at_the_declaration_in_the_language_spelling() {
    let script = "import { html } from 'lit'\n\nexport class MoneyService {}\n";
    let failure = FolderRoleCheck::check(Path::new("client/services/MoneyService.js"), script)
        .expect_err("missing marker");
    let span = &failure.diagnostics()[0].spans[0];
    assert_eq!(span.start, 28);
    let lua = "MoneyService = {}\nMoneyService.__index = MoneyService\n\nfunction MoneyService:_total()\n  return 0\nend\n";
    let failure = FolderRoleCheck::check(Path::new("server/services/MoneyService.lua"), lua)
        .expect_err("missing Lua marker");
    let message = &failure.diagnostics()[0].message;
    assert!(
        message.contains("required `-- @Service` marker"),
        "{message}"
    );
}

#[test]
fn the_role_layout_help_names_the_authored_lua_and_fsharp_file() {
    for (path, source, wanted) in [
        (
            "server/services/Money.lua",
            "-- @Service\nMoney = {}\nMoney.__index = Money\n\nfunction Money:_total()\n  return 0\nend\n",
            "'services/MoneyService.lua'",
        ),
        (
            "server/services/Money.fs",
            "[<Service>]\ntype Money() =\n    member _.Total: int = 0\n",
            "'services/MoneyService.fs'",
        ),
    ] {
        let failure = super::super::validate(Path::new(path), source).expect_err("no suffix");
        let help = failure.diagnostics()[0].help.clone().unwrap_or_default();
        assert!(help.contains(wanted), "{path}: {help}");
    }
}
