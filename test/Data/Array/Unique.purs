module Test.Data.Array.Unique
  ( spec
  ) where

import Data.Array.Unique (UniqueArray, empty, fromShared, isEmpty, length, reverse, singleton, snoc, toShared)
import Data.Function.Sub (type (-*), borrow, clone, fst', runShared, snd')
import Data.Tuple (Tuple, fst, snd)
import Prelude
import Effect (Effect)
import Test.Data.Assert (assertEqual)

-- Test-only observer: ordinary user code cannot execute these resource arrows
-- or obtain UniqueArray values through runShared's Shared output constraint.
foreign import assertCloneIndependence
  :: (UniqueArray Int -* Tuple (UniqueArray Int) (UniqueArray Int))
  -> (UniqueArray Int -* UniqueArray Int)
  -> (forall a b. Tuple a b -> a)
  -> (forall a b. Tuple a b -> b)
  -> Effect Unit

spec :: Effect Unit
spec = do
  assertEqual "UniqueArray.empty" ([] :: Array Int)
    (runShared (empty >>> toShared) unit)

  assertEqual "UniqueArray.singleton" [1]
    (runShared (singleton >>> toShared) 1)

  assertEqual "UniqueArray.snoc" [2, 1]
    (runShared (singleton >>> borrow length >>> snoc >>> toShared) 2)

  let snd'' = snd' :: Tuple (UniqueArray Int) Boolean -* Boolean
  assertEqual "UniqueArray.isEmpty true" true
    (runShared (empty >>> borrow isEmpty >>> snd'') unit)
  assertEqual "UniqueArray.isEmpty false" false
    (runShared (singleton >>> borrow isEmpty >>> snd') 1)

  assertEqual "UniqueArray.reverse" [3, 2, 1]
    (runShared (fromShared >>> reverse >>> reverse >>> reverse >>> toShared) [1, 2, 3])

  let input = [1, 2, 3]
  assertEqual "UniqueArray.borrow preserves ownership" [3, 2, 1]
    (runShared (fromShared >>> borrow length >>> fst' >>> reverse >>> toShared) input)
  assertEqual "UniqueArray.fromShared preserves original" [1, 2, 3] input

  assertCloneIndependence clone reverse fst snd
