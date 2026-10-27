# ADR 0014: Environment-Selected Yon Isolation Backends

- Status: Accepted
- Date: 2026-08-09
- Amended by: [ADR 0111](0111-yon-keeps-handler-processes-warm-by-default.md): `YON_ISOLATION` also accepts `warm`, which is now the default, and `YON_WARM_MAX_REQUESTS` and `YON_WARM_IDLE_SECONDS` tune it

## Context

Yon supervises one bounded child process for each handler invocation. This
process boundary contains crashes, protocol violations, output, concurrency,
deadlines, and environment inheritance. It does not remove the child
process's normal operating-system access to the filesystem and the network.

Some deployments run many tenants on one host and need hardware isolation.
They need a stronger backend, such as a Firecracker microVM (a small, fast
virtual machine). But isolation is a security decision for the operator who
deploys the application. An application repository must not be able to weaken
the deployment boundary by editing `tachyon.json` or another project file.

Firecracker is a low-level virtual-machine monitor for Linux and KVM
(Kernel-based Virtual Machine, the virtualization support built into Linux).
It is not an application runtime. Yon should keep its route and handler
behavior the same. A separately owned control program manages the rest:

- kernels and root filesystems;
- pools and snapshots;
- networking;
- the jailer;
- host qualification.

## Decision

Yon selects handler isolation only from the environment of the parent
process:

| Variable | Meaning | Default or bound |
| --- | --- | --- |
| `YON_ISOLATION` | `process` or `firecracker` | `process` |
| `YON_FIRECRACKER_DRIVER` | Absolute control-program path | required for `firecracker`; regular, non-symlinked, executable, and not group/world-writable |
| `YON_FIRECRACKER_POOL` | Operator-owned pool identifier | `default`; 1–64 portable identifier characters |
| `YON_FIRECRACKER_VCPUS` | Requested virtual CPUs | `1`; 1–32 |
| `YON_FIRECRACKER_MEMORY_MIB` | Requested guest memory | `256`; 128–32,768 MiB |
| `YON_FIRECRACKER_EGRESS` | Guest network policy | `deny`; no other value is currently accepted |

No project file can set these values, and no handler request can override
them. If the configuration is malformed, partial, not Unicode, or out of
range, Yon fails with `TY2010` before any handler runs.

### The two backends

`process` keeps the existing backend, which spawns the handler directly.

`firecracker` spawns the configured control program directly, without a
shell. Yon sends it the same length-prefixed Handler Protocol v1 request that
the built-in adapters use. The control program receives these values as
separate arguments:

- canonical project and source metadata;
- the adapter identity;
- the pool;
- the CPU count and memory size;
- the deny-egress policy.

The control program must return one bounded Handler Protocol v1 response. It
must accept the existing cancellation frame. Yon still supervises the control
program for deadline, stderr, output, cancellation, forced termination, and
concurrency.

### Supported languages

The driver contract currently accepts only validated JavaScript and Python
source snapshots. Their stable adapter identities are `javascript.v1` and
`python.v1`. Yon resolves the project-relative `--source` identity below an
owned, project-shaped `--project-root`. As a result, replacing an authored
file at its original path on disk cannot change what the control program
transfers.

TypeScript and the Java, PHP, Kotlin, C#, and Rust direct paths depend on a
runtime workspace that Tachyon prepares. The project-relative `--source`
argument never represents those workspaces. Firecracker mode therefore
rejects them with `TY2010` before the driver starts.

A future extension must define an authenticated, bounded transfer of a set of
artifacts. Passing one prepared artifact path, or the authored source, instead
is not an artifact contract.

### Trust in the control program

The control program is a trusted deployment component. It must prove that the
selected pool really uses all of these:

- Firecracker and the jailer;
- unique credentials;
- cgroups and namespaces;
- seccomp;
- immutable images;
- safe snapshot lineage;
- deny-by-default guest networking.

Selecting the backend alone is not evidence of hardware isolation.

### Where the policy applies

The same environment-selected policy applies to:

- HTTP route handlers;
- middleware;
- scheduled workers;
- explicit `ty handler invoke`.

Yon never executes handlers during compilation.

## Consequences

- Application code cannot downgrade production isolation.
- Handler Protocol v1 stays independent of virtualization and of language.
- A thin control client may connect to a local pool daemon. Then Yon does not
  need to create a new microVM for each HTTP request.
- Process mode keeps its current behavior unless an operator chooses another
  backend.
- Firecracker can be deployed only where the control program and the host have
  independent production evidence.
- Firecracker mode currently supports only JavaScript and Python. If an
  operator selects it for a prepared TypeScript or direct-language handler, Yon
  fails safely and refuses the handler before the driver starts.
- These features need later, separate pieces of work: allowlisted egress,
  writable filesystems, warm-pool policy, snapshot identity, attestations, and
  a first-party control program. This transport boundary does not imply any of
  them.

## Rejected Alternatives

- **Isolation in `tachyon.json`**: An application could weaken the operator's
  security boundary. It also mixes deployment authority into source
  configuration.
- **Invoke the Firecracker binary as a handler**: The Firecracker API does not
  speak Handler Protocol. It does not understand Yon sources or routes.
- **One hard-coded virtualization backend inside the supervisor**: This ties
  application behavior to Linux and KVM lifecycle details. It also prevents
  replacing the backend with another qualified provider.
- **Allow unrestricted egress initially**: This opens a path for SSRF
  (server-side request forgery) and data exfiltration before controls for
  origin, DNS, redirects, and responses exist.

## Acceptance Gate

- Environment parsing tests prove defaults, bounds, missing values, unsafe
  paths, invalid identifiers, and deny-only egress.
- A real executable control-driver fixture receives framed Handler Protocol v1
  and the bounded policy, and returns a validated response.
- Prepared TypeScript and direct-language sources are rejected before the
  control driver starts. JavaScript and Python keep their stable adapter
  identities.
- Process-mode handler tests stay unchanged.
- Every handler entry point builds its supervisor from the same parent
  environment.
- Documentation makes no hardware-isolation claim without qualified driver
  and host evidence.
