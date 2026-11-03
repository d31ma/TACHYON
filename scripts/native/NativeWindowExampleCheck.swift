@main
struct NativeWindowExampleCheck {
    @MainActor
    static func main() {
        let application = NSApplication.shared
        application.setActivationPolicy(.accessory)
        let window = NSWindow(contentRect: NSRect(x: 80, y: 80, width: 300, height: 200), styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        NativeBridge.shared.window = window
        __REGISTER__
        let prefix = "{\"route\":\"/\",\"owner\":\"6F9619FF-8B86-D011-B42D-00C04FC964FF\",\"service\":\"report\","
        let started = __CALL__(prefix + "\"op\":\"init\"}")
        precondition(!started.contains("error"), started)
        let resized = __CALL__(prefix + "\"op\":\"call\",\"name\":\"resize\",\"args\":[]}")
        precondition(!resized.contains("error"), resized)
        precondition(Int(window.contentRect(forFrameRect: window.frame).width) == 1280)
        precondition(Int(window.contentRect(forFrameRect: window.frame).height) == 800)
        let renamed = __CALL__(prefix + "\"op\":\"call\",\"name\":\"rename\",\"args\":[]}")
        precondition(!renamed.contains("error"), renamed)
        precondition(window.title == "Quarterly report")
        let denied = NativeBridge.shared.handle("window.close", "{}")
        precondition(!denied.contains("\"ok\":true"), denied)
        precondition(window.title == "Quarterly report")
        _ = __CALL__(prefix + "\"op\":\"dispose\"}")
        window.close()
        NativeBridge.shared.window = nil
        print("Native ReportService changed its owned macOS window title and size; ungranted close refused")
    }
}
