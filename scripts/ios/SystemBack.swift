import XCTest

final class SystemBack: XCTestCase {
    static let wait: TimeInterval = 90

    func swipe(_ element: XCUIElement, from start: CGFloat, to end: CGFloat) {
        let origin = element.coordinate(withNormalizedOffset: CGVector(dx: start, dy: 0.5))
        origin.press(forDuration: 0.05, thenDragTo: element.coordinate(withNormalizedOffset: CGVector(dx: end, dy: 0.5)))
    }

    @MainActor
    func testEdgeSwipesGoBackAndForwardThroughThePageHistory() throws {
        let environment = ProcessInfo.processInfo.environment
        let names = environment["TACHYON_BACK_NAMES"].flatMap { $0.data(using: .utf8) }
            .flatMap { try? JSONDecoder().decode([String: String].self, from: $0) } ?? [:]
        let app = XCUIApplication(bundleIdentifier: environment["TACHYON_BACK_APP"] ?? "")
        app.launch()
        let web = app.webViews["tachyon-application"]
        XCTAssertTrue(web.waitForExistence(timeout: Self.wait), "the Tachyon web view is on screen")
        let home = web.staticTexts[names["heading", default: ""]]
        let about = web.staticTexts[names["about", default: ""]]
        XCTAssertTrue(home.waitForExistence(timeout: Self.wait), "the first route is on screen")
        web.links[names["link", default: ""]].tap()
        XCTAssertTrue(about.waitForExistence(timeout: Self.wait), "the link opened the second route")
        swipe(web, from: 0.01, to: 0.85)
        XCTAssertTrue(home.waitForExistence(timeout: Self.wait), "a swipe from the left edge went back to the first route")
        swipe(web, from: 0.99, to: 0.15)
        XCTAssertTrue(about.waitForExistence(timeout: Self.wait), "a swipe from the right edge went forward to the second route")
    }
}
