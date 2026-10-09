# A Rust resource that cannot leave its thread

This suite tests a real `Rc<RefCell<i64>>` owner through PureScript and purust,
including purust's `--threaded` mode. It does **not** make the owner `Send` or
`Sync`, and contains no `unsafe impl`. The owner remains in a thread-local arena;
only a checked identifier is represented in PureScript.

```sh
node test/ffi/rust/run.mjs --suite thread-affinity
```

No network or service is needed. The existing harness uses the local typed
PureScript compiler, purust and offline Cargo. Reports, commands and generated
Rust are saved under `test/ffi/rust/artifacts/thread-affinity/`; successful
temporary build directories are removed by the harness.

## The program actually executed

```purescript
program :: Effect Int
program = L.withOwner 10 \owner -> L.do
  changed <- L.add owner 7
  observed <- L.read owner
  L.pure (changed + observed)
```

This returns `34`. Executing the same `Effect` value twice acquires and destroys
two separate native owners. The scoped callback executes synchronously on the
calling thread. That is the owner thread; the adapter does not dispatch work
to another thread.

`NativeOwner` contains the real `Rc<RefCell<_>>`, and both mutation and reads
use that cell. `owner.rs` is inlined into the adapter by the suite's preparation
hook and is also included verbatim by the independent Rust trait controls.
The `thread_rc` module alias prevents purust's ordinary threaded `Rc` conversion
from replacing this intentionally thread-confined resource. The hook checks
that the generated adapter preserves it in both modes.

## Three different protections

**PureScript API.** A generative `region` parameter ties `Handle region` and
`Local region a` to a particular invocation of `withOwner`. Both region roles
are nominal, the constructors and native operations are hidden, and there is
no public conversion from `Local` to ordinary `Effect`. Direct handle escape,
deferred actions/closures with an escaping region, coercion to another region,
and execution through the ordinary effect API are rejected by the compiler.
Handle aliases within one scope are allowed; this is scoped access, not a
linear type for the handle itself.

**Rust's type system.** Moving the exact `NativeOwner` into `thread::spawn` is
rejected with `E0277`. Requiring that same type to implement `Sync` is also
rejected with `E0277`. The native region guard additionally carries a local
`Rc` marker, so it cannot be transferred to another thread. No raw pointer or
unsafe thread-safety assertion is involved.

**Runtime boundary.** A `NativeHandle` contains a globally unique identifier
and the creation thread's `ThreadId`, not a pointer to the owner. Every access
checks the thread, the entry's continued existence and the currently active
scope before touching the arena. Identifiers are never reused; exhaustion
fails instead of wrapping. Transporting a handle therefore does not transport
the owner or grant permission to access it elsewhere.

The `RegionGuard` removes and destroys the native owner on scope exit, outside
the arena's `RefCell` borrow, on its original thread. Every native destructor
asserts that thread identity. Keeping or even forgetting a handle cannot
prolong this resource lifetime.

## Executed checks

All **12 PureScript cases** meet their expected outcome: **3 accepted** and
**9 rejected**. One acceptance is an explicit `unsafeCoerce` control that is
never executed.

The other important acceptance is **existential storage**: rank-2 regions do
not prevent hiding a handle in an existential package. `Main` really retains
such a package through PureScript. A test-only FFI observer deliberately
bypasses `Local` afterwards and confirms that native access reports the handle
as closed. That observer is test support, not a public operation of the API.
Attempting to revive the package through the public API in a new region is a
separate rejected typing case.

Both native modes pass these runtime checks:

* Replay creates fresh owners and returns `34` twice.
* A genuine PureScript `Effect.Exception` is raised within `Local`, caught
  outside `withOwner`, and leaves no live owner or active scope.
* A real other OS thread receives a handle; reads and writes are refused. The
  original thread can still use the owner and observes no foreign mutation.
* A closed handle cannot read, write, or revive when a later scope is opened.
* Inside a nested native scope, an outer handle is rejected; after the inner
  scope closes, the outer scope works again.
* Native unwinding cleans the owner; a retained handle then reports closed.
* A delayed native closure cannot access its former owner after scope exit.
* Forgetting a handle does not leak its owner.
* Another OS thread can independently create, use and clean its own TLS owner.

The final counters are **12 acquisitions and 12 destructions**, with empty
owner and scope tables. Each destruction also checks its creation thread.

```text
THREAD_AFFINITY_GUARDS_OK wrong-thread=blocked stale=blocked nested-scope=blocked unwind=cleaned forgotten-handle=cleaned worker-local=ok
THREAD_AFFINITY_OK results=34,34 ps-exception=cleaned ps-existential=expired owners=12 destroyed=12
```

Three additional standalone `rustc` controls pass: one valid local execution,
one refused `Send` use, and one refused `Sync` use. The report records the
owner source hash and preservation of the `Rc` representation in both native
modes.

## What this establishes, and its limits

A library adapter can expose useful functionality of a genuinely non-`Send`,
non-`Sync` Rust owner to PureScript even with a threaded runtime. This strategy
keeps ownership and execution on the same thread and rejects remote access;
it does not make an arbitrary resource usable on arbitrary threads.

This is a synchronous scoped API with one owner per invocation. Nested-scope
and cross-thread adversarial checks exercise the native backstop directly;
the public API does not expose an arbitrary scheduler or an effect-lifting
escape hatch. Sending a generated PureScript callback to another thread, an
asynchronous dispatcher, and a thread-bound event-loop API are not tested.

Cleanup is established for normal return, PureScript exceptions and native
unwinding. Aff cancellation, suspension/resumption on another thread,
nontermination, process abort and thread termination during unwinding are not
covered. `--threaded` runs the same PureScript program with that runtime mode;
the explicit OS-thread tests are implemented in the trusted native observer.

All FFI signatures and implementations remain trusted. Explicitly unsafe
coercion or dishonest native code can bypass the static API; the ordinary
runtime guards still reject the stale and alien-thread handles tested here.
The suite changes no production compiler, backend, runtime or library core.
