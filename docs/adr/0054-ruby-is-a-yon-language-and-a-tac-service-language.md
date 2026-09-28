# ADR 0054: Ruby Is a Yon Language and a Tac Service Language

- Status: Accepted
- Date: 2026-09-26
- Builds on: [ADR 0049](0049-tac-ruby-uses-a-pinned-mruby-runtime.md) (the
  browser runtime), [ADR 0048](0048-yon-routes-come-from-annotations.md)
  (routes), [ADR 0051](0051-controller-error-handlers.md) (error handlers),
  [ADR 0044](0044-an-annotated-file-holds-one-class.md) (one class per file)
  and [ADR 0043](0043-models-hold-data-types.md) (models)
- Amends: [ADR 0031](0031-realms-mirror-each-other.md) (the Yon languages)
  and [ADR 0045](0045-go-is-a-service-language.md) (which service languages
  reach the web as WebAssembly)

## Context

Go showed that a language without decorators can still be a Tachyon
language: the annotations are comments. Ruby has no decorators and no type
syntax. Ruby developers already write types in YARD comments, and RuboCop
and YARD leave comment lines alone.

## Decision

### Annotations

- A Ruby annotation is `#[Name]` or `#[Name(arguments)]`, alone on its line
  directly above the class, the `def`, or a parameter. Ruby reads the line as
  a comment, so an annotated file is ordinary Ruby for `ruby -c`, RuboCop and
  editors. The names and arguments are the same as in every other language.
- A parameter annotation sits on its own line inside a multiline parameter
  list, directly above the parameter.
- Tachyon reads a `#[Name]` line as an annotation only when Name is a
  Tachyon annotation, an old name that Tachyon explains, or a close
  misspelling (refused with "did you mean"). Other `#[...]` comments stay
  comments. `@Name` on its own line and `# [Name]` (the space RuboCop's
  `Layout/LeadingCommentSpace` adds) are refused with the right spelling.
  Projects turn that RuboCop rule off for annotation lines.
- Heredocs, `=begin` blocks, strings and `__END__` data never hold
  annotations.

### Types

- Route parameter and return types, and the error type of a HandlesError
  method, come from YARD tags on the method: `# @param id [Integer]`,
  `# @return [Array<Order>]`, `# @param error [OrderNotFound]`. `String`,
  `Integer`, `Float`, `Boolean`, `Array<T>` and `T, nil` map to the shared
  route types; other constants are models. A parameter without a tag
  accepts any JSON value.
- A model in `client/models/` or `server/models/` is a `Data.define` or
  `Struct.new` record whose fields are typed with YARD `@!attribute`
  directives, or a class with `attr_reader` fields, each typed with a
  `# @return [T]` line, and an `initialize` method. Nothing else may live in
  a models file.

### Yon

- Ruby (`.rb`) is a Yon language: controllers, services and other roles,
  error handlers, workers, root middleware (`middleware.rb` with
  `self.before(request)` and `self.after(request)`) and test units.
- Tachyon runs Ruby with CRuby 3.3 or newer (`ruby`, or
  `YON_RUBY_RUNTIME`), because it needs `Data.define` and strict JSON
  generation. `ty doctor` and startup check the version.
- A plain handler is `class Handler` with class methods named after HTTP
  methods. A controller route is an instance method whose name starts with
  its HTTP method in snake case (`get_orders`). A streaming route uses
  `yield`; a plain handler streams by returning an `Enumerator`.
- Providers take their dependencies with `#[Needs(OrderStore)]` and
  `def initialize(order_store)`. `NeedsAll` is not available in Ruby.
- The runner loads every `server/models/**/*.rb` file, then the program.
  Tachyon adds the composed role files after the author's file, so the
  author's line numbers stay the same in error messages.
- `relay`, `DataSource`, `RedisTemplate` and Gateway calls into a Ruby
  class follow the same rules as Python. A Ruby class can be a Gateway
  target and a Gateway caller. The rules are: scrubbed environment, remaining
  deadline, fixed failure answer, direct loopback resource calls.
  Firecracker isolation does not run Ruby, the same as PHP: the driver
  transfers only JavaScript and Python source.
