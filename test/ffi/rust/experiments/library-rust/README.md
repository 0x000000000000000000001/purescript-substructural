# The public library running on Rust

This suite copies the actual `src/Data/Function/Sub.purs`,
`src/Data/Array/Unique.purs`, `Data.Function.Sub.Aff`, `Aff.Unsafe` and `Aff.Do`
unchanged into its temporary workspace. It also copies the existing
public-library runtime tests, including `Test.Data.Function.Sub.Aff.Do`.
The hook records each copied source's SHA-256 in `librarySources` in the report.
The adjacent `src/Data/Function/Sub.rs` and
`src/Data/Array/Unique.rs` supply the native implementations, without changing
the public PureScript signatures. Tests copy those actual files too.

```sh
npm run test:rust -- --suite library-rust
```

## Evidence

The existing Sub and UniqueArray runtime tests execute through TAST and purust in normal
and threaded modes: composition, identity, cloning tuples, array creation,
length, borrowing, appending, reversal and custom discard operations. Foreign
observers check distinct clone wrappers and independent array mutation.

Additional PureScript assertions exercise `runShared` on `String`, `Boolean`,
shared tuples and arrays of strings. This execution boundary is no longer the
fixed `Int -> Int` boundary of the earlier interpreter experiment.

`Owned.purs.fixture` adds a non-`Clone` native owner with a lawful discard
operation. The actual `UniqueArray` stores owners, including nested containers.
Three owners are acquired and destroyed by native Rust destructors; one is read
through the public `borrow` operation before being discarded. The counts are
asserted at runtime. Negative fixtures reject cloning that owner, cloning a
container containing it, and returning it through `runShared`.

`DropFailure` executes a throwing element-discard operation inside a test-only
`Effect` observer. The first operation destroys its owner and raises a tagged
native exception. The exception is caught and verified; all three native owners
are destroyed once by Rust unwinding, but only the first PureScript `Drop`
method is called. This establishes a concrete distinction between memory
cleanup and completing every resource-specific finalization protocol.

`LibraryRust.AffDoMain` runs the same `Test.Data.Function.Sub.Aff.Do.spec` as the
JavaScript suite. The test and the three public Aff modules are copied without
rewriting them or substituting a native test implementation. The entry point
prints `SUBSTRUCTURAL_AFF_DO_RUST_OK` only after the entire `Aff` test succeeds.
It adds no custom FFI. This execution uses only threaded mode; the existing
Sub and UniqueArray executions retain both normal and threaded modes.

Running this suite requires Node.js 24 or later, the TAST compiler, a built
purust checkout with its local PureScript ports, and Rust/Cargo with the needed
crates cached. Set `PURUST_ROOT` and `PURUST_PURS` when those checkouts are not
in the runner's usual locations. Cargo runs offline. The Aff test uses purust's
existing Aff, Ref and console ports; the suite does not supply replacements.

## Representation and limits

The implementation uses purust's existing generic `Value` and function ABI.
A unique array holds `Mutex<Option<Vec<Value>>>`. Consuming operations move its
buffer out and invalidate the old wrapper; borrowing reads it synchronously.
The backend may clone wrapper references, so native checks remain relevant.
Array cloning invokes the element's supplied `Clone` arrow. Copying a generic
`Value` reference does not establish exclusive ownership of its payload.

`Sub` composes reusable functions, and `borrow` passes a temporary cloned
wrapper reference internally. It does not transport a Rust lifetime or a native
`&T` across the PureScript boundary. The public `Shared` constraints and trusted
foreign operations are essential to the contract. The unsafe historical
operations retain their unsafe meaning.

The Sub and UniqueArray operations above are synchronous and pure-facing.
The example owner is an owned
memory buffer, not an IO resource acquired secretly through a pure function.
Counters are test instrumentation. Effectful acquisition, exception-safe custom
protocol completion and asynchronous operations need explicit effectful APIs;
the Aff.Do test exercises one such scoped protocol. This suite does not establish that
all native FFI types or all public API combinations are supported.

The native implementations now live alongside the public modules so application
builds can consume the same files tested here, without importing test fixtures.
They remain specific to purust's current ABI and retain the limits above.
No compiler or production runtime change is required.
