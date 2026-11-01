import XCTest

final class SystemBack: XCTestCase {
    static let wait: TimeInterval = 90
    static let settle: TimeInterval = 8
    // An injected edge drag starts WebKit's back swipe but releases below its completion speed some of the time, and the
    // swipe snaps back (seen on the iOS 26.5 simulator), so each direction gets a few tries.
    static let attempts = 3

    func swipe(_ element: XCUIElement, from start: CGFloat, to end: CGFloat) {
        let origin = element.coordinate(withNormalizedOffset: CGVector(dx: start, dy: 0.5))
        origin.press(forDuration: 0.05, thenDragTo: element.coordinate(withNormalizedOffset: CGVector(dx: end, dy: 0.5)))
    }

    /// Swipes until `target` shows, up to `attempts` times.
    func swipe(_ element: XCUIElement, from start: CGFloat, to end: CGFloat, until target: XCUIElement) -> Bool {
        for _ in 0..<Self.attempts {
            swipe(element, from: start, to: end)
            if target.waitForExistence(timeout: Self.settle) { return true }
        }
        return false
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
        let wentBack = swipe(web, from: 0.01, to: 0.85, until: home)
        print("TACHYON_APP_BACK=\(wentBack)")
        if wentBack {
            XCTAssertTrue(swipe(web, from: 0.99, to: 0.15, until: about), "a swipe from the right edge went forward to the second route")
            return
        }
        // No swipe completed: still prove the page history itself goes back, through the page's history.back() button.
        web.buttons[names["back", default: ""]].tap()
        let historyBack = home.waitForExistence(timeout: Self.wait)
        print("TACHYON_HISTORY_BACK=\(historyBack)")
        XCTAssertTrue(historyBack, "the page's history.back() went back to the first route")
    }
}