- Behavior annotations run in Ruby; see "Behavior annotations" below.

### Tac

- Ruby is a Tac service language on the web. `client/services/<Name>Service.rb`
  runs on the mruby runtime of ADR 0049. Native hosts render the same web
  bundle, so a Ruby service reaches every target through it, as Go does.
- A service's page members are its public `attr_reader` and
  `attr_accessor` fields (writable when `attr_accessor`) and its public
  instance methods. `#[Service]`, `#[Lifetime]` and `#[NeedsAll]` work as
  in the other languages. `#[OnStart]`, `#[OnStop]`, `#[OnEvent]`,
  `#[Order]` and `#[InBackground]` work through the page's event bus. A
  service publishes with `ApplicationEventPublisher.new.publish_event`.
- Client test units run in CRuby on the build machine. The browser needs
  nothing installed. The developer's machine builds the runtime once into the
  per-user Tachyon cache with its own Ruby and Emscripten (ADR 0049,
  amendment 2026-09-27); a project without Tac Ruby services never needs
  Emscripten.
- The runtime adds about 1.1 MB (about 350 KiB with Brotli) to a page that
  uses a Ruby service, once for all Ruby services. It stays under the
  4 MiB service-worker precache limit.

## Consequences

- Ruby joins the multi-language examples that use only these features.
- Ruby and mruby differ in places (ADR 0049). The same Ruby code must not
  rely on the full CRuby standard library in a Tac service.
- Behavior annotations, typed events and Yon events arrived in the Ruby
  annotation phase (below).
- The website writes two of its services in Ruby, so its pages download the
  mruby runtime once: about 1.1 MB, or about 350 KiB with Brotli. The
  service worker caches it with the other assets.

## Behavior annotations

Added in the Ruby annotation phase. Ruby follows the Spring and Lombok
meaning of each annotation, like the other languages, and every marker is
checked by the shared rule table (ADR 0056); Ruby has no checks of its own.

- One runtime file, `behavior/polyglot/ruby_runtime.rb`, runs the behaviors in
  Yon (CRuby) and in Tac services (mruby). It follows the PHP runtime and
  passes the same boundary contract in both Rubies
  (`tests/fixtures/behavior/ruby_runtime_contract.rb`).
- The compiler keeps the author's lines. A behavior method becomes
  `def find(sku); @behaviors.invoke(self, "CatalogService", "find", [sku],
  lambda do |sku| ... end); end`. A lambda is used so that `return` inside
  the method body returns from the body, not from the wrapper.
- The class keeps its context in `@behaviors`, set by `Behaviors.create(Setup,
  partition: ..., signal: BehaviorSignal.new)` (often in `#[OnStart]`) or by
  `Behaviors.construct(Type, behaviors: context)`. Adapters are hashes of
  callables or objects with `get`/`set`/`delete`/`exchange`/`execute`
  methods.
- Types come from YARD: `Settings` fields are `attr_reader` lines with a
  `@return` type. Ruby has no field initializers, so `Settings` takes
  `"defaults"`, as in Rust and Go. A checked parameter keeps its JSON value
  and `#[Check(ProductInput)]` names the class.
- Ruby serves one call at a time, so `MaxAtOnce` takes only `"reject"`, as
  in PHP. In the browser a `Retry` does not wait between attempts, because
  `sleep` would freeze the page.
- Events are typed (ADR 0055): a listener names its model with
  `# @param order [PlacedOrder]`, a model `@return` publishes the returned
  value, and `publish_event(model)` publishes a `Data.define` or `Struct`
  value. Yon Ruby services get the local event bus, `OnStart` and `OnStop`.
- Test units get `Fake` fields, cleared before each case, and
  `construct(Role)`, which builds a role with the fakes and the unit's
  context. Ruby can set fields from outside the class, so it gets the
  construction that JavaScript has.
- `ty migrate fix --write` reads Ruby files for the event conversion only.
