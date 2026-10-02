@MainActor
final class MacOsMenus {
    private var navigation: MacOsMenuNavigation?

    func install(_ application: NSApplication, _ window: NSWindow, _ name: String, _ navigation: MacOsMenuNavigation? = nil) {
        let root = NSMenu()
        let view = viewMenu(window)
        let help = helpMenu(application)
        root.addItem(applicationMenu(application, name))
        root.addItem(editMenu())
        root.addItem(view)
        root.addItem(windowMenu(application, window))
        root.addItem(help)
        self.navigation = navigation
        navigation?.install(root, view.submenu!, help.submenu!)
        application.mainMenu = root
    }

    private func applicationMenu(_ application: NSApplication, _ name: String) -> NSMenuItem {
        let menu = NSMenu(title: name)
        menu.addItem(item("About " + name, #selector(NSApplication.orderFrontStandardAboutPanel(_:)), "", application))
        menu.addItem(.separator())
        menu.addItem(item("Hide " + name, #selector(NSApplication.hide(_:)), "h", application))
        let others = item("Hide Others", #selector(NSApplication.hideOtherApplications(_:)), "h", application)
        others.keyEquivalentModifierMask = [.command, .option]
        menu.addItem(others)
        menu.addItem(item("Show All", #selector(NSApplication.unhideAllApplications(_:)), "", application))
        menu.addItem(.separator())
        menu.addItem(item("Quit " + name, #selector(NSApplication.terminate(_:)), "q", application))
        return group(menu)
    }

    private func editMenu() -> NSMenuItem {
        let menu = NSMenu(title: "Edit")
        menu.addItem(item("Undo", NSSelectorFromString("undo:"), "z", nil))
        let redo = item("Redo", NSSelectorFromString("redo:"), "z", nil)
        redo.keyEquivalentModifierMask = [.command, .shift]
        menu.addItem(redo)
        menu.addItem(.separator())
        menu.addItem(item("Cut", #selector(NSText.cut(_:)), "x", nil))
        menu.addItem(item("Copy", #selector(NSText.copy(_:)), "c", nil))
        menu.addItem(item("Paste", #selector(NSText.paste(_:)), "v", nil))
        menu.addItem(item("Select All", #selector(NSText.selectAll(_:)), "a", nil))
        return group(menu)
    }

    private func viewMenu(_ window: NSWindow) -> NSMenuItem {
        let menu = NSMenu(title: "View")
        let fullscreen = item("Enter Full Screen", #selector(NSWindow.toggleFullScreen(_:)), "f", window)
        fullscreen.keyEquivalentModifierMask = [.command, .control]
        menu.addItem(fullscreen)
        return group(menu)
    }

    private func windowMenu(_ application: NSApplication, _ window: NSWindow) -> NSMenuItem {
        let menu = NSMenu(title: "Window")
        menu.addItem(item("Minimize", #selector(NSWindow.performMiniaturize(_:)), "m", window))
        menu.addItem(item("Zoom", #selector(NSWindow.performZoom(_:)), "", window))
        menu.addItem(.separator())
        menu.addItem(item("Bring All to Front", #selector(NSApplication.arrangeInFront(_:)), "", application))
        application.windowsMenu = menu
        return group(menu)
    }

    private func helpMenu(_ application: NSApplication) -> NSMenuItem {
        let menu = NSMenu(title: "Help")
        application.helpMenu = menu
        return group(menu)
    }

    private func group(_ menu: NSMenu) -> NSMenuItem {
        let item = NSMenuItem(title: menu.title, action: nil, keyEquivalent: "")
        item.submenu = menu
        return item
    }

    private func item(_ title: String, _ action: Selector, _ key: String, _ target: AnyObject?) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: key)
        item.target = target
        item.keyEquivalentModifierMask = .command
        return item
    }
}
