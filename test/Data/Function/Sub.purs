module Test.Data.Function.Sub
  ( spec
  ) where

import Data.Function.Sub (class Drop, type (-*), clone, drop, fst', liftShared, runShared, snd')
import Data.Tuple (Tuple(..))
import Prelude
import Effect (Effect)
import Test.Data.Assert (assertEqual)

-- Test-only native values. The FFI assertion executes the real class methods
-- inside Effect, keeping the instrumented destruction out of pure test code.
foreign import data Tracked :: Type
foreign import trackedDrop :: Tracked -* Unit

instance dropTracked :: Drop Tracked where
  drop = trackedDrop

foreign import assertDrops
  :: (Tuple Tracked Tracked -* Unit)
  -> (Tuple Tracked Int -* Int)
  -> (Tuple Int Tracked -* Int)
  -> (forall a b. a -> b -> Tuple a b)
  -> Effect Unit

spec :: Effect Unit
spec = do
  assertEqual "Sub.fst'" 1 (runShared (clone >>> fst') 1)
  assertEqual "Sub.snd'" 1 (runShared (clone >>> snd') 1)
  assertEqual "Sub.identity" 9 (runShared identity 9)
  assertEqual "Sub.compose order" 21
    (runShared (liftShared (_ + 1) >>> liftShared (_ * 3)) 6)
  assertEqual "Sub.clone tuple" (Tuple (Tuple 2 3) (Tuple 2 3))
    (runShared clone (Tuple 2 3))
  assertDrops drop snd' fst' Tuple
