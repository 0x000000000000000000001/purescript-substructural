# Applying the library to b8x

Inspection dated 2026-10-06. The application integrations are now consolidated
in b8x branch `codex/ffi-ownership`, based on `master` at `cc9e5ce173`.
The branch changes application call sites under `src/`: PostgreSQL's EventStore
transaction calls and RabbitMQ's Worker subscriptions. These changes are not
merged into `master` or deployed. Tests of those real adapters live under
`run/bak/rust/tests/ownership/`; redundant standalone prototypes and their old
branches have been removed. Generic library regressions stay in this package's
`test/` directory.

Both adapters now use the public `Data.Function.Sub.Aff.Do` module. This
migration replaces the application-specific transaction bind and the consumer's
manual arrow composition. Their native operation behavior is preserved; the
PostgreSQL destructor now explicitly names `std::ops::Drop` to avoid a generated
import of the library's `Drop` class masking the Rust trait.
The migrated sources passed 16 PostgreSQL assertions and 14 RabbitMQ observations
against disposable services. The 1,352-module Worker closure also passed TAST,
native generation and `cargo check`. These results are recorded with fresh
source fingerprints in the application snapshots linked below.

## Actual local interfaces

There are **16 Rust source files under b8x `src/`**. This is a source inventory,
not a percentage of FFI safety or of executed application paths.

| Local implementation | Ownership relevance |
| --- | --- |
| `Infra/Client/Postgres/Postgres.rs` | Pool/dedicated handles, queued operations, release and transaction protocol. Used by the indexed transaction API. |
| `Infra/Client/RabbitMq/RabbitMq.rs` | Connections, channels, publication and the existing callback API. |
| `Infra/Client/RabbitMq/Consumer.rs` | Dedicated subscription, awaited delivery handling, one-use settlement and cancellation cleanup. Used by the public `Aff.Do` adapter. |
| `Util/Run/Router.rs` | Retained reusable handlers in a persistent builder; registration copies the map. No one-shot resource consumption is exposed by this API. |
| `Util/Json/Native.rs` | Synchronous success/error callbacks translating parsing results; no retained borrowed reference. |
| `Util/Json/TaggedSum.rs`, `Util/NeedOpt.rs` | Ordinary foreign-value/record transformations. |
| `Util/Html/Clean/Clean.rs`, `Util/Html/Encode/Encode.rs`, `Util/Type/String/String.rs` | String/array transformations with owned results. |
| `Util/Crypto/Hash.rs`, `Util/File/Path.rs` | Hashing and path values; the Promise-shaped hash operation exposes no consumable handle. |
| `Util/Type/Ulid.rs`, `Infra/EventStore/Postgres/EventStore.rs` | Identifier generation with synchronized native state; no ownership token returned. |
| `_/Config/InternalConfig.rs`, `_/Config/PublicConfig.rs` | Configuration values. |

The common branch addresses two concrete resource protocols. Ordinary value
conversions still need semantic and encoding tests; absence of an ownership
protocol is not proof that an FFI has no defects.

## PostgreSQL: application integration

The two actual EventStore append paths pass a program directly to
`withStoreTxConnectionHandle`; they no longer receive a transaction value in a
callback. `Infra.Client.Postgres.Transaction.Program scope a` is an alias for
the library's `Program (Tx scope) (Tx scope) a`. Queries preserve the hidden
transaction, while commit and rollback leave `Unit`. The Store wrapper supplies
the terminal operation according to the query outcome, within one `Aff.bracket`.
SQL, parameters and the returned `Either Error (Array Foreign)` are unchanged.

The adapter keeps `Tx scope` opaque and nominal. It grants `Shared` to its
private-constructor `SqlRows` and `QueryResult` wrappers, relying on the driver
to return ordinary SQL data and errors without hidden capabilities. It does
not add blanket `Shared` instances for `Foreign`, `Error` or transactions.
`queryDecoded` can instead expose an already shared decoded result. Native
acquisition, parameterized queries and release use the existing
`src/Infra/Client/Postgres/Postgres.rs` implementation and its real pool;
the library supplies sequencing, while the adapter supplies finalization.

The runner compiles the EventStore import closure and checks nine invalid typed
uses, including resource-scope coercion and attempting to expose a transaction
through `fromArrow`. A compile-only control documents that arbitrary SQL can
bypass the typed protocol; it is never executed. Its 16 native assertions cover
parameters/results, returned and thrown errors, SQL failure, cancellation while
native SQL is active, BEGIN failure, replay and connection reuse. The added
`do` scenarios cover dependent decoded queries, branching with lifted effects,
unselected branches, explicit commit/rollback and decoder-failure rollback.
The migrated-source run passed; see the source hashes in
[application results](../../../b8x/run/bak/rust/tests/ownership/postgres/results.json).

