module Test.Data.Assert (assertEqual) where

import Prelude
import Effect (Effect)
import Effect.Exception (throw)

assertEqual :: forall a. Eq a => Show a => String -> a -> a -> Effect Unit
assertEqual label expected actual =
  unless (expected == actual)
    (throw (label <> ": expected " <> show expected <> ", got " <> show actual))
