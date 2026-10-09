# Rust FFI coverage and limits

Verified on 2026-10-06 with the tool versions and source hashes in
[`test/results.json`](../results.json). These are executable examples and
counterexamples, not a proof that a library can express every Rust API.

The [b8x application audit](B8X.md) maps these mechanisms to existing application
FFI on the common `codex/ffi-ownership` branch.

The aggregate snapshot totals **135 PureScript typing cases**: 31 accepted
and 104 rejected, with **19 successful end-to-end executions**
(one JavaScript, 18 Rust), plus separate native controls. The public checks
account for 56 cases (9 accepted, 47 rejected); the native suites account for
79 (22 accepted, 57 rejected). These counts describe fixtures and program runs,
not 135 distinct Rust language features. The minimal
normal-mode `Aff` program's build failure is recorded separately.
The public indexed `Aff.Do` addition reran all 56 public typing cases and its
JavaScript execution, plus all five `library-rust` typing cases and its three
executions. The other seven native suites retain their prior
evidence rather than being represented as freshly rerun.
The separate b8x RabbitMQ integration adds application evidence for `SubAff`:
14 observations of the actual application consumer over seven queues. Its Worker
import closure passes native generation and `cargo check`; the PostgreSQL
integration executes the actual indexed transaction wrapper. These application
results remain outside the package totals.

## What works, and where the guarantee lives

| Need | Executed evidence | Static boundary | Native implementation and remaining limit |
| --- | --- | --- | --- |
| Conditional duplication and discard | [capabilities](rust/experiments/capabilities/README.md): deep copies, shared aliases, explicit finish, independent FFI resource | Missing `Clone`, `Drop` or `Shared` instances reject the specified operations | Primitive implementations and instances are trusted. A Rust destructor does not by itself grant the PureScript `Drop` policy. |
| Actual public library on Rust | [library-rust](rust/experiments/library-rust/README.md): unchanged `Sub` / `UniqueArray` modules, their existing tests, generic shared inputs/outputs | Real `runShared` and conditional container instances reject non-shared escape and cloning non-clonable owners | Purust adapters live beside the public modules; tests copy those same files. They retain the ABI and ownership limits documented in the suite. |
| Effectful public resource arrows | [public tests](../README.md) exercise awaited composition, tensor, swap and fresh resources on replay; [b8x RabbitMQ](B8X.md) uses `SubAff` for a real settlement receipt | Opaque nominal arrows/products, constrained shared boundaries, and typed receipt consumption | Trusted adapters construct resource primitives and own bracketed cleanup. The API is specialized to `Aff`; it does not supply generic finalizers or prove broker acknowledgement after network failure. |
| Public indexed `do` over resource arrows | [public tests](../README.md) and [library-rust](rust/experiments/library-rust/README.md) use the same runtime module for dependent results, branches, effect order, replay, failure and cancellation; 18 new typing fixtures cover qualified and ordinary imported `do` | `Program before after a` keeps state in an opaque `Pair`; only `Shared` results cross into ordinary callbacks. Nominal roles and terminal state requirements reject the tested escapes and invalid transitions | The trusted scoped runner owns acquisition and cleanup. The Rust execution uses threaded mode. Two resources in `Pair` are a typing control, not a general proof of independent lifetimes or alias exclusion. |
| Owned resources in containers | [library-rust](rust/experiments/library-rust/README.md): non-clonable native owners in arrays and nested arrays; exact destructor counts | Container capabilities require corresponding element capabilities | Moving the wrapper's contents invalidates its old slot. Alias checks and actual release still depend on the adapter. |
| Shared borrow and mutation exclusion | [borrowed-views](rust/experiments/borrowed-views/README.md): read/mutation phases, stale views, real thread contention | Nominal region and state indices reject the tested scope/state violations | Owner/range/epoch handles are checked at runtime; actual `&[u8]` exists only during the native call. |
| Nested shared borrows | [advanced-borrows](rust/experiments/advanced-borrows/README.md): nested leases and ancestor reads | State indices retain each lease token; an expired child's action is rejected | Reading an ancestor requires explicit `parentRead`. Runtime lease membership backs up the types. |
| Mutable reborrow and disjoint slices | [advanced-borrows](rust/experiments/advanced-borrows/README.md): real native reborrow and `split_at_mut`; exceptions restore parent access | A parent writer cannot be used during a child phase | Handles are ordinary duplicable values, with access serialized and validated. Reborrowing one split half conservatively suspends its sibling too. This is not unrestricted Rust lifetime inference. |
| References returned by native functions | [advanced-borrows](rust/experiments/advanced-borrows/README.md): real returned native subview | Returned handles retain region and lease indices | The adapter converts the reference to a checked owner/range handle; it does not store an arbitrary borrowed Rust reference inside PureScript. |
| `FnOnce` / `FnMut` with non-clonable captures | [native-callbacks](rust/experiments/native-callbacks/README.md): invocation, replay, exceptions, reentrancy, close and destruction | Independent rustc controls confirm the closure constraints | Ordinary PureScript handles remain unrestricted. Runtime slots prevent double consumption and reject reentrant mutation. This suite is synchronous. |
| Thread-bound `!Send` / `!Sync` resources | [thread-affinity](rust/experiments/thread-affinity/README.md): actual `Rc<RefCell<_>>` owner, wrong-thread access and twelve owner destructions | Nominal scoped API plus rustc `Send` / `Sync` rejection controls on the same owner source | The owner stays in a thread-local arena; transported handles are checked. No automatic scheduling back to its thread, or async migration support. |
| Generics, trait objects and native result types | [native-shapes](rust/experiments/native-shapes/README.md): `Box<dyn Trait>`, `impl Iterator`, two generic instantiations, real `Either String (Maybe Int)` results | Opaque cursor cannot be forged/coerced through the tested public API | Explicit adapters instantiate generics and translate values. Aliasing, replay and escaped effects work with dynamic lifecycle checks. No arbitrary signature derivation. |
| Pinned future and asynchronous release | [async-resources](rust/experiments/async-resources/README.md): actual `!Unpin` future, native byte result and error, `Aff.bracket` cleanup | rustc rejects requiring `Unpin`; no linear PureScript handle claim | Threaded only with the installed port. Cooperative shutdown completes before resource release. No arbitrary self-referential value conversion is established. |
| Cancellation and completion competition | [async-resources](rust/experiments/async-resources/README.md): six end-to-end scenarios, ordered cancellation/completion and 32 native OS-thread races | Handles can escape and a second start can typecheck | Runtime gates reject duplicate start, access after release and duplicate delivery. Arbitrary blocking operations cannot be forcibly cancelled. Tested schedules are not an exhaustive race proof. |

