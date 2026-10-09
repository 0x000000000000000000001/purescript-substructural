module Test.Main
  ( main
  ) where

import Effect (Effect)
import Effect.Aff (launchAff_)
import Effect.Class (liftEffect)
import Effect.Console (log)
import Prelude
import Test.Data.Array.Unique as Data.Array.Unique
import Test.Data.Function.Sub as Data.Function.Sub
import Test.Data.Function.Sub.Aff as Data.Function.Sub.Aff
import Test.Data.Function.Sub.Aff.Do as Data.Function.Sub.Aff.Do

main :: Effect Unit
main = do
  Data.Array.Unique.spec
  Data.Function.Sub.spec
  launchAff_ do
    Data.Function.Sub.Aff.spec
    Data.Function.Sub.Aff.Do.spec
    liftEffect (log "SUBSTRUCTURAL_JS_OK")