This executes the transaction wrapper, not the complete EventStore workload.
The separate advisory-lock wrapper remains unchanged. Raw SQL is trusted, and
cancellation still waits for submitted SQL before rollback; it does not send a
PostgreSQL CancelRequest. Other language backends have not been validated.

## RabbitMQ: application integration

Both actual command/event subscriptions in
`Inter.Cli.Worker.Main` call `Infra.Client.RabbitMq.Consumer.assertConsume`
with an `Aff` handler. The application module imports the public `Aff.Do`
operations for ordinary `do`: `readBody` produces a shared `String`,
`liftAff (handler body)` waits for processing, and `acknowledge` consumes the
hidden receipt. The private program has type `Program Receipt Unit Unit`;
business code receives only the message body. The existing scoped runner executes
`toArrow` inside its subscription bracket. Its Rust FFI still uses long-lived
`basic_consume` on a dedicated channel with prefetch 1.

The native application consumer test retains 14 observations over seven
disposable queues: asynchronous handling before acknowledgement, handler error, active
and idle cancellation, independent subscriptions, continued publication and
failed acquisition. Sentinel deliveries prove that earlier acknowledgements
reached the broker. The executable exits naturally after cancellation, which
also checks that no pending native receive keeps its runtime alive. These checks
passed again on the migrated sources. See the
[application evidence](../../../b8x/run/bak/rust/tests/ownership/rabbitmq/evidence.json)
and [integration contract](../../../b8x/run/bak/rust/tests/ownership/rabbitmq/README.md).

Worker supervision cancels the other subscription when one fails and waits for
resource cleanup before exiting. Existing business errors that handlers log and
return normally are still acknowledged; the change does not silently redefine
that policy. Signals retain the existing busy-counter behavior, not a stronger
stop-accepting-and-drain protocol. Network failure, reconnect behavior and
exactly-once business processing remain outside this validation. The complete
Worker process has not been run against application services.

The complete Worker import closure was revalidated separately through TAST,
purust generation and `cargo check` on the migrated sources. No application
services or Worker process were started for that check. An earlier check
required aligning the existing
`purust-arraybuffer-types` opaque `ArrayView` declaration with the backend's
non-generic foreign-handle ABI. That small port correction is not an
implementation or runtime qualification of native typed arrays or Affjax.

## Dependencies beyond local Rust files

The existing default native build `build-Oxu0in` resolves 1,404 modules. Its
manifest points FS, Aff, Stream and EventEmitter at the sibling `purust-*` ports;
their checked source hashes still match the manifest. Its recorded 286/286 test
execution is historical evidence, not a fresh whole-application run for this
audit and not proof that every foreign operation ran.

The concrete additional application concern is
[`Infra.Cache.Fs.Cache`](../../../b8x/src/Infra/Cache/Fs/Cache.purs): it writes a
unique temporary file and then renames it, without a temporary-file finalizer.
FS uses blocking IO on a worker; cancelling an Aff wait does not stop that IO.
An error or interruption before rename can therefore leave cleanup work to do.
This deserves its own protocol test; it is not established here as a memory
safety defect, nor solved merely by adding indexed types.

During the earlier audit, the existing Aff regression
`resumption_tests::from_blocking_kill_waits_for_late_work` was rerun successfully.
It checks that cancellation returns before the blocking
work completes, that the runtime waits for that work, and that the cancelled
continuation is not called. The recorded source fingerprint was checked against
the current runtime. No runtime was rebuilt or changed for that check.

Other dependency APIs expose file descriptors, streams and listeners. Their
presence in the dependency graph does not establish use of those operations in
b8x. HTTP and WebSocket were absent from this manifest's resolution set and are
not counted as exercised native FFI here.

## Interpreting coverage

This application has concrete ownership-sensitive areas where the library's
experiments are useful, alongside many ordinary value interfaces. It supports
targeted abstractions around resource protocols while keeping much of the caller
API conventional. It does not support estimating a universal percentage of Rust
FFI coverage, or claiming that the application is fully protected because the
integration tests pass. Each test suite records its exact executions;
[COVERAGE.md](COVERAGE.md) describes the library-wide limits.
The public indexed `do` preserves the tested resource transitions and shared
result boundary. It does not infer Rust lifetimes, prove arbitrary alias
exclusion or replace the adapters' cleanup and native safeguards.
