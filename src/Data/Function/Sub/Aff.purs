module Data.Function.Sub.Aff
  ( SubAff
  , type (-*)
  , Pair
  , identity
  , tensor
  , swap
  , liftShared
  , fromShared
  , runShared
  ) where

import Prelude hiding (identity)

import Data.Function.Sub as Pure
import Data.Function.Sub.Aff.Unsafe as Unsafe
import Data.Tuple (Tuple(..))
import Effect.Aff (Aff)

-- | Opaque representations; construction and general execution are confined
-- | to the explicitly unsafe adapter module.
type SubAff a b = Unsafe.SubAff a b

type Pair a b = Unsafe.Pair a b

infixr 4 type SubAff as -*

-- | An effectful arrow with restricted resource access. Descriptions are reusable;
-- | every execution still needs an appropriate input resource from its runner.
identity :: forall a. a -* a
identity = Unsafe.unsafeFromFunction pure

-- | Run the left branch, then the right branch. An exception or cancellation
-- | can prevent the right branch from running; the enclosing resource scope
-- | must provide cleanup. This operation does not install a generic finalizer.
tensor :: forall a b c d. (a -* b) -> (c -* d) -> Pair a c -* Pair b d
tensor left right = Unsafe.unsafeFromFunction \pair ->
  case Unsafe.unsafeUnpair pair of
    Tuple a c -> do
      b <- Unsafe.unsafeRun left a
      d <- Unsafe.unsafeRun right c
      pure (Unsafe.unsafePair b d)

swap :: forall a b. Pair a b -* Pair b a
swap = Unsafe.unsafeFromFunction \pair ->
  case Unsafe.unsafeUnpair pair of
    Tuple a b -> pure (Unsafe.unsafePair b a)

-- | Ordinary Aff functions may access only values declared safe to share.
-- | This does not expose a resource such as a settlement receipt to a callback.
liftShared :: forall a b. Pure.Shared a => Pure.Shared b => (a -> Aff b) -> a -* b
liftShared = Unsafe.unsafeFromFunction

-- | Embed an existing pure arrow when both observable boundaries are shared.
fromShared :: forall a b. Pure.Shared a => Pure.Shared b => Pure.Sub a b -> a -* b
fromShared arrow = liftShared (pure <<< Pure.runShared arrow)

-- | An unrestricted caller can execute only shared inputs and outputs.
-- | Resource adapters use a trusted, scoped runner instead.
runShared :: forall a b. Pure.Shared a => Pure.Shared b => (a -* b) -> a -> Aff b
runShared = Unsafe.unsafeRun
