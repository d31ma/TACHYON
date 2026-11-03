@Service
final class SettingsService {
    @Needs
    private let applicationClient: ApplicationClient
    var api: String = ""

    func load() async throws {
        let application = try await applicationClient.load()
        api = try Behaviors.values(PublicSettings.self, application).get("api").text
    }
}
