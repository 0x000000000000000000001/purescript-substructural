module Test.Data.Function.Sub.Aff (spec) where

import Prelude

import Data.Either (Either(..))
import Data.Function.Sub as Pure
import Data.Function.Sub.Aff as Sub
import Data.Function.Sub.Aff.Unsafe as Unsafe
import Data.Tuple (Tuple(..))
import Data.Time.Duration (Milliseconds(..))
import Effect (Effect)
import Effect.Aff (Aff, attempt, delay, error, throwError)
import Effect.Class (liftEffect)
import Effect.Exception (message)
import Test.Data.Assert (assertEqual)

-- Only these test primitives may construct or inspect an owned resource.
foreign import data Resource :: Type
foreign import reset :: Effect Unit
foreign import acquire :: String -> Int -> Effect Resource
foreign import consume :: Resource -> Effect Int
foreign import record :: String -> Effect Unit
foreign import events :: Effect (Array String)
foreign import assertAllConsumed :: Effect Unit

acquireOne :: Sub.SubAff Unit Resource
acquireOne = Unsafe.unsafeFromFunction \_ -> liftEffect (acquire "one" 7)

consumeOne :: Sub.SubAff Resource Int
consumeOne = Unsafe.unsafeFromFunction (liftEffect <<< consume)

acquireTwo :: Sub.SubAff Unit (Sub.Pair Resource Resource)
acquireTwo = Unsafe.unsafeFromFunction \_ -> do
  left <- liftEffect (acquire "left" 5)
  right <- liftEffect (acquire "right" 8)
  pure (Unsafe.unsafePair left right)

subtractPair :: Sub.SubAff (Sub.Pair Int Int) Int
subtractPair = Unsafe.unsafeFromFunction \pair ->
  case Unsafe.unsafeUnpair pair of
    Tuple left right -> pure (left - right)

checkEvents :: Array String -> Aff Unit
checkEvents expected = do
  actual <- liftEffect events
  liftEffect (assertEqual "Sub.Aff effect order" expected actual)

spec :: Aff Unit
spec = do
  identityResult <- Sub.runShared Sub.identity 9
  liftEffect (assertEqual "Sub.Aff identity" 9 identityResult)
  lifted <- Sub.runShared (Sub.fromShared (Pure.liftShared (_ + 1))) 4
  liftEffect (assertEqual "Sub.Aff fromShared" 5 lifted)
  liftEffect reset
  composed <- Sub.runShared
    (Sub.liftShared (\n -> do
      liftEffect (record "first-start")
      delay (Milliseconds 1.0)
      liftEffect (record "first-end")
      pure (n + 1)) >>>
      Sub.liftShared (\n -> liftEffect (record "second") $> (n * 3))) 6
  liftEffect (assertEqual "Sub.Aff compose result" 21 composed)
  checkEvents [ "first-start", "first-end", "second" ]

  liftEffect reset
  let action = Sub.runShared (acquireOne >>> consumeOne) unit
  checkEvents []
  first <- action
  second <- action
  liftEffect (assertEqual "Sub.Aff replay result" (Tuple 7 7) (Tuple first second))
  checkEvents [ "acquire:one:1", "consume:one:1", "acquire:one:2", "consume:one:2" ]
  liftEffect assertAllConsumed

  liftEffect reset
  tensorResult <- Sub.runShared
    (acquireTwo >>> Sub.tensor consumeOne consumeOne >>> subtractPair) unit
  liftEffect (assertEqual "Sub.Aff tensor result" (-3) tensorResult)
  checkEvents [ "acquire:left:1", "acquire:right:2", "consume:left:1", "consume:right:2" ]
  liftEffect assertAllConsumed

  liftEffect reset
  swapped <- Sub.runShared
    (acquireTwo >>> Sub.swap >>> Sub.tensor consumeOne consumeOne >>> subtractPair) unit
  liftEffect (assertEqual "Sub.Aff swap result" 3 swapped)
  checkEvents [ "acquire:left:1", "acquire:right:2", "consume:right:2", "consume:left:1" ]
  liftEffect assertAllConsumed

  liftEffect reset
  let
    pairInput :: Sub.SubAff Unit (Sub.Pair Int Int)
    pairInput = Unsafe.unsafeFromFunction \_ -> pure (Unsafe.unsafePair 2 3)
    fails :: Sub.SubAff Int Int
    fails = Sub.liftShared \_ -> do
      liftEffect (record "left-fails")
      throwError (error "expected tensor failure")
    right :: Sub.SubAff Int Int
    right = Sub.liftShared \n -> liftEffect (record "right-must-not-run") $> n
    after :: Sub.SubAff (Sub.Pair Int Int) Unit
    after = Unsafe.unsafeFromFunction \_ -> liftEffect (record "after-must-not-run")
  failed <- attempt (Sub.runShared (pairInput >>> Sub.tensor fails right >>> after) unit)
  case failed of
    Left failure -> liftEffect (assertEqual "Sub.Aff original failure" "expected tensor failure" (message failure))
    Right _ -> throwError (error "Sub.Aff tensor unexpectedly succeeded")
  checkEvents [ "left-fails" ]
