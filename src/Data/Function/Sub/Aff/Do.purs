-- | Indexed, effectful programs with hidden resources and shared results.
-- | Use qualified `Do.do`, or import `bind` and `discard` unqualified to
-- | rebind ordinary `do`. Resource-specific adapters still own finalization.
module Data.Function.Sub.Aff.Do
  ( Program
  , ResourceProgram
  , fromArrow
  , toArrow
  , pure
  , bind
  , discard
  , liftAff
  ) where

import Prelude hiding (bind, discard, pure)
import Prelude as P

import Data.Function.Sub (class Shared)
import Data.Function.Sub.Aff (Pair, SubAff)
import Data.Function.Sub.Aff.Unsafe as Unsafe
import Data.Tuple (Tuple(..))
import Effect.Aff (Aff)

-- | A step receives `before`, leaves `after`, and produces a result `a`.
-- | The resource stays in the opaque product. Only results declared `Shared`
-- | may reach ordinary callbacks, which can duplicate or discard their inputs.
-- | The constructor is private; nominal roles protect all three boundaries.
newtype Program before after a = Program (SubAff before (Pair after a))
type role Program nominal nominal nominal

-- | A program whose resource type does not change.
type ResourceProgram resource a = Program resource resource a

-- | Adapt an arrow whose result is safe to expose to an ordinary continuation.
fromArrow :: forall before after a. Shared a =>
  SubAff before (Pair after a) -> Program before after a
fromArrow = Program

-- | Recover the arrow without exposing either component of its output product.
-- | Resource-specific scoped runners handle execution and finalization.
toArrow :: forall before after a.
  Program before after a -> SubAff before (Pair after a)
toArrow (Program arrow) = arrow

-- | Return a shared result while preserving the resource.
pure :: forall state a. Shared a => a -> Program state state a
pure value = Program (Unsafe.unsafeFromFunction \resource ->
  P.pure (Unsafe.unsafePair resource value))

-- | Run an ordinary Aff action without exposing the hidden resource to it.
-- | The action is deferred until execution and runs again on each execution.
liftAff :: forall state a. Shared a => Aff a -> Program state state a
liftAff action = Program (Unsafe.unsafeFromFunction \resource -> P.do
  value <- action
  P.pure (Unsafe.unsafePair resource value))

-- | Pass the shared result to the continuation and the remaining resource to
-- | its program. The intermediate resource types must agree. An exception or
-- | cancellation stops sequencing; this operation installs no finalizer.
bind :: forall before middle after a b. Shared a =>
  Program before middle a ->
  (a -> Program middle after b) ->
  Program before after b
bind (Program first) next = Program (Unsafe.unsafeFromFunction \resource -> P.do
  pair <- Unsafe.unsafeRun first resource
  case Unsafe.unsafeUnpair pair of
    Tuple remaining value -> case next value of
      Program second -> Unsafe.unsafeRun second remaining)

-- | The other entry point used by do notation. Shared is intentionally stricter
-- | than Drop: neither entry point may expose a non-shared value to a callback.
discard :: forall before middle after a b. Shared a =>
  Program before middle a ->
  (a -> Program middle after b) ->
  Program before after b
discard = bind

-- There is no standard Bind/Monad instance: its unconstrained methods cannot
-- require Shared, and its bind cannot express these resource transitions.
-- Qualified or rebound do uses the functions above directly.
--
-- Shared instances and the Unsafe implementations behind supplied arrows are
-- trusted. This API does not infer aliasing or native lifetimes, and does not
-- make arbitrary Foreign values or functions safe to share.
