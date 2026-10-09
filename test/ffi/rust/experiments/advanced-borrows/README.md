# Beyond a single borrowed view

This extends the *experiment*, without changing the compiler, backend or earlier
suites. It uses an opaque indexed effect, fresh region/lease tokens, and a native
`Vec<u8>` adapter. The API now supports:

- nested shared read scopes, including access to an ancestor view;
- a native operation returning a shared slice, exposed as a scoped subview;
- an exclusive mutable scope, with temporary mutable sub-borrows;
- two disjoint mutable views obtained through actual Rust `split_at_mut`;
- restoring the parent/root state after a nested callback exception.

```purescript
B.withBuffer "abcd" \buffer -> B.do
  _ <- B.withRead buffer \outer -> B.withRead buffer \inner -> B.do
    parent <- B.parentRead (B.checksum outer)
    tail <- B.tailView inner
    child <- B.checksum tail
    B.pure (parent + child)
  B.withWrite buffer \view ->
    B.withSplit view 2 \left right -> B.do
      B.fill left 65
      B.fill right 66
```

No token or phantom type is written at these call sites. The tradeoff is qualified
`B.do`, scoped combinators, and explicit `parentRead` when using an older read
lease inside a newer one.

## Tested typing boundary

`withRead` pushes `Reading token parent`; only `Ready` or another `Reading` phase
allows this acquisition. `withWrite` needs `Ready` and enters `Writing token`.
Mutable reborrow/split introduce a fresh token, temporarily replacing the parent's
capability. Nominal roles prevent changing the token through `Safe.Coerce`.
Constructors and raw effect runners remain hidden.

A first empirical probe used an instance chain to search automatically for a
read token among ancestors. The compiler rejected a legitimate ancestor access
because rigid tokens partially overlapped the first instance. The final API uses
`parentRead`, which can only lift an action already confined to a shared-reading
phase. It cannot lift mutation or a `Ready` action. This inference limitation is
part of the result, not evidence that ancestor reads are impossible in a library.

| Fixture | Expected result |
| --- | --- |
| `Nested` | Accept nested reads, parent access, and a native returned subview. |
| `Mutable` | Accept mutable reborrow, restored parent, two split views, and later reading. |
| `Unwind` | Accept catching a state-preserving scoped action and resuming parent/root. |
| `Existential` | Accept storing an inert writer behind an existential. |
| `WriteDuringRead`, `ReadDuringWrite` | Reject incompatible acquisitions. |
| `ParentDuringReborrow` | Reject parent access while its child owns the mutable capability. |
| `SiblingDuringReborrow` | Reject sibling use during a child reborrow; a concrete restriction. |
| `MutableEscape`, `SharedChildEscape`, `ReturnedViewEscape` | Reject direct/expired view escape. |
| `DeferredWrite`, `ReviveExistential` | Reject delayed and existential attempts to reactivate old access. |
| `CoerceWrite`, `CoerceAction` | Reject safe coercion of token or action state. |
| `ParentReadMutation` | Reject misuse of the ancestor-read lifting combinator. |
| `ForgeReadable` | Reject an orphan instance making `Writing` readable. |
| `RawRunner` | Reject access to the indexed action constructor. |

Existential storage is still permitted. The tests establish that the stored handle
cannot regain an execution capability through this API, not that no handle can
be stored. The safety contract excludes `unsafeCoerce`, untrusted foreign code,
and modifications to the implementation.

## Actual native representation

The buffer owns `Vec<u8>`. Views carry an `Arc` to a mutex-protected owner,
a range, access kind and lease number. A reader set admits overlapping shared
leases; a writer stack suspends mutable ancestors. Access checks reject inactive
or suspended leases and closed owners. No mutex is held while PureScript callbacks
run, so rejected reentry does not deadlock.

`native_tail :: &[u8] -> &[u8]`, `native_sub_mut`, and `native_split` are ordinary
Rust functions returning actual references. The adapter consumes those results
inside the locked native call, derives range metadata, and returns a handle.
The shared-tail test also checks that its pointer belongs to the original buffer.
There is **no stored Rust reference, unsafe lifetime widening, or transmute**.
Each read/write subsequently obtains a fresh native borrow under the lock.

Thus the bytes are not copied by checksumming, taking a subview, or splitting.
Initial allocation, reference counting, view allocation and locking still cost
work. This is checked zero-copy access through owner/range handles, **not native
Rust lifetimes carried across arbitrary PureScript code**. Only scalar results
and opaque handles cross the boundary.

Split ranges are genuinely disjoint because Rust creates them with `split_at_mut`.
They are manipulated sequentially by this API. A handle itself remains duplicable:
`Mutable` deliberately aliases the left handle before using it. This is safe here
because each operation obtains and releases its native borrow before returning.
It does not give the caller two unrestricted native `&mut` references.

## Executed evidence

Run from this repository root, with the existing optional Rust-test prerequisites:

```sh
node test/ffi/rust/run.mjs --suite advanced-borrows
```

On 2026-10-06 all **18 typing cases** passed (4 accepted, 14 rejected), followed
by actual PureScript → TAST → purust → Rust execution in **normal and threaded**
modes. Cargo runs offline; there are no services or extra dependencies.

`Main` asserts:

- nested/ancestor reads plus the returned tail give checksum aggregate `1873`;
- mutable sub-borrow, split halves and restored parent give `1737`;
- replaying the outer effect gives the same result with a fresh owner;
- exceptions unwind both a child back to its parent and an entire writer back
  to `Ready`; subsequent mutation succeeds and yields `268`;
- all four application buffers have closed before native adversarial checks run.

The explicit native controls bypass PS types and confirm overlapping readers,
writer acquisition refusal during reading, stale-view refusal, parent suspension,
read acquisition refusal during mutable reborrow, disjoint split ranges, parent
restoration, and closed-owner refusal. These dynamic checks are a separate
backstop, not additional static guarantees attributed to PureScript.

Expected final markers are `NATIVE_BACKSTOP_OK ...` and `ADVANCED_BORROWS_OK`.
Detailed diagnostics, tool versions, commands and generated code are under
`test/ffi/rust/artifacts/advanced-borrows/`. The shared harness checks rejection
code and module, then checks executable exit status and the final marker.

## Remaining limits

A mutable reborrow suspends the whole writer phase. Even reborrowing the left
split half temporarily prevents using the disjoint right half; the negative
fixture records this restriction explicitly. Multiple independent buffers in one
region and simultaneously executing mutable operations would need a richer
capability layout and adapter contract. The current native mutex serializes calls.

Callbacks are synchronous. `attempt` catches only state-preserving indexed actions;
there is no arbitrary lift/unlift of `Effect` and no async cancellation contract.
RAII releases/restores leases during the tested PureScript exception unwinds.
Cleanup does not imply transaction-style rollback of prior byte mutations.

This is a hand-written library API, not automatic import of arbitrary Rust
lifetime relationships. The experiment goes beyond the earlier `borrowed-views`
suite, while keeping the distinction between indexed access permissions and
Rust's own references explicit.
