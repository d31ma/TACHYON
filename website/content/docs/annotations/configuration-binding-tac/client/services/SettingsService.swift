final class SettingsService {
    var title: String = "Catalog"
    var pageSize: Int = 20

    init() {
        (title, pageSize) = Self.settingsOrFallback()
    }

    private static func settingsOrFallback() -> (String, Int) {
        do {
            return try readSettings()
        } catch {
            return ("Configuration unavailable", 20)
        }
    }

    private static func readSettings() throws -> (String, Int) {
        let published = try BehaviorJSON.parse(
            "{\"tac\":{\"catalog\":{\"title\":\"Catalog\",\"page\":{\"size\":20},\"cache\":{\"enabled\":true}}}}")
        let catalog = try Behaviors.properties(CatalogSetup.self, published)
        let values = try Behaviors.values(PublicValues.self, published)
        let context = try Behaviors.create(
            CatalogSetup.self, partition: "public-catalog", signal: BehaviorSignal(),
            profiles: ["public"], properties: published)
        context.dispose()
        return (try values.get("title").text, try catalog.get("pageSize").integer)
    }
}
