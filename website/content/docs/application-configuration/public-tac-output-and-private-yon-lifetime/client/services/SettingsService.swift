@Service
final class SettingsService {
    private let applicationClient: ApplicationClient = ApplicationClient()
    var api: String = ""

    func load() async throws {
        let application = try await applicationClient.load()
        api = try Behaviors.values(PublicSettings.self, application).get("api").text
    }
}
