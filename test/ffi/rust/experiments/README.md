# Experimental Rust FFI adaptations

These eight suites investigate ownership mechanisms implemented with PureScript
libraries and trusted Rust wrappers. Most use separate `LinearLab.*` APIs.
`library-rust` instead copies the actual public `Data.Function.Sub` and
`Data.Array.Unique` modules unchanged and supplies experimental Rust adapters.
The public modules' native implementations now live beside their PureScript
sources and are copied into this compatibility suite. The independent
`LinearLab.*` APIs remain experiments under `test/`.

PureScript sources use the `.purs.fixture` suffix so ordinary `spago build` or
`spago test` globs such as `test/**/*.purs` do not compile these Rust-specific
fixtures as JavaScript. The Rust runner copies them into a temporary workspace,
restores their `.purs` extension and keeps adjacent `.rs` files in place.

Run one suite from the repository root:

```sh
node test/ffi/rust/run.mjs --suite capabilities
node test/ffi/rust/run.mjs --suite borrowed-views
node test/ffi/rust/run.mjs --suite native-callbacks
# Or execute every suite:
node test/ffi/rust/run.mjs
```

| Suite | Mechanism |
| --- | --- |
| [capabilities](capabilities/README.md) | Opaque arrows, conditional clone/discard/share capabilities, and an independently supplied native resource. |
| [borrowed-views](borrowed-views/README.md) | Region and borrow-state indices combined with dynamically checked native owner/range views. |
| [native-callbacks](native-callbacks/README.md) | Actual Rust `FnOnce` and `FnMut` closures exposed through runtime-checked PureScript handles. |
| [advanced-borrows](advanced-borrows/README.md) | Nested shared leases, mutable reborrows, disjoint slices, returned subviews and exception cleanup. |
| [thread-affinity](thread-affinity/README.md) | A real `!Send` / `!Sync` owner retained in a thread-local arena behind checked handles. |
| [native-shapes](native-shapes/README.md) | Generic and opaque iterators, trait objects, and actual `Result` / `Option` payload conversion. |
| [async-resources](async-resources/README.md) | Pinned futures and cooperative cancellation through `Aff.bracket`, with native race controls. |
| [library-rust](library-rust/README.md) | The actual public library modules and their tests with native adapters, non-clonable owners and nested containers. |

The suites do not import each other or any source from their former test
location. Imports between `LinearLab.*` modules resolve inside the corresponding
suite. In `capabilities`, `ForeignExample` depends on that suite's `Sub`; in
`borrowed-views`, the existential probes share that suite's `Existential` module.
These are internal dependencies. The deliberate exception is `library-rust`,
whose hook reads public library sources from this repository and records their
hashes in its report.

Execution requires Node.js, the typed PureScript compiler fork, a configured
Purust checkout, Rust/Cargo, and cached PureScript/native dependencies. The shared
runner supplies those dependencies and generates fresh Rust. Synchronous suites
execute in normal and threaded modes. The async suite executes in threaded mode
and separately records the current normal-mode `Aff` port's build failure.
No service is required. Compiler rejections are intentional
fixtures checked against their expected diagnostics; they are not modules to
include wholesale in a normal successful build.

Only source fixtures, suite configuration and documentation are kept here.
Reports and generated output are produced afresh under the ignored
`test/ffi/rust/artifacts/` directory. The repository-wide
[results snapshot](../../../results.json) records the verified runs. Each suite's
README states its static guarantees, runtime checks and remaining limitations;
the [coverage matrix](../../COVERAGE.md) compares them.
