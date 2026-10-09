# Generic Rust iterators and trait objects through a typed effect API

This experiment tests actual Rust shapes without changing the compiler or
backend, or introducing another arrow DSL. The public PureScript API is:

```purescript
wide  :: String -> Effect Cursor
small :: String -> Effect Cursor
next  :: Cursor -> Effect (Either String (Maybe Int))
close :: Cursor -> Effect Unit
```

`Effect` is deliberate: advancing or closing a stateful iterator is observable.
There is no mutation or resource cleanup hidden behind a supposedly pure call.

## Native objects exercised

`Cursor.rs` has all of the following, used by the compiled PureScript executable:

- a genuine `Box<dyn CursorOps + Send>` stored behind an opaque handle;
- `IteratorCursor<I>` implementing that trait for generic iterators;
- `boxed<I>` converting a concrete iterator into the trait object;
- `parsed<T>` returning `impl Iterator<Item = Result<i64, CursorError>>`;
- an ordinary generic `parse<T: FromStr>`, instantiated for `i32` and `u8`;
- a non-`Clone`, non-`Copy` `Words` resource captured by the iterator closure,
  with `Drop` counters and an `&mut self` method ensuring whole-resource capture.

The wide parser uses `i32` to stay within PureScript `Int` range, then widens
to the backend's `i64` argument representation.

This is not a mock iterator or a boolean standing in for a native result.
`CursorOps::pull` calls `Iterator::next` and `Option::transpose`, giving a real
`Result<Option<i64>, CursorError>`. The private adapter then selects actual
PureScript constructors:

| Rust result | PureScript result |
| --- | --- |
| `Ok(Some(value))` | `Right (Just value)` |
| `Ok(None)` | `Right Nothing` |
| `Err(error)` | `Left` containing the error text |

The PureScript module supplies `Left`, `Right Nothing`, and the `Right <<< Just`
constructor function. Native code chooses the branch and passes the payload.
Parsing errors retain both the offending input and Rust's real `ParseIntError`
message. The public API intentionally flattens that error into a string; it does
not preserve Rust's error type or identity.

## Consumption policy and executed checks

Handles and `Effect` values remain unrestricted PureScript values. Aliasing a
handle or replaying `next cursor` is accepted and advances the **same** iterator.
The wrapper serializes access with a mutex and owns the boxed iterator in an
`Option`. On exhaustion, parse failure, or explicit close, it takes and drops that
owner exactly once. Later access returns `Left "cursor closed"`. Repeated close
is harmless. Closing at exhaustion is this wrapper's policy, not a universal
property of Rust iterators.

`Main` asserts complete ADT values, including successful numbers, `Nothing`,
and error contents. Its intermediate counters prove retention and release:

1. `wide "7,11"` returns 7 and 11 through an aliased/replayed action, then
   `Right Nothing`; exhaustion drops the captured resource once.
2. An empty iterator returns `Right Nothing` and releases its resource.
3. `wide "5,not-a-number,9"` returns 5, then the real invalid-digit error with
   `not-a-number` in its payload; the terminal error destroys the iterator before
   9 is visited. A retry returns the distinct closed error.
4. `small "255,256,1"` returns 255, then the real `u8` overflow error for 256.
   The same generic code instantiated as `i32` supports the other examples.
5. Closing an unused iterator twice drops once and reads no items.
6. A function returns a deferred action capturing a cursor for `-8`. The action
   retains the resource after the function returns, then releases it on exhaustion.

Final counters are **6 resources created, 6 dropped, 7 items read**. Resource
release occurs before invoking the constructor translating the terminal result.
No assertion relies on a supposed PureScript last-use or garbage-collection point.

## Reproduce

With the repository's optional native-test prerequisites installed:

```sh
node test/ffi/rust/run.mjs --suite native-shapes
```

Validation on 2026-10-06: all **6 typing cases** passed (3 accepted, 3 rejected),
then actual PureScript → TAST → purust → Rust execution passed in **normal and
threaded** modes. Cargo runs offline and no service or extra dependency is used.
Expected marker: `NATIVE_SHAPES_OK`. Detailed evidence is written to
`test/ffi/rust/artifacts/native-shapes/`.

The acceptance controls explicitly preserve aliasing/replay and escaping effects.
The negative controls reject safe coercion into an array, access to the private
raw translator, and constructing a forged `Cursor`. Importing `Cursor(..)` alone
is allowed for foreign data, so the constructor test actually attempts to build
`Cursor`; it does not mistake an import convention for a safety guarantee.

## Scope of this result

This demonstrates that these Rust forms can remain entirely native behind a
small library adapter. It does **not** automatically derive trait implementations
or generic instantiations from arbitrary PureScript types. `wide` and `small`
are two explicit instantiations; arbitrary associated types, borrowed trait-object
lifetimes, pinning and async iterators are not covered here.

The ownership policy is dynamically enforced. These tests do not reject duplicate
handles, replay, escaped effects, or forgotten explicit close at compile time.
Both backend modes are executed, but there is no simultaneous race probe and no
async cancellation contract. The adapter is trusted, as is any FFI implementation.
