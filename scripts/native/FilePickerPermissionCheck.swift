@main
@MainActor
private struct FilePickerPermissionCheck {
    static func main() {
        let application = NSApplication.shared
        application.setActivationPolicy(.regular)
        DispatchQueue.main.async {
            exercise(application)
            application.stop(nil)
        }
        application.run()
    }

    private static func exercise(_ application: NSApplication) {
        __INITIAL_GRANT__
        precondition(MacOsFilePicker.shared.granted, "The generated manifest must grant filePicker")
        __REGISTER__
        application.activate(ignoringOtherApps: true)
        initialize()
        recordPid()
        check("choose", "selected")
        check("read", "read")
        check("text", "selected through generated service\n")
        MacOsFilePicker.shared.granted = false
        check("read", "denied")
        MacOsFilePicker.shared.granted = true
        check("read", "invalid")
        dispose()
        MacOsFilePicker.shared.disposeAll()
        print("real NSOpenPanel selection/read; revoked grant denied; regrant rejected old handle; activation disposed")
    }

    private static func request(_ operation: String) -> String {
        let prefix = "{\"route\":\"/\",\"owner\":\"6F9619FF-8B86-D011-B42D-00C04FC964FF\",\"service\":\"import\","
        return tacNativeInvoke(prefix + operation + "}")
    }

    private static func initialize() {
        let answer = request("\"op\":\"init\"")
        precondition(!answer.contains("error"), answer)
    }

    private static func check(_ method: String, _ expected: String) {
        let answer = request("\"op\":\"call\",\"name\":\"" + method + "\",\"args\":[]")
        let data = Data(answer.utf8)
        let decoded = try! JSONSerialization.jsonObject(with: data) as! [String: Any]
        precondition(decoded["value"] as? String == expected, method + ": " + answer)
        print(method + ": " + expected.trimmingCharacters(in: .newlines))
    }

    private static func recordPid() {
        let folder = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
        let text = String(ProcessInfo.processInfo.processIdentifier)
        try! text.write(to: folder.appendingPathComponent("picker.pid"), atomically: true, encoding: .utf8)
    }

    private static func dispose() {
        let answer = request("\"op\":\"dispose\"")
        precondition(!answer.contains("error"), answer)
    }
}
