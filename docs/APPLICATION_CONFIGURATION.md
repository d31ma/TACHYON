# Application configuration

Tachyon splits configuration in two ways: by realm (Tac on the client, Yon on
the server) and by secrecy (public or private).

| File | Owner | Secrecy | What it holds |
| --- | --- | --- | --- |
| `client/manifest.json` | Tac | Public | The web app manifest. Its `tac` member is Tac's public configuration. The whole file goes to every visitor ([ADR 0034](adr/0034-tac-configuration-is-the-web-app-manifest.md)). |
| `server/application.json` | Yon only | Not secret | Application Configuration v3. The file is the `yon` namespace itself. Tachyon reads its members as `yon.*` paths. The file has no root key. |
| `.env` in the project root | Yon only | Private | Yon secrets. |

ADR means Architecture Decision Record.

Both kinds of files (JSON and `.env`) use bounded parsers. They add no new
dependency. The configuration parser itself adds no dependency for a file
format.

At startup, Yon also initializes the datasource and Redis resources that you
configure. For connection settings and repository injection, see
[Managed storage](MANAGED_STORAGE.md).

`yon.datasource.driver` accepts:

| Driver | `yon.datasource.url` | Credentials |
| --- | --- | --- |
| `postgresql` | `postgresql://host:5432/catalog` | `.env` username/password |
| `mysql`, `mariadb` | `mysql://host:3306/catalog` | `.env` username/password |
| `sqlite` | `sqlite::memory:` or `sqlite:///absolute/existing.db` | None |
| `sqlserver` | `sqlserver://host:1433/catalog` | `.env` username/password |
| `fylo` | `fylo:///absolute/root` | None; no TLS keys either |

The `fylo` driver ([ADR 0032](adr/0032-fylo-datasource.md)) has special rules:

- It takes only `driver` and `url`.
- Its root directory must already exist, and the path must be written in
  canonical form.
