module Data.Function.Sub.Aff.Unsafe
  ( SubAff
  , Pair
  , unsafeFromFunction
  , unsafeRun
  , unsafePair
  , unsafeUnpair
  ) where

import Prelude

import Data.Tuple (Tuple(..))
import Effect.Aff (Aff)

-- | Trusted adapter representation. Ordinary clients should import
-- | `Data.Function.Sub.Aff` instead of constructing or executing primitives here.
newtype SubAff a b = SubAff (a -> Aff b)

type role SubAff nominal nominal

-- | Product whose components are only accessible to trusted primitives.
data Pair a b = Pair a b

type role Pair nominal nominal

instance semigroupoidSubAff :: Semigroupoid SubAff where
  compose (SubAff after) (SubAff before) =
    SubAff \value -> before value >>= after

instance categorySubAff :: Category SubAff where
  identity = SubAff pure

-- | The adapter author must preserve the resource protocol: no hidden copying,
-- | escaping aliases, replay of consuming actions, or unaccounted resource loss.
-- | Resource-specific exception/cancellation cleanup is also their responsibility.
unsafeFromFunction :: forall a b. (a -> Aff b) -> SubAff a b
unsafeFromFunction = SubAff

-- | Bypasses the shared-input/output boundary. Use only inside a trusted runner
-- | that owns its input and implements the required acquisition/finalization.
unsafeRun :: forall a b. SubAff a b -> a -> Aff b
unsafeRun (SubAff run) = run

-- | The caller is responsible for transferring both input capabilities.
unsafePair :: forall a b. a -> b -> Pair a b
unsafePair = Pair

-- | Exposes unrestricted aliases to the components. Only trusted primitives may
-- | use them while preserving the enclosing resource protocol.
unsafeUnpair :: forall a b. Pair a b -> Tuple a b
unsafeUnpair (Pair left right) = Tuple left right
