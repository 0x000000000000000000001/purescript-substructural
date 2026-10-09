# Asynchronous native resources and cooperative cancellation

This experiment exercises a real Rust `Future` from an ordinary PureScript
`Aff` program through the installed Purust FFI. It changes no compiler or
production port. It is a separate experimental API, not a Rust implementation
of `src/Data/Function/Sub.purs`.

From this repository:

```sh
node test/ffi/rust/run.mjs --suite async-resources
```

Fixtures use `.purs.fixture` to stay outside the ordinary JavaScript test glob.
The shared runner restores `.purs` in a scratch workspace. The local hook expands
`native-core.rs` into that scratch copy of `Native.rs`; the same core is also
compiled by the standalone Rust controls. Reports and generated sources are
written to the ignored `test/ffi/rust/artifacts/async-resources/` directory.

## Execution modes

**Threaded mode executes the complete PureScript → Rust → PureScript path.**
The installed `Effect.Aff` port uses an Arc-based worker runtime, and Purust's
`--threaded` mode supplies its Tokio dependency.

**Normal mode currently fails to compile that installed port.** The separate
[AffNormal.purs.fixture](AffNormal.purs.fixture) contains only
`launchAff_ (pure unit)` and imports none of this experiment's resource wrapper.
The hook compiles it in normal mode and records the actual compiler diagnostics:
`Rc`/`Arc` callback mismatches and missing `Send`/`Sync` implementations inside
`Purs_Effect_Aff` (`E0631`/`E0277`), as well as the missing normal-mode Tokio
dependency (`E0433`). The port's own native regression runner uses `--threaded`.
Adding a dependency alone would not resolve the independently reported ownership
type mismatches. No generated port code is patched to bypass them.

The suite's `complete: true` means all specified checks, including that recorded
mode rejection, matched expectations. It does **not** mean normal-mode async
execution succeeded, or that library-based cancellation is impossible there.
If that port later supports normal mode, the expected-rejection hook must be
replaced with a real normal-mode execution.

## The actual asynchronous operation

[Native.purs.fixture](Native.purs.fixture) exposes `withResource`, implemented
with `Aff.bracket`. Acquisition allocates a non-`Clone`, non-`Copy` Rust owner
containing a `Vec<u8>`. A custom future computes the sum of those bytes after
receiving a completion signal; the observed result is not supplied by the test.

In these scenarios, completion and cancellation wait for `awaitStarted`, so
the future's first poll returns `Poll::Pending` and records its task `Waker`.
`awaitStarted` is resolved by that first poll, so the test never relies on an
arbitrary delay before cancellation. A signal sent before the first poll could
instead make that poll return `Ready`. A completion, failure or cancellation
signal updates mutex-protected state and wakes the native task. `Box::pin`
owns the future, which is deliberately `!Unpin` through `PhantomPinned`.
This tests an actual pinned future; it makes no claim about importing arbitrary
self-referential native values or Rust lifetimes into PureScript.

The `makeAff` canceler signals native cancellation and then waits for the
future's `Drop` handshake. The bracket finalizer also waits for that handshake
before taking and destroying the byte owner. The native guard rejects release
while an operation remains active. Thus the bytes cannot be freed while a
cooperating operation still uses them.

The native completion callback may be queued after the waiting Aff is killed.
A gate rejects delivery if cancellation was recorded before delivery was
accepted. It rejects duplicate delivery attempts too. An accepted delivery
immediately preceding cancellation can still call Aff's completion callback;
Aff then arbitrates its fiber state. This wrapper does not promise an atomic
boundary spanning native delivery and all subsequent PureScript continuation
code. Completion callbacks inspect only the inert control object, not freed
resource bytes.

## Checks

[Main.purs.fixture](Main.purs.fixture) runs these end-to-end scenarios:

| Scenario | Assertions |
| --- | --- |
| Success | Actual native byte sum reaches PureScript; killing the already completed fiber repeats no cleanup. |
| Native error | The original error reaches `attempt`; no success continuation runs. |
| Cancellation while Pending | `killFiber` waits for native shutdown and bracket cleanup; late completion is suppressed; no success continuation runs. |
| Completion/cancellation competition | Separate Aff fibers attempt completion and cancellation; either allowed outcome has one finalizer and one terminal callback attempt. |
| Second start while the first is pending | A catchable error is returned without poisoning the mutex; the original operation still succeeds. |
| Unused scope and escaped handle | Bracket releases a resource even without starting a future; a subsequent operation on the escaped handle raises a catchable closed-resource error. |

Every case checks exactly one acquisition, one finalizer release and one resource
`Drop`. Cases that start an operation check one future `Drop`; the unused scope
checks zero. A further attempted duplicate delivery is refused after each
completed or cancelled operation.
The competition is a scheduling probe, not an exhaustive proof of every
possible interleaving. Ordered complete-before-cancel and cancel-before-complete
cases are separately tested.

[native-controls.rs](native-controls.rs) compiles the same core without
PureScript. It explicitly polls Pending, checks wake notification and the byte
sum, rejects premature and duplicate release, then runs 32 real OS-thread races
using `Barrier`. A separate Rust compile-fail control confirms that the future
cannot satisfy an `Unpin` bound (`E0277`). These are additional native controls,
not substitutes reported as PureScript executions.

No service, network or sleep is involved. Each subprocess has the shared
runner's 60-second timeout; a protocol regression that leaves a future waiting
fails the run instead of waiting indefinitely.

## Boundaries

Cancellation is cooperative: the custom future handles a cancellation signal
and wakes promptly. Arbitrary blocking FFI code cannot be forcibly interrupted
by this API. A noncooperating operation would delay finalization until the
runner timeout. No process abort, network failure, external side effect rollback
or asynchronous destructor failure is modeled.

Each diagnostic control permits one acquisition and one operation. Resource
handles remain ordinary PureScript values and can escape their scope; the tested
second-start and post-release guards reject those invalid operations at runtime.
The wrapper and bracket enforce the lifecycle dynamically. This suite does not add linear PureScript types, prevent
all handle escape, or grant arbitrary user closures Rust lifetime guarantees.
The public acquisition and cleanup are scoped by `bracket`; its acquisition
and finalizer masking semantics are used, but cancellation during a long-running
acquisition itself is not exercised here.
