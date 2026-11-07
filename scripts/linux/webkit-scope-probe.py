#!/usr/bin/env python3

import json

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("WebKit", "6.0")
from gi.repository import GLib, Gtk, WebKit


class WebKitScopeProbe:
    WAIT_SECONDS = 60
    COLOR = "rgb(255, 0, 0)"
    PAGE = """<!doctype html><html><head><style>
@scope (tachyon-component[data-tac-scope="card"]) to (tachyon-component[data-tac-scope]) {
  p { color: rgb(255, 0, 0); }
}
</style></head><body>
<p id="outside">outside</p>
<tachyon-component data-tac-scope="card"><p id="inside">inside</p>
<tachyon-component data-tac-scope="other"><p id="nested">nested</p></tachyon-component>
</tachyon-component>
</body></html>"""
    SCRIPT = """JSON.stringify({
  inside: getComputedStyle(document.getElementById('inside')).color,
  outside: getComputedStyle(document.getElementById('outside')).color,
  nested: getComputedStyle(document.getElementById('nested')).color,
  rules: [...document.styleSheets[0].cssRules].map((rule) => rule.constructor.name)
})"""

    def __init__(self):
        self.result = {}
        self.application = Gtk.Application(application_id="dev.tachyon.WebKitScopeProbe")
        self.application.connect("activate", self.activate)

    def activate(self, application):
        window = Gtk.ApplicationWindow(application=application)
        view = WebKit.WebView()
        view.connect("load-changed", self.loaded)
        window.set_child(view)
        view.load_html(WebKitScopeProbe.PAGE, "about:blank")
        window.present()
        GLib.timeout_add_seconds(WebKitScopeProbe.WAIT_SECONDS, application.quit)

    def loaded(self, view, event):
        if event == WebKit.LoadEvent.FINISHED:
            view.evaluate_javascript(WebKitScopeProbe.SCRIPT, -1, None, None, None, self.finished)

    def finished(self, view, task):
        self.result.update(json.loads(view.evaluate_javascript_finish(task).to_string()))
        self.application.quit()

    def main(self):
        self.application.run([])
        colors = [self.result.get(name) == WebKitScopeProbe.COLOR for name in ("inside", "nested", "outside")]
        self.result["webkit"] = f"{WebKit.get_major_version()}.{WebKit.get_minor_version()}.{WebKit.get_micro_version()}"
        self.result["scoped"] = colors == [True, False, False]
        print(json.dumps(self.result), flush=True)


if __name__ == "__main__":
    WebKitScopeProbe().main()