- It runs a local `fylo` subprocess. It does not connect to a database server.
- So `YON_DATASOURCE_USERNAME` and `YON_DATASOURCE_PASSWORD` do not apply to it.
- Its collections are created by the application with
  `dataSource.createCollection(name)`, not by this file. See
  [MANAGED_STORAGE](MANAGED_STORAGE.md#fylo) for names, documents, scroll and
  the queue.

## Files and paths

`client/manifest.json` is the web app manifest. A browser installs the app from
this file:

```json
{
  "name": "Catalog",
  "short_name": "Catalog",
  "start_url": "/",
  "display": "standalone",
  "tac": { "api": { "url": "/api" } }
}
```

`server/application.json` is Yon's server configuration. It holds no secrets:

```json
{
  "server": { "host": "127.0.0.1", "port": 8080 },
  "datasource": { "driver": "postgresql", "url": "postgresql://localhost:5432/catalog" },
  "catalog": { "page": { "size": 20 } }
}
```

`.env` holds secrets. The values below are example placeholders. They are never
real credentials:

```dotenv
YON_DATASOURCE_USERNAME=replace-me
YON_DATASOURCE_PASSWORD=replace-me
YON_AUTH_SIGNING_KEY=replace-me
```

Rules for `.env`:

- Do not commit `.env`.
- When it helps, commit a `.env.example` file without secrets.
- Tachyon loads only the exact `.env` file in the project root.
- A production secret manager may create that file. The file must pass the
  same checks as other private configuration inputs: it must be a regular file,
  and Tachyon does not follow links to it.

Each underscore in a name starts another nested key:

| Application name | Canonical path |
| --- | --- |
| TAC_API_URL | tac.api.url |
| YON_SERVER_PORT | yon.server.port |
| YON_CATALOG_PAGE_SIZE | yon.catalog.page.size |
| YON_DATASOURCE_PASSWORD | yon.datasource.password |
| YON_AUTH_SIGNING_KEY | yon.auth.signing.key |

Path rules:

- A JSON path segment starts with a lowercase ASCII letter. The rest is
  lowercase letters or digits.
- Underscores, dots and empty segments are never literal JSON keys.
- Arrays are values. Environment names cannot index into an array.
- Numbers and booleans stay typed JSON values. Secrets are literal strings.
- Tachyon does not silently pick one value when paths conflict. These cases
  fail:
  - one path is a scalar and another path needs an object at the same place;
  - a secret tries to overwrite a value from the JSON configuration.

## Field reference

### `client/manifest.json`

This file is a standard web app manifest:

- A browser reads its standard members to install the application.
- Tachyon reads a few of them for native builds.
- The `tac` member is the whole Tac configuration (ADR 0052). There is no
  `tac.config.js` and no `tachyon.json`; a build refuses both and says where
  each value moved.

The whole file is public. Tachyon publishes it at `/manifest.webmanifest`.
Never put a secret in it, including in `tac.application`.

| Member | Meaning | Read by |
| --- | --- | --- |
| `name` | The application's name. A native build uses it to name the app and the executable (it uses `short_name` when `name` is absent). 1 to 64 printable characters. | browser, native |
| `short_name` | The label that a launcher shows. | browser, native |
| `start_url` | The route where the application opens, and the entry route of a native build. It must be a canonical local route. | browser, native |
| `icons` | `[{ "src", "sizes" }]`, with local bundle paths. A native build uses the largest raster icon. A manifest with only SVG icons gets no native icon. | browser, native |
| `display`, `theme_color`, `background_color`, … | Standard members for browser install. Tachyon publishes them as written. | browser |
| `tac` | Tac's public configuration (ADR 0034). It binds with `TAC_` names and `tac.` paths. Keys are lowercase. Names that look like secrets are not allowed. The Tachyon members below are read by the build and are not service configuration. | Tac services |
| `tac.application.id` | Bundle identifier in reverse-DNS form, such as `com.example.catalog`. Without it, the id is derived from the project folder. | native |
| `tac.application.version` | Portable version string, such as `1.4.0` (1.0.0 unless set). | native |
| `tac.application.window` | `width`/`height`: 1024 by 768 unless set, each from 64 to 16384. `minWidth`/`minHeight`. `controls`: the window controls that a page may use (`close`, `fullscreen`, `maximize`, `minimize`, `resize`, `title`). A page may use none unless they are listed. `titleBar`: `standard` (default) or `integrated`; on macOS an integrated title bar lets the page reach the top edge under the real window buttons, sets `--tac-titlebar-height`, `--tac-titlebar-leading-inset` and `--tac-titlebar-trailing-inset`, and moves the window from elements marked `data-tac-drag-region` (ADR 0098); other targets keep the standard title bar and print a note. `titleBarButtonInset`: `{ "leading", "top" }` in points (0 to 200), only with `integrated`. | native |
| `tac.nativeCapabilities` | The operating system features that native services may use, such as `["filePicker"]`. Each name may appear once; `filePicker` is the only name today. Without the grant, the file picker answers `denied` before any dialog opens. The grant does not replace the user's consent or the platform's signing entitlements (ADR 0089). | build, native |
| `tac.origin` | The site's public address, such as `https://example.com`, without a path. It turns on the built-in `sitemap.xml` and `robots.txt`, and a `canonical` or `image` that starts with `/` is joined to it. | build |
| `tac.scripts`, `tac.styles` | Up to 8 local paths each, such as `/shared/app.js`, linked into every page. | build |
| `tac.scriptSources` | For each third-party script in `tac.scripts`, its `package`, `version`, `license` and pinned `sha256`. The file stays in `client/shared/` and is listed in both settings (`TY1365`); bytes that do not match the pinned SHA-256 fail with `TY1403`. | build |
| `tac.styleSources` | The kind of each style in `tac.styles`: `{"kind": "foundation"}` for the project's own foundation styles, or `{"kind": "vendor", ...}` with the same `package`, `version`, `license` and `sha256` as a script source. | build |
| `tac.browserApis` | The browser APIs that need the user's consent and that services may use: any of `clipboard`, `share`, `notifications` and `geolocation`, each once. Any other name stops the build (`TY1601`); the page refuses an API that is not listed (ADR 0065). | build, browser |
| `tac.elements` | The components that the build also exports as custom elements, such as `["shop-counter"]`. A name without a hyphen, or a name that is not a component, stops the build (`TY1358`, ADR 0086). | build |
| `tac.application.menus` | The native File, View and Help menu items. An item's action names a client Rust application service, its method and an optional event model (`TY1370`, ADR 0097). | native |
| `tac.python` | `{"runtime": "pyodide-314.0.7", "profile": "offline"}` turns on Python services on the web. Any other value stops the build. `ty init` and `ty add language client py` write it (ADR 0093). | build |
| `tac.go.compiler` | `"tinygo"` or `"go"`: the compiler for Go web services. Without it, a release build tries TinyGo first and falls back to Go. | build |
| `tac.dotnet.aot` | `true` compiles C# and F# web services ahead of time in a release build: faster for heavy computing, but a larger download. | build |
| `tac.budgets.script`, `tac.budgets.webassembly`, `tac.budgets.check` | The size budget of every route in kilobytes: 200 for scripts and 3584 for WebAssembly unless set. `check: false` turns the budget check (`TY1337`) off. | build |
| `tac.cache` | Up to 64 `{ "path", "policy" }` rules for the service worker; `policy` is `cache-first`, `network-first` or `no-store`, and `*` in `path` matches the rest of a path. | build |
| `tac.metadata` | `defaults` and `routes`: page head values (`title`, `description`, `canonical`, `image`, `siteName`, `lang`, `noindex`). `routes` is keyed by route, such as `/docs/_topic`; each page gets its route entry over the defaults. | build |
| `tac.sitemap` | For a page route with one path variable, the values to list: `{ "data": "/shared/data/items.json", "values": "/items", "field": "id" }`. `values` is a JSON pointer to a list; `field` picks a member of each listed object. | build |
| `tac.outgoing.hosts` | The hosts that page code may send requests to, such as `["api.example.com", "*.cdn.example.com"]`: names without scheme or port. The build refuses a request to another host (`TY1326`), and the page's `connect-src` allows only these hosts (ADR 0068). | build, browser |
| `tac.cohesion.check` | `false` turns off the cohesion check (`TY1314`, ADR 0063) for Tac services. Use it only while you migrate. | build |
| `tac.duplication.check` | `false` turns off the duplication check (`TY1315`, ADR 0064) for Tac code. Use it only while you migrate. | build |
| `tac.implicitmembers.suggest` | `true` turns on suggestions (`TY1362`) to drop a receiver such as `this.` that the language does not need before a class member. Off by default. Keep a receiver when a local name hides the member, inside a closure, and in a Dart constructor parameter such as `this.name`. This is a build setting, not service configuration. | build, migrate |
| `tac.dependencies.check`, `tac.names.check`, `tac.signatures.check`, `tac.constants.check`, `tac.visibility.check`, `tac.types.check` | `false` turns off one design rule (`TY1327` to `TY1330`, ADR 0069; `TY1333`, ADR 0071; `TY1356`, ADR 0079, off until set to `true`) for Tac code: dependencies point down, plain-English names, small signatures, named values. Use them only while you migrate. | build |
| `tac.tests.check` | `false` turns off the test-case rule (`TY1339`) for Tac code. Use it only while you migrate. | build |
| `tac.accessibility.check` | `false` turns off the accessible-view check (`TY1336`, ADR 0074). Use it only while you migrate. | build |
| `tac.simplify.suggest` | `false` hides the simpler-form suggestions for Tac code. | build |
| `tac.names.words` | Words of your domain, such as a brand or a product name, that Tac names may use (`TY1328`, ADR 0069), for example `["fylo"]`. Lowercase. | build |

### `server/application.json`

This file is Yon's configuration. The file itself is the `yon` namespace: its
members are `yon.*` paths, with no root key. Tachyon reads `server`,
`datasource`, `redis`, `topics`, `outgoing`, `cohesion`, `duplication`, `dependencies`, `names`, `signatures`, `constants`, `visibility`, `types`, `tests`, `simplify` and `implicitmembers`. Every other member belongs to the application. A
profile file has the same shape. `server` is a closed set: it holds only
`host`, `port`, `hosts`, `proxies` and `environment`, and any other member
stops the server with "yon.server supports only host, port, hosts, proxies and
environment."

| Path | Meaning |
| --- | --- |
| `server.host` | The address that `ty serve` and `ty start` bind. `127.0.0.1` unless set. `localhost` or a name under `.localhost`, such as `shop.localhost`, binds `127.0.0.1` and the ready line prints the name (ADR 0094). An address that is not loopback also needs `--allow-non-loopback`. `--host` overrides it. |
| `server.hosts` | Extra `Host` names a loopback server answers, such as `["preview.example.com", "*.tunnel.example"]`: names without scheme, port or path. Localhost names, IP addresses and the `tac.origin` host are always answered; other names get `403` with `TY1366`. With `--allow-non-loopback`, the `Host` check runs only when this list is set (ADR 0094). |
| `server.proxies` | The reverse proxies whose forwarding headers count, as IP addresses or CIDR ranges such as `["10.0.0.0/8", "192.0.2.10"]`; a host name is refused. Empty by default, so `ty` removes `Forwarded`, `X-Forwarded-*` and `X-Real-IP` from every other peer and writes the real client address. From a listed proxy it keeps them, appends its own hop and sets `X-Real-IP` to the nearest address that is not a listed proxy (ADR 0096). |
| `server.environment` | The environment variables of `ty` that handler processes may see, beyond the baseline (`PATH`, `LANG`, `LC_ALL` and `TMPDIR` on Unix). Each entry is a name, such as `HOME`, or a prefix pattern that ends in `*`, such as `HD_*` or `AWS_*`. Empty by default. `*` alone is refused, and so is any entry that names or covers `ty`'s own variables (`TAC_*`, `TACHYON_*`, `YON_*`, `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, `NO_PROXY`, `NODE_USE_ENV_PROXY`), with `TY2010`. Every Yon language, Gateway command and relay program gets the same list. See [Environment variables for handlers](#environment-variables-for-handlers). |
| `server.port` | Port, 0 to 65535. `8080` unless set. 0 picks a free port. `--port` overrides it. |
| `datasource.driver` | `postgresql`, `mysql`, `mariadb`, `sqlserver`, `sqlite` or `fylo`. |
| `datasource.url` | The database location, without credentials: `postgresql://db:5432/catalog`, `sqlite:///var/lib/catalog.db`, `sqlite::memory:`, or an absolute FYLO root. |
| `datasource.username` | The database user. Usually it is `YON_DATASOURCE_USERNAME` in `.env`, next to `YON_DATASOURCE_PASSWORD`. Only `.env` may hold the password. SQLite and FYLO take neither. |
| `datasource.tls.ca` | Absolute path of a CA (certificate authority) certificate to trust. |
| `redis.url` | `redis://`, or `rediss://` for TLS. |
| `redis.username` | The Redis user. Usually it is `YON_REDIS_USERNAME` in `.env`, next to `YON_REDIS_PASSWORD`. Only `.env` may hold the password. |
| `topics.directory` | The folder of the topic logs (ADR 0061). `data/topics` unless set. A plain path inside the project, such as `storage/events`, or an absolute path with no symbolic link in it. A path with `..`, a path inside `.tachyon`, and a folder that a symbolic link leads out of the project stop the server (`TY1303`). |
| `outgoing.hosts` | The hosts that Yon code may send requests to (`yon.outgoing.hosts`), such as `["api.example.com"]`: names without scheme or port, or `*.example.com`. The build refuses a request to another host (`TY1326`), and at run time handler processes send their requests through a local guard that answers 403 to other hosts. With no list, every outgoing request is refused (ADR 0068). |
| `cohesion.check` | `false` turns off the cohesion check (`TY1314`, ADR 0063) for Yon services and controllers. Use it only while you migrate. |
| `duplication.check` | `false` turns off the duplication check (`TY1315`, ADR 0064) for Yon code. Use it only while you migrate. |
| `implicitmembers.suggest` | `true` turns on suggestions (`TY1362`) to drop a receiver such as `this.` that the language does not need before a class member in Yon code. Off by default. Keep a receiver when a local name hides the member or the code is in a nested scope. |
| `dependencies.check`, `names.check`, `signatures.check`, `constants.check`, `visibility.check`, `types.check` | `false` turns off one design rule (`TY1327` to `TY1330`, ADR 0069; `TY1333`, ADR 0071; `TY1356`, ADR 0079, off until set to `true`) for Yon code. Use them only while you migrate. |
| `tests.check` | `false` turns off the test-case rule (`TY1339`) for Yon code. Use it only while you migrate. |
| `simplify.suggest` | `false` hides the simpler-form suggestions for Yon code. |
| `names.words` | Words of your domain that Yon names may use (`TY1328`, ADR 0069), for example `["fylo"]`. Lowercase. |
| anything else | The application's own settings. For example, `catalog.page.size` is `yon.catalog.page.size` and `YON_CATALOG_PAGE_SIZE`. |

Keys use lowercase letters and digits. A file holds at most 128 keys, 16
levels and 64 KiB.

## Secrets and validation

### Which secrets are allowed

- `.env` accepts only YON_ secret names. Tachyon rejects TAC_ secrets.
- Treat everything that goes to a browser or a native client as public.
- Application JSON rejects recognized credential keys and URLs that contain
  credentials. The recognized keys are: password, passwd, secret/secrets,
  token/tokens, credential/credentials, privatekey, signingkey, apikey,
  accesskey, clientsecret and private/signing/api/access.key.
- Tachyon cannot recognize every unusual secret name by its spelling.
  Developers must still put those values in `.env`.

### How Tachyon reads `.env`

The dotenv reader accepts:

- bounded literal assignments;
- blank lines;
- full-line comments that start with #.

Parsing rules:

- It trims whitespace around values.
- It removes matching single or double quotes. It does not process escapes.
- An unquoted # is literal data. It does not start an inline comment.
- It does not run a shell, run an export command, interpolate variables, read
  multiline secrets or change the process environment.
- JSON environment placeholders are no longer supported.
- Reserved supervisor names and deployment-policy names cannot be secret
  assignments.

### Environment variables for handlers

`ty` clears the environment of every handler process. A handler sees only the
baseline and the names that `server.environment` lists, so configuration, not
code, decides what a handler can read. Use it for cloud SDKs and tools that
read their credentials from the environment:

```json
{
  "server": {
    "environment": ["HOME", "AWS_*", "HD_*"]
  }
}
```

With this list, `AWS_ACCESS_KEY_ID` and `HD_REGION` reach handlers in every
Yon language as ordinary environment variables (`process.env`, `os.environ`,
`os.Getenv`, `System.getenv` and so on). `OTHER_SECRET` does not. Values stay
in the environment of `ty`: they are never written to the configuration, to
logs or to diagnostics. A refused entry names the entry, never a value.
Settings that belong to the application itself, such as a database password,
still go in `.env` as `YON_` names and reach handlers through the
configuration snapshot.

### How Tachyon validates JSON

JSON files reject:

- duplicate keys;
- a `yon` or `tac` root key;
- comments and trailing commas;
- invalid key segments;
- secret fields;
- too large a size, depth or key count.

Inputs are limited to 64 KiB. The resolved transport is limited to 16 KiB.
Diagnostics and Debug output leave out values. Never log or return the private
resolved snapshot.

## Profiles and precedence

- Yon selects one `server/application-<profile>.json` file through the
  `YON_PROFILE` variable of the parent process.
- The profile name is bounded and safe to use in a path.
- If you explicitly select a profile file and it is missing, Yon fails.
- Objects merge recursively. Arrays replace each other.

Endpoint precedence, from highest to lowest:

1. Explicit CLI --host / --port.
2. Selected profile value.
3. Base server/application.json.
4. Existing command default.

Other rules:

- Application variables in the parent process, such as YON_PORT and
  YON_DATASOURCE_PASSWORD, do not override file settings. They do not supply
  secrets either.
- The operator's environment policy for runtime and isolation is separate. It
  does not change.
- Exposure on an address that is not loopback still requires
  --allow-non-loopback.
- The behavior context activates WhenProfile annotations explicitly. Selecting a
  file does not automatically switch the DI (dependency injection) container.

## Public Tac and private Yon snapshots

### Tac: public

Tac's configuration is the `tac` member of `client/manifest.json`.

- Discovery validates it with the same key rules as `server/application.json`.
  It also rejects names that look like secrets, in the same way.
- The build publishes the manifest, including the `tac` member, at
  `/manifest.webmanifest`. Every page links to it, so a browser can install the
  app from it.
- Web bundles and staged native bundles carry the same file.
- The manifest's own members (`name`, `icons`, `start_url`) belong to the
  browser. They are not configuration. Only `tac` binds.
- The Tachyon members of `tac` in the field reference above (`origin`,
  `application`, `scripts`, `styles`, `outgoing`, `budgets`, the `check` and
  `suggest` switches, `python`, `go`, `dotnet` and the others) are build
  settings. Tachyon removes them before services bind, so `Settings` and
  `Setting` never see them.
- No profile overlays it.
- Compilation never opens `.env` or a Yon profile. It never publishes the `yon`
  namespace.
- A build succeeds without production credentials.
- Native packaging is not secret storage.

### Yon: private

`server/application.json` and its profiles configure Yon only. If one of them
still has a `tac` section, the build stops with a message that names the
manifest. `ty migrate` also reports it.

### Loading public settings in a browser service

A browser service can load public JSON through a declarative client and then
bind it explicitly. Put this role in `client/remotes/SettingsClient.js`:

```javascript
@HTTPClient
class SettingsClient {
  @GetRequest("/manifest.webmanifest")
  async load() {}
}
```

- `NeedsAll` injects a `settingsClient` field into the browser
  service (`client/services/SettingsService.js`).
- `await this.settingsClient.load()` returns the published manifest.
- The manifest binds as it is, for
  `Behaviors.properties(PublicSettings, application)` or
  `Behaviors.values(PublicValues, application)`.
- Native services keep a platform loader that they own explicitly, or a public
  snapshot that is supplied to them.
- Tachyon adds no implicit synchronous filesystem or network access. It adds no
  global Tac dependency container.

Path and name rules:

- Tac Setting paths must start with tac.
- Yon paths must start with yon.
- Settings keeps its prefixed name spelling. It resolves each
  underscore into the canonical nested path.
- Settings takes only the prefix (and optional `names` renames). The class's
  instance fields are the settings: the declared type and initial value of a
  field give its type and default, and `@Min`/`@Max` on the field give its
  range. A field without an initial value is required. The environment name is
  the field name in capitals with underscores, so `pageSize` reads
  `YON_CATALOG_PAGE_SIZE`. Rust and Go fields have no initial values, so their
  defaults go in `"defaults":{...}`.
- Flat in-memory property maps that the caller supplies explicitly still work
  with adapters. They are not a way to write server/application.json.

### The Yon snapshot at runtime

- Server startup reads the configuration and `.env` once, into a private
  snapshot that cannot change.
- Handlers, middleware and workers receive the snapshot as the reserved
  supervisor output `YON_APPLICATION_JSON`.
- A value from the parent process cannot replace that output.
- Changes to Yon settings, profiles or `.env` need a restart.
- A change to `client/manifest.json` reloads Tac. It does not replace the live
  private snapshot.

## Read and bind in every Yon language

These nine alternatives use the same `server/application.json`. Each source
file owns one class and its imports. The empty `yon.<ext>` file marks the
route; its controller owns `GET`. The compiler composes the same-language
behavior-annotated files in `server/helpers/` without making them injectable.

Each endpoint returns only selected non-secret values, never the private
snapshot. `Setting` returns validated data; assigning that data to an instance
is an explicit step.

#### `server/application.json`

```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 8080
  },
  "catalog": {
    "page": {
      "size": 20
    }
  }
}
```

### JavaScript

#### `server/helpers/CatalogSettings.js`

```js
@Settings({"prefix":"YON_CATALOG_"})
class CatalogSettings {
  @Min(1)
  @Max(100)
  pageSize = 20;
}
```

#### `server/helpers/Endpoint.js`

```js
class Endpoint {
  @Setting("${yon.server.port}")
  port;
  @Setting("${yon.redis.url:redis://127.0.0.1:6379}")
  redis;
}
```

#### `server/controllers/ConfigurationController.js`

```js
@RequestRoute("/configuration")
@Public
class ConfigurationController {
  @GetRoute
  getConfiguration() {
    const application = JSON.parse(process.env.YON_APPLICATION_JSON ?? '{}');
    const settings = Behaviors.properties(CatalogSettings, application);
    const endpoint = Behaviors.values(Endpoint, application);
    return { pageSize: settings.pageSize, port: endpoint.port };
  }
}
```

### TypeScript

#### `server/helpers/CatalogSettings.ts`

```ts
@Settings({"prefix":"YON_CATALOG_"})
class CatalogSettings {
  @Min(1)
  @Max(100)
  pageSize: integer = 20;
}
```

#### `server/helpers/Endpoint.ts`

```ts
class Endpoint {
  @Setting("${yon.server.port}")
  port!: number;
  @Setting("${yon.redis.url:redis://127.0.0.1:6379}")
  redis!: string;
}
```

#### `server/controllers/ConfigurationController.ts`

```ts
@RequestRoute("/configuration")
@Public
class ConfigurationController {
  @GetRoute
  getConfiguration() {
    const application = JSON.parse(process.env.YON_APPLICATION_JSON ?? '{}');
    const settings = Behaviors.properties(CatalogSettings, application);
    const endpoint = Behaviors.values(Endpoint, application);
    return { pageSize: settings.pageSize, port: endpoint.port };
  }
}
```

### Python

#### `server/helpers/CatalogSettings.py`

```py
from typing import Annotated
import json
import os

@settings({"prefix":"YON_CATALOG_"})
class CatalogSettings:
    pageSize: Annotated[int, min(1), max(100)] = 20
```

#### `server/helpers/Endpoint.py`

```py
from typing import Annotated
import json
import os

class Endpoint:
    port: Annotated[int, setting("${yon.server.port}")] = 0
    redis: Annotated[str, setting("${yon.redis.url:redis://127.0.0.1:6379}")] = ""
```

#### `server/controllers/ConfigurationController.py`

```py
import json
import os

@request_route("/configuration")
@public
class ConfigurationController:
    @get_route
    def get_configuration(self, request):
        application = json.loads(os.environ.get("YON_APPLICATION_JSON", "{}"))
        settings = Behaviors.properties(CatalogSettings, application)
        endpoint = Behaviors.values(Endpoint, application)
        return {"pageSize": settings["pageSize"], "port": endpoint["port"]}
```

### PHP

#### `server/helpers/CatalogSettings.php`

```php
<?php

#[Settings({"prefix":"YON_CATALOG_"})]
class CatalogSettings {
    #[Min(1)]
    #[Max(100)]
    public int $pageSize = 20;
}
```

#### `server/helpers/Endpoint.php`

```php
<?php

class Endpoint {
    #[Setting('${yon.server.port}')]
    public int $port = 0;
    #[Setting('${yon.redis.url:redis://127.0.0.1:6379}')]
    public string $redis = '';
}
```

#### `server/controllers/ConfigurationController.php`

```php
<?php

#[RequestRoute("/configuration")]
#[Public]
class ConfigurationController {
    #[GetRoute]
    public function getConfiguration($request) {
        $applicationJson = getenv('YON_APPLICATION_JSON') ?: '{}';
        $application = json_decode($applicationJson, true, 512, JSON_THROW_ON_ERROR);
        $settings = Behaviors::properties(CatalogSettings::class, $application);
        $endpoint = Behaviors::values(Endpoint::class, $application);
        return ['pageSize' => $settings['pageSize'], 'port' => $endpoint['port']];
    }
}
```

### Java

#### `server/helpers/CatalogSettings.java`

```java
@Settings({"prefix":"YON_CATALOG_"})
class CatalogSettings {
    @Min(1)
    @Max(100)
    int pageSize = 20;
}
```

#### `server/helpers/Endpoint.java`

```java
class Endpoint {
    @Setting("${yon.server.port}")
    long port;
    @Setting("${yon.redis.url:redis://127.0.0.1:6379}")
    String redis;
}
```

#### `server/controllers/ConfigurationController.java`

```java
@RequestRoute("/configuration")
@Public
class ConfigurationController {
    @GetRoute
    Object getConfiguration() {
        var applicationJson = System.getenv("YON_APPLICATION_JSON");
        var application = (java.util.Map<?, ?>) Json.parse(applicationJson);
        var settings = Behaviors.properties("CatalogSettings", application);
        var endpoint = Behaviors.values("Endpoint", application);
        return java.util.Map.of("pageSize", settings.get("pageSize"), "port", endpoint.get("port"));
    }
}
```

### Kotlin

#### `server/helpers/CatalogSettings.kt`

```kt
@Settings({"prefix":"YON_CATALOG_"})
class CatalogSettings {
    @Min(1)
    @Max(100)
    var pageSize: Int = 20
}
```

#### `server/helpers/Endpoint.kt`

```kt
class Endpoint {
    @Setting("${yon.server.port}")
    var port: Long = 0
    @Setting("${yon.redis.url:redis://127.0.0.1:6379}")
    var redis: String = ""
}
```

#### `server/controllers/ConfigurationController.kt`

```kt
@RequestRoute("/configuration")
@Public
class ConfigurationController {
    @GetRoute
    fun getConfiguration(): Any {
        val applicationJson = System.getenv("YON_APPLICATION_JSON")
        val parsed = Json.parse(applicationJson) as Map<*, *>
        val application = parsed.mapKeys { (key, _) -> key as String }
        val settings = Behaviors.properties("CatalogSettings", application)
        val endpoint = Behaviors.values("Endpoint", application)
        return mapOf("pageSize" to settings["pageSize"], "port" to endpoint["port"])
    }
}
```

### C#

#### `server/helpers/CatalogSettings.cs`

```cs
[Settings({"prefix":"YON_CATALOG_"})]
public sealed class CatalogSettings {
    [Min(1)]
    [Max(100)]
    public int pageSize = 20;
}
```

#### `server/helpers/Endpoint.cs`

```cs
public sealed class Endpoint {
    [Setting("${yon.server.port}")]
    public long port;
    [Setting("${yon.redis.url:redis://127.0.0.1:6379}")]
    public string redis = "";
}
```

#### `server/controllers/ConfigurationController.cs`

```cs
[RequestRoute("/configuration")]
[Public]
class ConfigurationController {
    [GetRoute]
    public object GetConfiguration() {
        var applicationJson = System.Environment.GetEnvironmentVariable("YON_APPLICATION_JSON") ?? "{}";
        var application = BehaviorJSON.Parse(applicationJson);
        var settings = Behaviors.properties(typeof(CatalogSettings), application);
        var endpoint = Behaviors.values(typeof(Endpoint), application);
        var pageSize = settings.Get("pageSize").Integer;
        var port = endpoint.Get("port").Integer;
        return new { pageSize, port };
    }
}
```

### Rust

#### `server/helpers/CatalogSettings.rs`

```rs
#[Settings({"prefix":"YON_CATALOG_","defaults":{"pageSize":20}})]
struct CatalogSettings {
    #[Min(1)]
    #[Max(100)]
    pageSize: i64,
}
```

#### `server/helpers/Endpoint.rs`

```rs
#[allow(dead_code)]
struct Endpoint {
    #[Setting("${yon.server.port}")]
    port: u16,
}
```

#### `server/controllers/ConfigurationController.rs`

```rs
#[RequestRoute("/configuration")]
#[Public]
struct ConfigurationController {}
impl ConfigurationController {
    fn new() -> Self { Self {} }
    #[allow(non_snake_case)]
    #[GetRoute]
    fn get_configuration(&self) -> Json {
        let application_json = std::env::var("YON_APPLICATION_JSON").expect("supervisor configuration");
        let application: BehaviorValue = Json::parse(&application_json).expect("validated configuration").into();
        let settings = Behaviors::properties::<CatalogSettings>(&application).expect("catalog settings");
        let endpoint = Behaviors::values::<Endpoint>(&application).expect("endpoint settings");
        let page_size = settings.get("pageSize").expect("page size").clone();
        let port = endpoint.get("port").expect("port").clone();
        let result = BehaviorValue::object([("pageSize", page_size), ("port", port)]);
        let result_json = result.to_json(1024).expect("bounded output");
        Json::parse(&result_json).expect("valid JSON")
    }
}
```

### Dart

#### `server/helpers/CatalogSettings.dart`

```dart
import 'dart:convert';
import 'dart:io';

@Settings({"prefix":"YON_CATALOG_"})
class CatalogSettings {
  @Min(1)
  @Max(100)
  int pageSize = 20;
}
```

#### `server/helpers/Endpoint.dart`

```dart
import 'dart:convert';
import 'dart:io';

class Endpoint {
  @Setting("${yon.server.port}")
  int port = 0;
  @Setting("${yon.redis.url:redis://127.0.0.1:6379}")
  String redis = '';
}
```

#### `server/controllers/ConfigurationController.dart`

```dart
import 'dart:convert';
import 'dart:io';

@RequestRoute("/configuration")
@Public
class ConfigurationController {
  @GetRoute
  Object? getConfiguration() {
    final applicationJson = Platform.environment['YON_APPLICATION_JSON'] ?? '{}';
    final parsed = jsonDecode(applicationJson) as Map;
    final application = parsed.cast<String, Object?>();
    final settings = Behaviors.properties(CatalogSettings, application) as Map;
    final endpoint = Behaviors.values(Endpoint, application) as Map;
    return {'pageSize': settings['pageSize'], 'port': endpoint['port']};
  }
}
```

### Go

#### `server/helpers/CatalogSettings.go`

```go
// @Settings({"prefix":"YON_CATALOG_","defaults":{"pageSize":20}})
type CatalogSettings struct {
	// @Min(1)
	// @Max(100)
	pageSize int64
}
```

#### `server/helpers/Endpoint.go`

```go
type Endpoint struct {
	// @Setting("${yon.server.port}")
	port uint16
}
```

#### `server/controllers/ConfigurationController.go`

```go
import (
	"encoding/json"
	"os"
)

// @RequestRoute("/configuration")
// @Public
type ConfigurationController struct{}

// @GetRoute
func (controller *ConfigurationController) GetConfiguration() (any, error) {
	var application map[string]any
	err := json.Unmarshal([]byte(os.Getenv("YON_APPLICATION_JSON")), &application)
	if err != nil {
		return nil, err
	}
	settings, err := BehaviorsProperties[CatalogSettings](application)
	if err != nil {
		return nil, err
	}
	endpoint, err := BehaviorsValues[Endpoint](application)
	if err != nil {
		return nil, err
	}
	pageSize := settings.(map[string]any)["pageSize"]
	port := endpoint.(map[string]any)["port"]
	return map[string]any{"pageSize": pageSize, "port": port}, nil
}
```

## A complete worked example

This example composes one Yon unit from four one-class TypeScript files. It
combines `Settings`, `Setting`, `Setup`, `WhenProfile` and
`WhenSetting`. The profile name is itself a property, so the selected
file determines which Setup is active. The handler reads no environment
variable except the reserved snapshot. Select the production file with
`YON_PROFILE=production`.

### `server/application.json`

```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 8080
  },
  "redis": {
    "url": "redis://127.0.0.1:6379"
  },
  "active": {
    "profile": "development"
  },
  "catalog": {
    "page": {
      "size": 20
    },
    "cache": {
      "enabled": false
    }
  }
}
```

### `server/application-production.json`

```json
{
  "server": {
    "port": 9000
  },
  "active": {
    "profile": "production"
  },
  "catalog": {
    "cache": {
      "enabled": true
    }
  }
}
```

### `server/helpers/CatalogProperties.ts`

```typescript
@Settings({"prefix":"YON_CATALOG_"})
class CatalogProperties {
  @Min(1)
  @Max(100)
  pageSize: integer = 20;
  cacheEnabled: boolean = false;
}
```

### `server/helpers/Endpoint.ts`

```typescript
class Endpoint {
  @Setting("${yon.server.port}")
  port!: number;
  @Setting("${yon.redis.url}")
  redis!: string;
  @Setting("${yon.active.profile:development}")
  profile!: string;
}
```

### `server/controllers/ProductsController.ts`

```typescript
@RequestRoute("/products")
@Public
class ProductsController {
  @GetRoute
  async getProducts() {
    const application = JSON.parse(process.env.YON_APPLICATION_JSON ?? '{}');
    const catalog = Behaviors.properties(CatalogProperties, application);
    const endpoint = Object.assign(new Endpoint(), Behaviors.values(Endpoint, application));
    const lifetime = new AbortController();
    const context = await Behaviors.create(ProductionSetup, {
      partition: 'public-catalog', signal: lifetime.signal,
      profiles: [endpoint.profile], properties: application,
    });
    try { return { pageSize: catalog.pageSize, port: endpoint.port }; }
    finally { context.dispose(); lifetime.abort(); }
  }
}
```

### `server/setups/ProductionSetup.ts`

```typescript
@Setup
@WhenProfile('production')
@WhenSetting({"name":"YON_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
class ProductionSetup {
  static configure() { return {}; }
}
```

The compiler discovers these same-language files within their existing role
folders (ADR 0031). `CatalogProperties` and `Endpoint` carry behavior
annotations only; neither becomes an injectable provider. Unknown
`YON_CATALOG_*` keys fail the `Settings` binding.

What happens:

- With the base file, `Behaviors.create` fails with `inactive-configuration`
  before `configure` runs. The reason: the profile is `development` and the
  cache is off.
- Under `YON_PROFILE=production`, both conditions pass. `endpoint.port` is 9000.
  `catalog.pageSize` is still 20, from the base file.
- The handler reads nothing from the process environment except the reserved
  snapshot. Nothing in this example returns the snapshot itself.

## Deployment authority stays outside the file

Application files cannot select any of these:

- `YON_ISOLATION`
- a Firecracker driver
- a runtime executable
- whether handlers inherit the environment
- native capabilities
- host security policy

These stay explicit deployment decisions. Application configuration gives no
authority.

In Firecracker mode:

- The supervised control program receives the private configuration
  environment value.
- The operator owns the integration. That integration must explicitly copy the
  value into the guest.
- Tachyon does not automatically forward the host environment into a microVM
  (a small virtual machine).
- The existing limits on languages and artifact transfer still apply.

See [ADR 0014](adr/0014-environment-selected-yon-isolation.md) and
[ADR 0023](adr/0023-private-application-configuration.md).

## Migration from v2

1. Move `application.json` and every `application-<profile>.json` from the
   project root into `server/`.
2. Move a root `manifest.json` into `client/`.
   If a file stays at the root, the build stops and says where the file
   belongs.
3. Move the members of the `yon` section to the top level of
   `server/application.json`, and remove the `yon` key. Do this in each profile
   too.
4. Move the `tac` section of `application.json` into `client/manifest.json` as
   its `tac` member. If the project has no manifest, create one. `name` and
   `start_url` are enough to start.
5. `.env` stays in the project root.
6. Change any client that loaded `/.tachyon/application.json` to load
   `/manifest.webmanifest`.

`ty migrate` reports each of these moves.

These do not change: paths such as `yon.server.port`, `TAC_`/`YON_` names,
`Setting` expressions and the `YON_APPLICATION_JSON` snapshot.

See [ADR 0034](adr/0034-tac-configuration-is-the-web-app-manifest.md) and the
[v3 schema](../api/application-config/v3/schema.json).

## Migration from v1

1. Move server/datasource/redis into the yon namespace.
2. Convert properties.TAC_*/YON_* to nested paths.
3. Move secrets into .env.
4. Remove JSON placeholders.
5. Update Setting paths.
6. Pass the canonical snapshot to explicit property binding.

See [ADR 0026](adr/0026-json-results-and-separated-configuration.md) and the
[v2 schema](../api/application-config/v2/schema.json). The v1 schema stays as a
historical record. Tachyon does not silently accept the old way of writing
configuration.