All native end-to-end results run through PureScript → TAST → purust → Rust.
The TAST compiler and backend were not modified for these extensions. Static
rejections check both the diagnostic code and the fixture's module, so an
unrelated failing dependency cannot count as the intended protection.

## Two observed limitations

**The installed `Effect.Aff` port fails in normal mode.** Even
`launchAff_ (pure unit)`, without the resource wrapper, produces normal-mode
Rust build errors in that port: `Rc`/`Arc` callback mismatches, missing
`Send`/`Sync` implementations and a missing Tokio dependency. Threaded execution
passes. This is an implementation constraint measured in the current port,
not a proof that a PureScript library cannot support async resource management.

**Throwing custom cleanup does not run every later cleanup method.** The public
library's experimental native `Drop (UniqueArray a)` adapter visits elements
in order. When the first element's custom method throws, later protocol methods
are skipped. The test observes one method invocation but three native owner
destructors, because RAII still releases the remaining native values during
unwinding. That does not prove that application-specific obligations, such as
flushing or sending a protocol message, were fulfilled. An effectful cleanup API
needs an explicit policy for continuing cleanup and reporting multiple errors.

## What this means for a reusable library

The tested approaches complement each other. Capability-constrained arrows
express whether duplication, discard or sharing is allowed. Indexed, scoped
operations express state transitions and borrowed access. Ordinary `Effect` /
`Aff` wrappers remain useful when the native adapter can safely own and check a
resource without exposing its protocol in every caller's type.

Keeping all of this in a library is viable for the concrete cases above, without
changing the PureScript language. It still requires trusted native adapters and,
for some APIs, runtime checks or a more restricted interface. A library does not
automatically turn unrestricted PureScript functions into linear functions.
The existing `runShared` is pure: effectful native operations must not acquire
an arbitrary execution order merely by being wrapped in `Sub`.
`Data.Function.Sub.Aff` provides a separate reusable effectful arrow. Its
RabbitMQ integration carries the actual receipt through the library.
`Data.Function.Sub.Aff.Do` adds reusable indexed composition over that arrow:
resource state remains inside the opaque product while shared results reach
ordinary continuations. Qualified `Do.do` and unqualified `do` with explicit
imports use the same constrained operations; no standard `Monad` instance is
claimed. Specialized borrowing and region APIs remain independent experiments.
Specializing these effectful APIs to `Aff` avoids assuming that every monad
binds its continuation only once, which is false for a branching monad such as
`Array`. Trusted primitive and `Shared` instances remain part of the contract;
the indexed layer supplies neither lifetime inference nor automatic finalizers.

Work still needed before claiming a general FFI solution:

* Extend the indexed API with reusable resource-specific adapters, integrating
  the independently tested borrowing/region protocols with useful error messages.
* Generalize beyond the tested region/state shapes and two-resource product:
  arbitrary independently scoped resources, lifetime-bearing structures and
  higher-order borrowing relationships remain unproved.
* Define cleanup failure policy and support the required async execution modes.
  Noncooperative cancellation, long-running acquisition and asynchronous
  destructor failures are outside these probes.
* Design or generate adapters for more signatures, associated types, callback
  lifetime contracts and thread-affine event loops. None is automatically
  inferred by these tests.
* Validate further application FFI beyond the tested RabbitMQ scope and
  PostgreSQL experiment, and measure costs. No performance claim follows from
  these correctness experiments.

Explicit unsafe operations, dishonest instances and broken foreign
implementations remain outside the safe API contract. Accepted control fixtures
make those trust boundaries visible; acceptance is not itself a safety claim.
