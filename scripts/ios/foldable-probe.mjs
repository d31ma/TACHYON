import { mkdirSync, writeFileSync } from 'node:fs'
import path from 'node:path'

export class FoldableProbe {
  static PROBE_FILE = 'fold-probe.json'

  /** @param {string} project @param {string} relative @param {string} source */
  static write(project, relative, source) {
    const file = path.join(project, relative)
    mkdirSync(path.dirname(file), { recursive: true })
    writeFileSync(file, source)
  }

  /** @param {string} project @param {{ appId: string, color: string, orientation: string }} settings @returns {string} */
  static stage(project, settings) {
    const { write } = FoldableProbe
    const checks = { check: false }
    write(project, 'client/manifest.json', JSON.stringify({
      name: 'FoldProbe', start_url: '/', theme_color: settings.color, background_color: settings.color, orientation: settings.orientation,
      tac: {
        application: { id: settings.appId, version: '1.0.0', window: {} },
        styles: ['/shared/fold-probe.css'],
        names: checks, constants: checks, visibility: checks, duplication: checks, cohesion: checks,
      },
    }))
    write(project, 'client/shared/fold-probe.css', `html, body { margin: 0; background: ${settings.color}; font: 14px sans-serif; }\n`)
    write(project, 'client/components/fold/probe/tac.css', FoldableProbe.componentStyles())
    write(project, 'client/pages/tac.html', '<fold-probe />\n')
    write(project, 'client/components/fold/probe/tac.html', '<main><p>{foldProbe.sequence}</p><div id="safe"></div></main>')
    write(project, 'client/services/FoldProbeService.js', FoldableProbe.script())
    write(project, 'client/services/FoldProbeService.swift', FoldableProbe.swift())
    return project
  }

  /** @returns {string} */
  static componentStyles() {
    return '#safe { position: fixed; visibility: hidden; padding: env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left); }\n'
  }

  /** @returns {string} */
  static script() {
    return `@Service()
export default class FoldProbeService {
  sequence = 0;
  @OnStart()
  initialize() {
    setInterval(() => { void this.measure(); }, 400);
  }
  async measure() {
    const safe = getComputedStyle(document.querySelector('#safe'));
    const inset = name => Math.round(Number.parseFloat(safe.getPropertyValue(name)) || 0);
    this.sequence += 1;
    const page = {
      sequence: this.sequence,
      width: innerWidth, height: innerHeight,
      documentWidth: document.documentElement.clientWidth, documentHeight: document.documentElement.clientHeight,
      safe: { top: inset('padding-top'), right: inset('padding-right'), bottom: inset('padding-bottom'), left: inset('padding-left') },
    };
    await this.note(JSON.stringify(page));
  }
}
`
  }

  /** @returns {string} */
  static swift() {
    return `import Foundation
import UIKit
import WebKit

@Service
final class FoldProbeService {
    var sequence: Int = 0
    private var layouts = 0
    private var lastBounds = ""

    func note(_ page: String) -> Int {
        let native = MainActor.assumeIsolated { Self.facts() }
        let bounds = (native["windowBounds"] as? [Double])?.map { String($0) }.joined(separator: ",") ?? ""
        if bounds != lastBounds {
            layouts += 1
            lastBounds = bounds
        }
        var record: [String: Any] = ["native": native, "layouts": layouts]
        record["page"] = (try? JSONSerialization.jsonObject(with: Data(page.utf8))) ?? [:]
        let directory = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let data = (try? JSONSerialization.data(withJSONObject: record, options: [.sortedKeys])) ?? Data()
        try? data.write(to: directory.appendingPathComponent("${FoldableProbe.PROBE_FILE}"), options: .atomic)
        return layouts
    }

    @MainActor
    private static func facts() -> [String: Any] {
        let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
        guard let scene = scenes.first(where: { $0.activationState == .foregroundActive }) ?? scenes.first,
              let window = scene.windows.first(where: { $0.isKeyWindow }) ?? scene.windows.first else {
            return ["error": "no window scene"]
        }
        let screen = scene.screen
        var facts: [String: Any] = [:]
        facts["scenes"] = scenes.count
        facts["screenBounds"] = rectangle(screen.bounds)
        facts["screenNative"] = rectangle(screen.nativeBounds)
        facts["scale"] = Double(screen.scale)
        facts["sceneBounds"] = rectangle(scene.coordinateSpace.bounds)
        facts["windowBounds"] = rectangle(window.bounds)
        facts["windowOnScreen"] = rectangle(window.convert(window.bounds, to: screen.coordinateSpace))
        facts["safe"] = insets(window.safeAreaInsets)
        facts["orientation"] = scene.interfaceOrientation.rawValue
        facts["horizontalClass"] = window.traitCollection.horizontalSizeClass.rawValue
        if let web = webView(window) {
            facts["webFrame"] = rectangle(web.convert(web.bounds, to: window))
            facts["webSafe"] = insets(web.safeAreaInsets)
        }
        return facts
    }

    private static func rectangle(_ rectangle: CGRect) -> [Double] {
        [rectangle.origin.x, rectangle.origin.y, rectangle.size.width, rectangle.size.height].map { Double($0) }
    }

    private static func insets(_ insets: UIEdgeInsets) -> [String: Double] {
        ["top": Double(insets.top), "left": Double(insets.left), "bottom": Double(insets.bottom), "right": Double(insets.right)]
    }

    @MainActor
    private static func webView(_ view: UIView) -> WKWebView? {
        if let web = view as? WKWebView { return web }
        for child in view.subviews {
            if let web = webView(child) { return web }
        }
        return nil
    }
}
`
  }
}
