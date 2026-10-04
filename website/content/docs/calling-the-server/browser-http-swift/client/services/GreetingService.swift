@Service
final class GreetingService {
    @Needs
    private let greetingClient: GreetingClient
    @Needs
    private let overrideClient: OverrideClient
    var message: String = ""
    var custom: String = ""
    var loads: Int = 0
    var clears: Int = 0

    func ask() async throws {
        loads += 1
        message = try await greetingClient.read().message
    }

    func clear() async throws {
        try await greetingClient.clear()
        clears += 1
    }

    func askCustom() async throws {
        custom = try await overrideClient.read().message
    }
}
