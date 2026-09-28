#!/usr/bin/env python3

import argparse
from pathlib import Path


def write(root, relative, source):
    destination = root / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(source, encoding="utf-8")


def write_shared_files(root):
    write(root, "client/manifest.json", """{
  "name": "Native Gate",
  "start_url": "/",
  "tac": {
    "application": { "id": "dev.tachyon.desktop-gate", "version": "0.0.1" },
    "styles": ["/shared/native-gate.css"]
  }
}
""")
    write(root, "client/shared/native-gate.css", """body {
  --native-gate: ready; font-family: sans-serif; padding: 24px;
}
button, a { display: block; padding: 8px; margin: 8px; }
""")
    write(root, "client/shared/native-gate.js", "export const marker = 'Shared module ready';\n")


def component_view(label, instance, link):
    return f"""<main aria-label="{label} route"><h1>{label} route</h1>
<p>{{{instance}.marker}}</p><p>{label} count {{{instance}.count}}</p><p>{{{instance}.status}}</p>
<p>{{{instance}.platformStatus}}</p><p>{{{instance}.styleStatus}}</p><p>{{{instance}.boundaryStatus}}</p>
<p>Publish {{{instance}.signal}}</p>
<button on:click="{instance}.verify()">Verify native {label}</button>
{link}<a href="https://example.invalid/">Leave app</a>
</main>
"""


def browser_service(service, instance, number, language):
    return f"""import {{ marker }} from '/shared/native-gate.js';
/**
 * @typedef {{Object}} NativeEvent
 * @property {{string}} text
 */
@Service()
export default class {service} {{
  marker = marker;
  count = -100;
  status = 'Waiting for native verification';
  platformStatus = '';
  styleStatus = '';
  boundaryStatus = '';
  signal = 'waiting';
  /** @param {{NativeEvent}} value */
  @OnEvent
  receive(value) {{ this.signal = value.text; }}
  async verify() {{
    this.count = {number};
    this.status = 'Native {language} ' + await this.doubled();
    this.platformStatus = await this.processId() > 0 ? 'OS ready' : 'OS failed';
    const stylesheet = await fetch('/shared/native-gate.css', {{ signal: AbortSignal.timeout(5000) }});
    const stylesheetType = stylesheet.headers.get('content-type')?.split(';')[0].trim();
    const style = getComputedStyle(document.body).getPropertyValue('--native-gate').trim();
    this.styleStatus = stylesheet.ok && stylesheetType === 'text/css' && style === 'ready'
      ? 'Styles ready' : 'Styles failed';
    let routeRejected = false;
    try {{
      const rejected = JSON.parse(await globalThis.__tachyonNativeHostCall(
        'companion.invoke', JSON.stringify({{ route: '/not-this-route', service: '{instance}', op: 'init' }})));
      routeRejected = Boolean(rejected.error);
    }} catch {{ routeRejected = true; }}
    this.boundaryStatus = routeRejected ? 'Route boundary ready' : 'Route boundary FAILED';
    await this.announce();
  }}
}}
"""


def rust_service(service):
    source = """#[derive(Default)]
struct __SERVICE__ {
    count: i64,
    application_event_publisher: ApplicationEventPublisher,
}
impl __SERVICE__ {
    fn doubled(&self) -> i64 { self.count * 2 }
    fn process_id(&self) -> i64 { std::process::id() as i64 }
    fn announce(&self) -> String {
        self.application_event_publisher.publish_event(crate::models::NativeEvent { text: "received".to_owned() }, false);
        "received".to_owned()
    }
}
"""
    return source.replace("__SERVICE__", service)


def csharp_service(service):
    source = """public class __SERVICE__
{
    public int Count = 0;
    private readonly ApplicationEventPublisher applicationEventPublisher;
    public int Doubled() => Count * 2;
    public int ProcessId() => Environment.ProcessId;
    public string Announce()
    {
        applicationEventPublisher.PublishEvent(new Models.NativeEvent("received"));
        return "received";
    }
}
"""
    return source.replace("__SERVICE__", service)


def write_route(root, platform, route, label, number):
    language = "Rust"
    if route and platform == "windows":
        language = "CSharp"
    component = f"gate/{label.lower()}"
    service = f"Gate{label}Service"
    instance = f"gate{label}"
    link = '<a href="/items/7/">Open second</a>'
    if route:
        link = '<a href="/">Return root</a>'
    write(root, f"client/pages/{route}tac.html", f'<gate-{label.lower()} />\n')
    write(root, f"client/components/{component}/tac.html", component_view(label, instance, link))
    write(root, f"client/services/{service}.js", browser_service(service, instance, number, language))
    if language == "Rust":
        write(root, f"client/services/{service}.rs", rust_service(service))
        write(root, "client/models/NativeEvent.rs", "pub struct NativeEvent {\n    pub text: String,\n}\n")
    else:
        write(root, f"client/services/{service}.cs", csharp_service(service))
        write(root, "client/models/NativeEvent.cs", "public record NativeEvent(string Text);\n")


def create(root, platform):
    if any(root.iterdir()):
        raise RuntimeError("the acceptance fixture directory must be empty")
    write_shared_files(root)
    write_route(root, platform, "", "Root", 7)
    write_route(root, platform, "items/_id/", "Second", 9)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("platform", choices=["linux", "windows"])
    arguments = parser.parse_args()
    arguments.directory.mkdir(parents=True, exist_ok=True)
    create(arguments.directory, arguments.platform)


if __name__ == "__main__":
    main()
