final class SettingsService {
    var title: String = "Products"
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
        let settings = try Behaviors.properties(
            CatalogSettings.self, ["TAC_CATALOG_PAGE_SIZE": "20"]
        )
        let title = "title"
        let values = try Behaviors.values(PublicValues.self, .object([
            "tac": .object(["catalog": .object([title: .string("Products")])])
        ]))
        return (try values.get(title).text, try settings.get("pageSize").integer)
    }
}
