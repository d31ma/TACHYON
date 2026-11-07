import Foundation

@main
struct HeardCaller {
    static let owner = "00000000-0000-4000-8000-000000000003"

    static func ask(_ rest: String) -> String {
        tacNativeInvoke("{\"route\":\"/\",\"service\":\"heard\",\"owner\":\"" + owner + "\"," + rest + "}")
    }

    static func main() {
        TacBridge.emit = { _ in }
        print("init " + ask("\"op\":\"init\""))
        print("first send " + ask("\"op\":\"call\",\"name\":\"send\",\"args\":[1]"))
        print("first heard " + ask("\"op\":\"get\",\"name\":\"heard\""))
        print("second send " + ask("\"op\":\"call\",\"name\":\"send\",\"args\":[2]"))
        print("second heard " + ask("\"op\":\"get\",\"name\":\"heard\""))
        print("dispose " + tacNativeInvoke("{\"op\":\"host-dispose\"}"))
    }
}
