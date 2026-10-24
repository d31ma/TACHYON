@MainActor
final class IosMenus {
    static let routes: [IosMenuRoute] = __NATIVE_MENU_ROUTES__
    static let groups: [(String, UIMenu.Identifier)] = [("file", .file), ("view", .view), ("help", .help)]

    static func build(_ builder: UIMenuBuilder) {
        guard builder.system == .main else { return }
        for (group, parent) in groups {
            let commands = routes.filter { (route: IosMenuRoute) -> Bool in route.group == group }.map(command)
            guard !commands.isEmpty else { continue }
            let identifier = UIMenu.Identifier("dev.tachyon.menu." + group)
            builder.insertChild(UIMenu(title: "", identifier: identifier, options: .displayInline, children: commands), atStartOfMenu: parent)
        }
    }

    static func shortcuts() -> [UIKeyCommand] {
        guard UIDevice.current.userInterfaceIdiom == .phone else { return [] }
        return routes.filter { (route: IosMenuRoute) -> Bool in !route.key.isEmpty }.map(keyCommand)
    }

    static func destination(_ command: UICommand) -> URL? {
        guard let id = command.propertyList as? String,
              let entry = routes.first(where: { (route: IosMenuRoute) -> Bool in route.id == id && !route.action }),
              tachyonRouteDocuments[entry.route] != nil else { return nil }
        return URL(string: "tachyon-app://bundle" + entry.route)
    }

    static func action(_ command: UICommand, _ view: WKWebView?) -> IosMenuRoute? {
        guard routes.count <= 64, let id = command.propertyList as? String,
              let entry = routes.first(where: { (route: IosMenuRoute) -> Bool in route.id == id && route.action }) else { return nil }
        return entry.event.isEmpty || (view != nil && tachyonMenuReady(entry.event)) ? entry : nil
    }

    private static func selector(_ route: IosMenuRoute) -> Selector {
        route.action ? #selector(RootViewController.runMenuAction(_:)) : #selector(RootViewController.openMenuRoute(_:))
    }

    private static func command(_ route: IosMenuRoute) -> UIMenuElement {
        if !route.key.isEmpty { return keyCommand(route) }
        return UICommand(title: route.label, action: selector(route), propertyList: route.id)
    }

    private static func keyCommand(_ route: IosMenuRoute) -> UIKeyCommand {
        let command = UIKeyCommand(title: route.label, action: selector(route),
                                   input: route.key, modifierFlags: route.modifiers, propertyList: route.id)
        command.discoverabilityTitle = route.label
        command.wantsPriorityOverSystemBehavior = true
        return command
    }
}
