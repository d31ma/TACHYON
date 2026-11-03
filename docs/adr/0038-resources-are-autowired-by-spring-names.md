# ADR 0038: Resources Are Autowired by Spring's Names, and Markers Are Written Bare

- Status: Accepted
- Date: 2026-09-23
- Amends: [ADR 0028](0028-owned-application-events-and-lifecycle.md) and
  [ADR 0031](0031-realms-mirror-each-other.md) (how resources are declared and
  named)
- Preserves: owner-scoped resources, the implicit `applicationEventPublisher`
  field, the empty-argument marker spelling
- Amended by: [ADR 0047](0047-annotation-names-are-plain-english.md): annotations and role folders have plain English names (for example `Needs`, `NeedsAll`, `Store`, `OnStart`, `stores/`); the old names are refused and `ty migrate fix --write` renames them

## Context

A service reached the objects that the framework owns through
`@Resource({"name":"…"})`. This borrowed Jakarta's name-based injection. The
annotations otherwise follow Spring, and two things did not match Spring:

- Spring injects `ApplicationEventPublisher`, a `DataSource` or a
  `RedisTemplate` by autowiring. `@Resource` is the Jakarta alternative, and a
  Spring application rarely writes it. Yon already autowired its resources by
  field name.
- Several names were Tachyon's own (`element`, `signal`, `renderScheduler`,
  `redis`). Spring has a name for the nearest thing to each.

Markers that take no arguments were also written with an empty argument list,
as in `@Service()`. Spring never does this. In JavaScript and TypeScript the
bare form had a different meaning. Bare `@Service`, `@Component` and
`@Repository` were Yon's stereotypes. Only the form with brackets was the Tac
role.

## Decision

- A resource is injected by `@Autowired` on a field named after the resource.
  This is how Spring autowires by name. `@Resource` is retired. The build
  refuses it and names the field to write instead.
- Each resource carries the name that Spring gives the nearest thing it has:

  | Realm | Resource | Meaning | Former name |
  | --- | --- | --- | --- |
  | Browser | `view` | the component's element | `element` |
  | Browser | `lifecycle` | an `AbortSignal` that is aborted on disposal | `signal` |
  | Browser | `cacheManager` | | |
  | Browser | `resourceLoader` | | |
  | Browser | `taskScheduler` | its one task is a render | `renderScheduler` |
  | Browser | `applicationEventPublisher` | | |
  | Server | `dataSource` | | |
  | Server | `redisTemplate` | now the `RedisTemplate` type | `redis` |
  | Server | `applicationEventPublisher` | | |

  The build refuses a field with a former name and gives its new name. Yon's
  JavaScript `signal` resource was accepted but never supplied. It is gone.
- An uninitialised `applicationEventPublisher` field is still injected without
  the marker.
- A marker that takes no arguments is written bare in every language:
  `@Service`, `#[Service]` in Rust, `[Service]` in C#. The form with an empty
  argument list is the same marker, and the build still accepts it.
- A bare role marker means the role of the realm that the file sits in. Under
  `client/` it is the Tac role. Under `server/` it is Yon's role.
- The TypeScript declarations type each zero-argument marker as usable bare or
  called. They name the resources in `TacResourceMap`, with
  `TacTaskScheduler` in place of `TacRenderScheduler`.

## Consequences

- An application's source reads like Spring's: `@Service`, `@Autowired`, and
  resources named as Spring names them. The documentation, the website and the
  scaffold are written that way.
- A project that uses `@Resource` or a former resource name stops building
  until the field is renamed and autowired. The diagnostic names both the old
  and the new form.
- `view`, `lifecycle` and `taskScheduler` are the nearest Spring names, not
  Spring's interfaces. The browser resources stay Tachyon contracts.
