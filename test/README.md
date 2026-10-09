# What the tests establish

## The public library

`npm test` uses the actual `src/Data/Function/Sub.purs` and
`src/Data/Array/Unique.purs`, `Data.Function.Sub.Aff`, `Aff.Unsafe` and `Aff.Do`,
together with their JavaScript dependencies and test-only observers.
There is no copied historical module in these checks.

The original 20 typing fixtures have 4 expected acceptances and 16 expected rejections.
They cover both boundaries of `runShared` and `liftShared`, the `Shared` result
of `borrow`, conditional tuple/array instances, private unchecked helpers, and
attempts to coerce or unwrap abstract foreign types. Positive controls include
ordinary newtype coercions and explicitly unsafe operations that typecheck but
are never executed. Ordinary aliasing of a supplied resource value also
typechecks; the guarantee depends on keeping such values behind the safe API.

The effectful API adds 18 fixtures: one acceptance and 17 rejections. They check
both `Shared` boundaries, opaque constructors and execution, nominal arrow and
product parameters, attempted omission or double consumption, and supplying a
single resource to two consumers combined with `tensor`.

`Data.Function.Sub.Aff.Do` adds 18 fixtures: four acceptances and 14 rejections.
They cover qualified and ordinary imported `do`, dependent shared results,
branches, resource transitions, two resources carried in an opaque `Pair`, and
conversion to and from arrows. Negative cases check resource results at
`fromArrow`, `pure`, `liftAff`, `bind` and `discard`; private construction and
execution; absent `Newtype`; all three nominal parameters; and missing, repeated
or followed-by-query terminal steps. The public library now has **56 typing
cases: 9 acceptances and 47 rejections**.

The runtime tests preserve the original eight assertions and add identity,
composition order, tuple cloning, reuse after a borrow and preservation of the
shared input array. Two foreign observation groups additionally check distinct
unique-array clones and actual calls to custom drop operations. Those observers
are test-only trusted FFI, not public escape hatches in the library.

The `SubAff` runtime tests check awaited composition, sequential tensor branches,
swapping, and fresh resource acquisition when an outer action is replayed. Opaque
test resources record their actual consumption. An error-path test confirms that
later branches do not execute; it does not claim automatic resource cleanup.
The success marker is emitted only after the asynchronous tests finish.

The indexed `Aff.Do` runtime tests exercise awaited effects, dependent results,
branch selection, qualified and ordinary `do`, and a result used after resource
consumption. Replaying the outer action acquires a fresh resource; constructing
the program runs no effects. Failure and cancellation skip later steps, while
the test adapter's scoped `Aff.bracket` runner performs cleanup. That finalizer
belongs to the runner, not the generic `Program` or its `bind`. The same test
module runs unchanged in JavaScript and in the Rust suite's threaded mode.

`npm run test:js -- --strict` exercises the standard Spago test path. Typing
fixtures have their own manifests and fresh outputs so a rejection in a
dependency cannot masquerade as an intended rejection in the test module.

## Experimental native APIs

`npm run test:rust` defines eight suites, with 79 typing cases
(22 expected acceptances, 57 expected rejections) and 18 native executions.
Each executed program is compiled from PureScript through TAST and purust.
Synchronous programs run in normal and threaded modes; async programs run
in threaded mode. A separate minimal normal-mode `Aff` program records the
installed port's build failure, not a successful execution.

| Suite | What is exercised | Main boundary |
| --- | --- | --- |
| [Capabilities](ffi/rust/experiments/capabilities/README.md) | Conditional cloning/discard, immutable sharing, independent FFI extension | Closed `Int -> Int` programs; trusted operations and instances |
| [Borrowed views](ffi/rust/experiments/borrowed-views/README.md) | Scoped reads, mutation exclusion, stale-view guards, a real thread contention check | Indexed synchronous API; native slices exist only inside the read operation |
| [Callbacks](ffi/rust/experiments/native-callbacks/README.md) | Captured non-clonable resources, replay, exceptions, reentrancy, destruction | Ordinary PS handles are checked at runtime; no async cancellation probe |
| [Advanced borrows](ffi/rust/experiments/advanced-borrows/README.md) | Nested reads, mutable reborrows, returned native subviews, disjoint slices, exception cleanup | Indexed states and runtime leases; no native reference stored in a PS value |
| [Thread affinity](ffi/rust/experiments/thread-affinity/README.md) | Actual `!Send` / `!Sync` owners, wrong-thread access, stale handles, cleanup | Synchronous native owner stays in its creating thread's arena |
| [Native shapes](ffi/rust/experiments/native-shapes/README.md) | Generic parsers, `impl Iterator`, `Box<dyn Trait>`, `Result<Option<_>, _>` | Explicit generic instantiations and runtime-checked effectful handles |
| [Async resources](ffi/rust/experiments/async-resources/README.md) | Pinned future, success, error, cancellation, duplicate start, escaped handle | Cooperative cancellation through `Aff.bracket`; threaded execution only |
| [Public library on Rust](ffi/rust/experiments/library-rust/README.md) | Actual public modules, shared boundaries, native owners, nested containers and unchanged `Aff.Do` runtime tests | Experimental Rust adapters; scoped runner cleanup for indexed `do`; throwing a custom drop interrupts later protocol calls |

Most experiments retain their own APIs under `LinearLab.*`; their features are
not automatically features of `Data.Function.Sub`. `library-rust` is different:
its preparation hook copies the actual public PureScript modules and tests
unchanged, records their hashes, and copies their actual adjacent Rust
implementations. The independent experiment APIs remain under `test/`.

Separate Rust controls validate closure ownership (`E0382`, `E0525`), the actual
thread-bound owner's missing `Send` / `Sync` implementations (`E0277`), and the
future's `!Unpin` property (`E0277`). Runtime controls include native thread
contention and 32 completion/cancellation races. Compiling with `--threaded`
alone is not treated as proof of concurrent behavior. Race probes exercise
particular schedules; they are not exhaustive concurrency proofs.

Together with the JavaScript path, the aggregate snapshot contains **135 typing
cases** (31 accepted, 104 rejected) and **19 successful end-to-end program
executions** (one JavaScript, 18 Rust), excluding
the additional standalone Rust controls. An execution may contain many assertions.
See the [coverage matrix](ffi/COVERAGE.md) for what each result establishes and
what remains open.

The `Aff.Do` addition reran all 56 public typing cases and the JavaScript
execution, plus all five `library-rust` typing cases and its three executions.
The other seven native suites retain their previous recorded
evidence; they are not counted as freshly rerun for this addition. The aggregate
snapshot preserves that distinction.

The separate [b8x application integrations](ffi/B8X.md), on the common branch
`codex/ffi-ownership`, exercise the actual public `SubAff` API with a native
settlement receipt. The application consumer has 14 broker observations over
seven queues, and its Worker import closure passes native generation and
`cargo check`. The PostgreSQL integration executes its actual indexed transaction
wrapper. These external application results are not added to the package totals
above; the integration notes distinguish execution from caller compilation.

## Reports and cleanup

The runners write detailed diagnostics, command logs and native output under
ignored `artifacts/` directories. Successful workspaces are removed; failed
ones are retained with their exact path printed for investigation. Pass
`--keep-output` to retain a successful workspace.

[`results.json`](results.json) is a snapshot of runs performed in this fork,
including source/tool hashes and the distinction between accepted controls,
expected rejections and native execution. It is not regenerated by ordinary
test runs; the detailed reports are the current evidence after a rerun.
No performance or universal FFI-coverage claim is made.
