# Shared behavior extensions

This document is the implementation contract for the extension to ADR 0022
(ADR means Architecture Decision Record). It describes the target that the work
must reach. It does not say what has shipped. The supported API guide and the
executable gates (the automated checks) decide what has shipped.

Two rules stay the same:

- Tac stays client-only.
- Yon stays REST-only.

## Cache mutations

`UpdateCache` and `ClearCache` accept exactly one argument form:
`{"method":"read","arguments":[0]}`.

- The target (`read` in this example) must be a Cached method in the same
  class.
- The mapping gives the positional arguments for the target, including the
  target's PageDefaults input.
- The operation reuses the target's normalized key, module identity, partition
  and CacheSettings policy.

Order of work and failure rules:

1. Authorization, adapter availability and key validation run before the
   application changes anything.
2. If the method fails, returns an invalid result, or is cancelled, the cache
   does not change.
3. UpdateCache never reads the cache. After success, it stores the result with the
   target's TTL (time to live) and bounds.
4. ClearCache deletes the entry only after success.
5. If the cache adapter fails, the error goes to the caller. These operations
   do not promise one atomic transaction across the database and the cache.

Arguments and validation:

- PageDefaults belongs to the target method, not to the mutation method.
- The framework does not run the target's DTO (data transfer object) validation
  again only to calculate a cache key.
- Validation of the mutation method's own arguments still runs before any work.

## Stores and queries

Bare `Store` keeps its existing meaning in Yon DI (dependency injection).

The form that names an Entity class is different:
`Store(Product, {"fields":["id","status"],"defaultSize":20,"maxSize":100})`.
Java writes `Product.class`, Kotlin and PHP `Product::class`, C# `typeof(Product)`.

- It is a shared query declaration that is local to its source file.
- It does not create a DI container.
- If you use Yon DI, add an explicit Service/provider declaration.

Rules for the entity and the query methods:

- The bound entity must declare Entity and Table in the same source module.
- Query methods have empty bodies (in Python: `pass`).
- Query methods use the existing language-specific bounded JSON return types.
- The compiler generates the code that runs the query.
- Put application logic in service methods. The compiler never silently
  replaces a method body with a query.

### The `Query` annotation

`Query` accepts exactly these keys: `operation`, `where` and `orderBy`. A
`find` takes its page request from the parameter marked `@PageDefaults`.

- Operations: `find`, `count`, `exists`.
- Predicates combine with AND.
- Operators: `eq`, `ne`, `lt`, `lte`, `gt`, `gte`, `in`, `isNull`, `isNotNull`.
- A value predicate has a field, an operator and an argument.
- A null predicate has only a field and an operator.
- Bindings stay data. The framework never puts them into SQL text.
- The application selects an entity adapter. The adapter translates the command
  into parameterized operations.

### Derived query names

Derived method names use the pattern
`(find|count|exists)By<Field><Operator>[And...]`. The equivalent snake_case
spellings also work.

Operators: Equals (or no operator), NotEquals, LessThan, LessThanEqual,
GreaterThan, GreaterThanEqual, In, IsNull, IsNotNull.

- If the compiler cannot split a name into known fields, or the split is
  ambiguous, compilation fails.
- The method must use all of its parameters.

This bounded query contract does not include nested paths, joins, OR, raw SQL,
expression languages or derived bulk mutations.

### Limits

| Limit | Value |
| --- | --- |
| Maximum page size for find queries | 100 |
| Predicates | at most eight |
| Authored sort fields | at most four |
| Declared fields | at most 64 |
| Scalar values per In binding | at most 100 |
| Total bindings | at most 64 KiB |
| Result size | at most 1 MiB |

Other rules:

- Find queries always end their ordering with the ID, so the order is
  deterministic when other values are equal.
- The annotation owns the ordering. The query's page.sort must be empty.
- Results: find returns at most page-size JSON objects. Count returns a
  nonnegative safe integer. Exists returns a boolean.
- `authorizeEntity(command, execution)` is required. It must return the literal
  value true before the query runs.
- Existing explicit repository CRUD keeps its authorization in the adapter.

## Configuration

