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
fn actual_role_folders_and_the_legacy_remote_alias_require_their_own_markers() {
    for realm in ["client", "server"] {
        for (folder, marker, suffix) in [
            ("services", "Service", "Service"),
            ("stores", "Store", "Store"),
            ("remotes", "HTTPClient", "Client"),
            ("clients", "HTTPClient", "Client"),
            ("setups", "Setup", "Setup"),
            ("entities", "Entity", "Entity"),
            ("helpers", "Helper", "Helper"),
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
fn helpers_cannot_hide_service_classes_and_formal_helpers_need_their_marker() {
    for realm in ["client", "server"] {
        let path = format!("{realm}/helpers/nested/MoneyService.rs");
        let error = FolderRoleCheck::check(Path::new(&path), "pub struct MoneyService;")
            .expect_err("misplaced service");
        assert!(error.to_string().contains("role class suffix"), "{error}");
        let path = format!("{realm}/helpers/nested/MoneyHelper.rs");
        let error = FolderRoleCheck::check(Path::new(&path), "pub struct MoneyHelper;")
            .expect_err("unmarked helper");
        assert_eq!(error.diagnostics()[0].code.number(), FolderRoleCheck::CODE);
    }
}
