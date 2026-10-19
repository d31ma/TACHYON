@MainActor
final class MacOsTray: NSObject, NSMenuDelegate {
    private let statusItem: NSStatusItem
    private let items: [MacOsTrayItem]
    private let ready: (String) -> Bool
    private let perform: (String) -> Bool
    private let show: (String?) -> Void
    private let pageShown: () -> Bool
    private var declared: [NSMenuItem] = []

    init(_ name: String, _ icon: String, _ template: Bool, _ tooltip: String, _ items: [MacOsTrayItem],
         _ ready: @escaping (String) -> Bool, _ perform: @escaping (String) -> Bool,
         _ show: @escaping (String?) -> Void, _ pageShown: @escaping () -> Bool) {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        self.items = items
        self.ready = ready
        self.perform = perform
        self.show = show
        self.pageShown = pageShown
        super.init()
        statusItem.button?.toolTip = tooltip
        statusItem.button?.setAccessibilityLabel(tooltip)
        if let image = MacOsTray.image(icon, template) {
            statusItem.button?.image = image
        } else {
            statusItem.button?.title = name
        }
        statusItem.menu = menu(name)
    }

    func menuNeedsUpdate(_ menu: NSMenu) {
        declared.forEach { (item: NSMenuItem) -> Void in item.isEnabled = enabled(item) != nil }
    }

    @objc private func openWindow(_ item: NSMenuItem) {
        show(nil)
    }

    @objc private func run(_ item: NSMenuItem) {
        guard let entry = enabled(item) else {
            NSSound.beep()
            return
        }
        if !entry.route.isEmpty {
            show(entry.route)
        } else if !perform(entry.id) {
            NSSound.beep()
        }
    }

    private func enabled(_ item: NSMenuItem) -> MacOsTrayItem? {
        guard items.count <= 32, let entry = item.representedObject as? MacOsTrayItem,
              let known = items.first(where: { (declared: MacOsTrayItem) -> Bool in declared.id == entry.id }) else { return nil }
        guard known.route.isEmpty, !known.event.isEmpty else { return known }
        return pageShown() && ready(known.event) ? known : nil
    }

    private func menu(_ name: String) -> NSMenu {
        let menu = NSMenu(title: name)
        menu.autoenablesItems = false
        menu.delegate = self
        let opener = NSMenuItem(title: "Open " + name, action: #selector(openWindow(_:)), keyEquivalent: "")
        opener.target = self
        menu.addItem(opener)
        if !items.isEmpty { menu.addItem(.separator()) }
        declared = items.map { (entry: MacOsTrayItem) -> NSMenuItem in
            let item = NSMenuItem(title: entry.label, action: #selector(run(_:)), keyEquivalent: "")
            item.identifier = NSUserInterfaceItemIdentifier(entry.id)
            item.target = self
            item.representedObject = entry
            return item
        }
        declared.forEach { (item: NSMenuItem) -> Void in menu.addItem(item) }
        menu.addItem(.separator())
        let quit = NSMenuItem(title: "Quit " + name, action: #selector(NSApplication.terminate(_:)), keyEquivalent: "")
        quit.target = NSApplication.shared
        menu.addItem(quit)
        return menu
    }

    private static func image(_ icon: String, _ template: Bool) -> NSImage? {
        guard let resources = Bundle.main.resourceURL else { return nil }
        let root = resources.appendingPathComponent("WebBundle").standardizedFileURL
        let file = root.appendingPathComponent(icon).standardizedFileURL
        guard file.path.hasPrefix(root.path + "/"), let image = NSImage(contentsOf: file) else { return nil }
        let height: CGFloat = 18
        let width = image.size.height > 0 ? image.size.width * height / image.size.height : height
        image.size = NSSize(width: width, height: height)
        image.isTemplate = template
        return image
    }
}
