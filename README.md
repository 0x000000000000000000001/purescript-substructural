# purescript-substructural

(WIP)

Substructural operations for PureScript: compose computations whose values can
be cloned, discarded or shared only through the corresponding capabilities.

This fork continues rightfold's library, preserved in
[nfgrusk/purescript-substructural](https://github.com/nfgrusk/purescript-substructural).
The original BSD-3-Clause license and Git history are retained. The historical
public API is preserved while its implementation is updated for PureScript
0.15.15, JavaScript ES modules and Spago.

## Using the library

`Data.Function.Sub` exposes the abstract `Sub a b` arrow, written `a -* b`,
composition, and the `Clone`, `Drop` and `Shared` capabilities.
`Data.Array.Unique` supplies an array API built with these operations.

```purescript
import Prelude
import Data.Array.Unique as Unique
import Data.Function.Sub (borrow, fst', runShared)

reversed :: Array Int
reversed = runShared
  (Unique.fromShared >>> Unique.reverse >>> Unique.toShared)
  [1, 2, 3]

-- Observe through a borrow, then continue using the owner.
observedThenReversed :: Array Int
observedThenReversed = runShared
  (Unique.fromShared >>> borrow Unique.length >>> fst'
    >>> Unique.reverse >>> Unique.toShared)
  [1, 2, 3]
```

Both examples return `[3, 2, 1]`. The shared input array remains unchanged.
`runShared` requires shared input and output types; a non-shared resource stays
inside the composed computation. The type system checks the exposed capability
constraints. Primitive implementations and instances must respect their
contracts. Explicit `unsafeClone`, `unsafeDrop` and `unsafeCoerce` remain escape
hatches, and ordinary PureScript functions do not acquire linearity automatically.

## Effectful resource protocols

`Data.Function.Sub.Aff` adds `SubAff a b`: an opaque arrow whose steps execute
in `Aff`. Composition waits for the preceding step; `tensor` sequences its two
branches. The opaque `Pair` carries multiple resources without exposing ordinary
unrestricted projections. The implementation is PureScript and needs no new
backend-specific FFI.

A native adapter can expose operations for its own resource. Inside b8x's
`Infra.Client.RabbitMq.Consumer`, these private primitives carry the real
delivery's settlement right through the public indexed `do` API:

```purescript
import Prelude hiding (bind, discard)
import Data.Function.Sub.Aff.Do (Program, ResourceProgram, bind, discard)
import Data.Function.Sub.Aff.Do as Do

readBody :: ResourceProgram Receipt String
acknowledge :: Program Receipt Unit Unit

-- Only the shared body reaches the continuation. The receipt stays hidden.
handleDelivery :: (String -> Aff Unit) -> Program Receipt Unit Unit
handleDelivery handler = do
  body <- readBody
  Do.liftAff (handler body)
  acknowledge
```

These are the adapter's private operations; callers use its ordinary
`assertConsume channel queue handler` API. The imported constrained `bind` and
`discard` give this ordinary `do` its resource protocol. It waits for the handler
before acknowledging, while the adapter owns acquisition, native settlement
and bracketed cleanup. Acknowledgement changes the resource state from `Receipt`
to `Unit`, preventing a second acknowledgement. Omitting it leaves `Receipt`
and cannot satisfy `handleDelivery`'s required terminal state.

The safe library surface offers `liftShared`, `fromShared` and `runShared`, each
requiring the existing `Shared` capability on both boundaries. A receipt has no
such instance. Adapter authors use the explicitly named
`Data.Function.Sub.Aff.Unsafe` module to define primitives and scoped runners;
ordinary callers do not need it. Its constructors are still private and it
exports no `Newtype` instance. See the source comments for those trusted operations.

This API deliberately specializes to `Aff`. An arbitrary `Monad`, such as
`Array`, can duplicate a continuation and a resource it captures. Exceptions
and cancellation can still interrupt a composed arrow, so a resource adapter
must provide its own finalization and native safeguards. The library does not
promise successful settlement on every execution or exactly-once business work.

The [b8x integration notes](test/ffi/B8X.md) distinguish the reusable library
from its application adapter and link the real broker tests.

## Do notation with hidden resources

`Data.Function.Sub.Aff.Do` supplies `Program before after a`: an effectful
program that receives a resource of type `before`, leaves one of type `after`,
and produces a shared result `a`. Its `bind` passes the remaining resource to
the next step automatically. `ResourceProgram resource a` is the alias for
programs that preserve the resource type.

```purescript
import Prelude
import Data.Function.Sub.Aff.Do as Sub
import Effect.Class (liftEffect)
import Effect.Console (log)

describe :: forall resource. Sub.ResourceProgram resource Int
describe = Sub.do
  value <- Sub.pure 20
  Sub.liftAff (liftEffect (log "Computing a shared result"))
  Sub.pure (value * 2)
```

The resource stays hidden; `value` is an ordinary `Int`. Operations supplied by
an adapter can also change the resource type. For example, a PostgreSQL query
may have type `Program (Tx scope) (Tx scope) SqlRows`, while commit has type
`Program (Tx scope) Unit Unit`. A subsequent query cannot follow commit because
its required input resource no longer matches. Multiple resources can travel
together inside the opaque `Pair`.

To write ordinary `do` instead of `Sub.do`, select the functions with imports:

```purescript
import Prelude hiding (bind, discard, pure)
import Data.Function.Sub.Aff.Do (bind, discard, pure)
```

These imports choose the operations for unqualified `do` in that module. Mixed
code can additionally import `Prelude as P` and use `P.do` for ordinary `Aff`.
There is no standard `Monad` instance: the custom `bind` both tracks changing
resource types and requires `Shared` for values passed to ordinary callbacks.
`pure`, `liftAff` and `fromArrow` enforce that same shared-result boundary.
The abstract constructor and nominal roles prevent bypassing it with newtype
unwrapping or safe coercion.

Adapters use `fromArrow` to adapt `SubAff before (Pair after a)` operations;
`toArrow` recovers an arrow without exposing the pair's components. Acquisition,
execution and cleanup remain in the adapter's scoped runner. The generic `do`
does not install a finalizer, inspect SQL or infer native lifetimes. Its
guarantees depend on correct `Shared` instances and trusted primitives. For
non-shared values that must be combined directly, the existing arrow and
`Pair` combinators remain available alongside this interface.

## Development and tests

Use Node.js 24 or later. Local development tools are pinned in `package.json`:
PureScript 0.15.15 and Spago 1.0.3.

```sh
npm ci
npm exec -- spago install
npm test
```

`npm test` compiles the accepted and rejected fixtures independently, checking
the diagnostic code and module for each expected rejection, then executes the
actual JavaScript implementation of the library. Each compilation uses fresh
temporary output. Detailed reports are saved under ignored `test/artifacts/`.

```sh
npm run build
npm run test:js             # Standard Spago runtime tests only
npm test -- --keep-output   # Retain the temporary compilation workspace
```

`PURS` can override the compiler used by `npm test`. It must be a compatible
0.15.x compiler. The tests are validated with the stock compiler; a TAST fork
is needed only for the optional Rust experiments below. With dependencies
already cached, Spago commands can use `--offline`.

## Optional Rust FFI experiments

The proof-of-concept work lives under
[`test/ffi/rust/experiments`](test/ffi/rust/experiments/README.md):

* Conditional `Clone` / `Drop` / `Shared` arrows, including a resource supplied
  by a separate FFI module.
* Indexed, scoped borrowed views and native checks for stale or concurrent use.
* Actual Rust `FnOnce` / `FnMut` closures behind checked ordinary effectful handles.
* Nested shared borrows, mutable reborrows and disjoint slice views.
* Thread-bound resources that are genuinely neither `Send` nor `Sync`.
* Generic iterators and trait objects, with real `Result` / `Option` conversion.
* Pinned native futures, errors and cooperative cancellation through `Aff`.
* The actual public `Sub` and `UniqueArray` APIs running with experimental Rust
  implementations, including nested containers of non-clonable native owners.

Most suites use separate experimental APIs named `LinearLab.*` to investigate
future library designs. The `library-rust` suite instead copies the real public
PureScript modules unchanged from `src/` and tests their native adapters.
The purust implementations now live beside those modules as `Sub.rs` and
`Unique.rs`, alongside the JavaScript implementations. Application builds use
these same tested sources; the abstractions remain independent of Rust.
These adapters target purust's current ABI, not an arbitrary Rust FFI toolchain.

Native experiments require a built
[purust checkout](https://github.com/0x000000000000000000001/purust), its installed
PureScript library ports, a TAST-enabled PureScript compiler, and Rust/Cargo with
the required dependencies cached. No service or network is used during the tests.

```sh
PURUST_ROOT=/path/to/purust/checkout \
PURUST_PURS=/path/to/tast-enabled/purs \
npm run test:rust

# One family, with the same environment variables:
npm run test:rust -- --suite capabilities
```

For the existing `htdocs` layout, the runner also discovers `../purust/purust`
and that checkout's local TAST compiler. `PURUST_PURS`, when set, is an executable
path, independent of the `PURS` override used by the JavaScript tests.
The runner invokes the existing backend bundle and native-workspace helper;
it does not depend on the old linearity lab. Reports go to
`test/ffi/rust/artifacts/`. Synchronous suites execute assertions in normal and
threaded modes. Async execution currently requires threaded mode: a minimal
normal-mode `Aff` program exposes build errors in the installed port, recorded
separately from successful executions.

## Layout and current evidence

```text
src/                       General library, with JavaScript implementations
test/Data/, test/Main.purs  Runtime tests of that library
test/compile-pass/         Accepted type-checking fixtures
test/compile-fail/         Rejected type-checking fixtures
test/ffi/rust/experiments/ Optional Rust proof-of-concept suites
test/support/              Shared fixture materialization
```

Fixture sources use `.purs.fixture`, restored to `.purs` only in a temporary
workspace by the dedicated runners. Consequently, normal Spago builds do not
accidentally compile deliberate failures or Rust-only foreign modules.

The [coverage matrix](test/ffi/COVERAGE.md) distinguishes static checks, runtime
guards and remaining limits. The [test notes](test/README.md) describe the test
paths; the [results snapshot](test/results.json) records verified outcomes and
source hashes. The longer-term goal is to improve reusable APIs while keeping
backend-specific machinery separate. General native lifetimes, arbitrary
noncooperative asynchronous operations and an automatic Rust-signature-to-wrapper
generator remain open work. These experiments establish concrete working
adaptations, not exhaustive coverage of Rust FFI.