`Settings` accepts `prefix` and `fields`.

- The prefix must start with `TAC_` in Tac or `YON_` in Yon, and end with `_`.
- Each field has a safe application name and a descriptor.

The descriptor has these keys:

| Key | Required | Meaning |
| --- | --- | --- |
| `name` | yes | Uppercase environment suffix |
| `type` | yes | string, boolean, integer or number |
| `default` | no | Value used when none is supplied |
| `min`/`max` | no | Numeric bounds |

Value rules:

- A missing value without a default is an error.
- A default must already have the declared JSON type.
- Every numeric value and bound has an absolute magnitude of at most
  9007199254740991. Integer values must also be whole numbers.
- Booleans accept only the literal `true` or `false`.
- Unknown keys under the selected prefix fail. This catches typos in
  configuration.

`Behaviors.properties(Type, values)` binds a string map into a bounded JSON
object. Each language has an equivalent typed or name-based form.

- The caller must supply the string map explicitly.
- It never reads the process environment.
- It never runs a Setup class.
- It never creates a DTO through reflection.
- Limits: 256 entries, 4 KiB per value and 64 KiB in total.
- Error messages never include the supplied values.

Conditions on a Setup class:

- `WhenProfile("development", "test")` is an allowlist. The class is active if any
  listed profile is active (OR).
- `WhenSetting({"name":"YON_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})`
  is an exact string match on an explicitly supplied value. It also goes on a
  Setup class.
- If you combine both, both conditions must pass.

Context creation:

- Context creation options gain the optional inputs `profiles` and `properties`.
  The native equivalents keep the existing default constructors.
- An inactive Setup fails with `inactive-configuration` before
  configure is called.
- This is explicit selection. It is not package scanning and not an automatic
  application container.
- The framework takes a snapshot of the creation inputs. They are not live views
  of environment data that can change.
- Only the prefix of the relevant framework is accepted.

## Validation

Checked marks a DTO schema that is local to its source file.

Field annotations constrain named direct fields:

- Required
- NotEmpty
- Length({"min":0,"max":100})
- Min(0)
- Max(100)

Rules:

- Length measures UTF-8 bytes for strings. For arrays and objects it counts the
  elements or keys.
- Length, Min and Max ignore missing or null values, unless Required or NotEmpty is
  also present.
- NotEmpty requires a string that is not empty and not only whitespace.
- Min and Max require finite JSON numbers. They never convert strings to numbers.
- A field constraint outside a Checked type fails.

`@Check` goes on one parameter of an instance method. It validates that JSON
object before the application code, caching or HTTP runs. In JavaScript,
TypeScript and Python the parameter's type names the Checked class; the other
languages keep the JSON parameter type and write the class in the marker, such
as `@Check(ProductInput.class) Object input` in Java.

`Behaviors.validate(Type, value)` gives the same explicit validation for a DTO
or a form.

- Constraints do not change the source objects.
- Constraints do not run constructors or getters.
- Unknown JSON fields are still allowed. This is validation. It does not decide
  which fields a caller may assign (mass assignment).
- Validation errors use the code `validation` and a bounded field/rule path.
  They do not repeat the input values.
- This slice does not include nested Check, schema recursion or validation
  groups.
- The JSON depth, node and size bounds still apply.

## Developer layout

- Layout conventions are optional and organized by feature.
- Filesystem route entrypoints are the only required layout.
- Annotated declarations are currently local to their source file. If you move
  them into a models, stores or setups subdirectory, the compiler
  does not find them.
- Where the language adapter supports imports, you may organize ordinary
  imported helper code by feature and adapter.
- Client and server adapters and configuration stay separate.
- The framework does not generate empty directories only to show an
  architecture.

## Acceptance

Every feature needs all of these:

- shared parser rejection tests
- real runtime execution for every relevant Tac and Yon language
- regression tests for authorization, cancellation and boundaries
- accurate types and examples
- website and API documentation
- the canonical Rust gate
- the required polyglot CI gate

If the compiler only recognizes a marker, that does not count as support.

AllOrNothing stays deferred until a real adapter that supports transactions
exists.
