import XCTest

final class AccessibilityAudit: XCTestCase {
    struct Element: Codable {
        let type: String
        let identifier: String
        let label: String
        let value: String
        let width: Double
        let height: Double
    }

    struct Finding: Codable {
        let audit: String
        let summary: String
        let detail: String
        let element: String
    }

    struct Report: Codable {
        var elements: [Element] = []
        var findings: [Finding] = []
        var checks: [String] = []
    }

    static let wait: TimeInterval = 90
    static let typeNames: [XCUIElement.ElementType: String] = [
        .application: "application", .window: "window", .webView: "webView", .other: "other",
        .button: "button", .link: "link", .textField: "textField", .staticText: "staticText",
        .image: "image", .scrollView: "scrollView", .keyboard: "keyboard", .key: "key",
    ]
    static let auditNames: [(XCUIAccessibilityAuditType, String)] = [
        (.contrast, "contrast"), (.elementDetection, "elementDetection"), (.hitRegion, "hitRegion"),
        (.sufficientElementDescription, "sufficientElementDescription"), (.dynamicType, "dynamicType"),
        (.textClipped, "textClipped"), (.trait, "trait"),
    ]

    var report = Report()

    static func typeName(_ type: XCUIElement.ElementType) -> String {
        typeNames[type] ?? "type \(type.rawValue)"
    }

    static func auditName(_ type: XCUIAccessibilityAuditType) -> String {
        auditNames.first { $0.0 == type }?.1 ?? "audit \(type.rawValue)"
    }

    static func flatten(_ snapshot: XCUIElementSnapshot) -> [Element] {
        let element = Element(
            type: typeName(snapshot.elementType), identifier: snapshot.identifier, label: snapshot.label,
            value: (snapshot.value as? String) ?? "", width: snapshot.frame.width, height: snapshot.frame.height)
        return [element] + snapshot.children.flatMap { flatten($0) }
    }

    func expect(_ element: XCUIElement, _ check: String) {
        XCTAssertTrue(element.waitForExistence(timeout: Self.wait), check)
        if element.exists { report.checks.append(check) }
    }

    func publish() {
        let environment = ProcessInfo.processInfo.environment
        guard let data = try? JSONEncoder().encode(report), let text = String(data: data, encoding: .utf8) else { return }
        print("TACHYON_ACCESSIBILITY_REPORT " + text)
        if let file = environment["TACHYON_AUDIT_REPORT"] {
            try? data.write(to: URL(fileURLWithPath: file))
        }
    }

    @MainActor
    func testPageReachesTheAccessibilityApi() throws {
        let environment = ProcessInfo.processInfo.environment
        let names = environment["TACHYON_AUDIT_NAMES"].flatMap { $0.data(using: .utf8) }
            .flatMap { try? JSONDecoder().decode([String: String].self, from: $0) } ?? [:]
        let app = XCUIApplication(bundleIdentifier: environment["TACHYON_AUDIT_APP"] ?? "")
        defer { publish() }
        app.launch()
        let web = app.webViews["tachyon-application"]
        expect(web, "the Tachyon web view is in the accessibility tree")
        expect(web.staticTexts[names["heading", default: ""]], "the heading is static text named by its words")
        expect(web.images[names["image", default: ""]], "the svg with role img is an image named by aria-label")
        expect(web.textFields[names["field", default: ""]], "the input is a text field named by its label")
        expect(web.buttons[names["button", default: ""]], "the button is a button named by its words")
        expect(web.links[names["link", default: ""]], "the link is a link named by its words")
        expect(web.staticTexts[names["idle", default: ""]], "the status text reads its first value")
        report.elements = Self.flatten(try app.snapshot())
        try app.performAccessibilityAudit { issue in
            self.report.findings.append(Finding(
                audit: Self.auditName(issue.auditType), summary: issue.compactDescription,
                detail: issue.detailedDescription,
                element: issue.element.map { Self.typeName($0.elementType) + " '" + $0.label + "'" } ?? "none"))
            return true
        }
        report.checks.append("the accessibility audit ran")
        let field = web.textFields[names["field", default: ""]]
        field.tap()
        field.typeText(names["typed", default: ""])
        web.buttons[names["button", default: ""]].tap()
        expect(web.staticTexts[names["saved", default: ""]], "pressing the button changes the status the system reads")
        web.links[names["link", default: ""]].tap()
        expect(web.staticTexts[names["about", default: ""]], "the link opens the next page in the accessibility tree")
    }
}
