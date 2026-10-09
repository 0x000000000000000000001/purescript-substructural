module Test.Data.Function.Sub.Aff.Do (spec) where

import Prelude hiding (bind, discard, pure)
import Prelude as P

import Data.Either (Either(..))
import Data.Function.Sub.Aff.Do (bind, discard, pure)
import Data.Function.Sub.Aff.Do as Do
import Data.Function.Sub.Aff.Unsafe as Unsafe
import Data.Time.Duration (Milliseconds(..))
import Data.Tuple (Tuple(..))
import Effect (Effect)
import Effect.Aff (Aff, attempt, bracket, delay, forkAff, killFiber, throwError)
import Effect.Class (liftEffect)
import Effect.Exception (error, message, throw)
import Effect.Ref (Ref)
import Effect.Ref as Ref
import Test.Data.Assert (assertEqual)

type Audit = { nextId :: Ref Int, events :: Ref (Array String) }

-- Only the private test adapter can construct, inspect or finish a resource.
-- Its runner provides cleanup; the generic do API has no finalizer policy.
newtype Resource :: Type -> Type
newtype Resource scope = Resource
  { id :: Int, open :: Ref Boolean, events :: Ref (Array String) }
type role Resource nominal

newAudit :: Effect Audit
newAudit = P.do
  nextId <- Ref.new 0
  events <- Ref.new []
  P.pure { nextId, events }

record :: Ref (Array String) -> String -> Effect Unit
record events event = Ref.modify_ (\previous -> previous <> [ event ]) events

acquire :: forall scope. Audit -> Aff (Resource scope)
acquire { nextId, events } = liftEffect P.do
  Ref.modify_ (_ + 1) nextId
  id <- Ref.read nextId
  open <- Ref.new true
  record events ("acquire:" <> show id)
  P.pure (Resource { id, open, events })

cleanup :: forall scope. Resource scope -> Aff Unit
cleanup (Resource { id, open, events }) = liftEffect P.do
  isOpen <- Ref.read open
  when isOpen P.do
    Ref.write false open
    record events ("abort:" <> show id)
  record events ("release:" <> show id)

readId :: forall scope. Do.Program (Resource scope) (Resource scope) Int
readId = Do.fromArrow (Unsafe.unsafeFromFunction \resource@(Resource { id, open, events }) -> P.do
  isOpen <- liftEffect (Ref.read open)
  unless isOpen (throwError (error "read after resource consumption"))
  liftEffect (record events ("read:" <> show id))
  P.pure (Unsafe.unsafePair resource id))

addOne :: forall scope. Int -> Do.Program (Resource scope) (Resource scope) Int
addOne value = Do.fromArrow (Unsafe.unsafeFromFunction \resource@(Resource { id, open, events }) -> P.do
  isOpen <- liftEffect (Ref.read open)
  unless isOpen (throwError (error "query after resource consumption"))
  liftEffect (record events ("add:" <> show id <> ":" <> show value))
  P.pure (Unsafe.unsafePair resource (value + 1)))

consume :: forall scope. Do.Program (Resource scope) Unit Unit
consume = Do.fromArrow (Unsafe.unsafeFromFunction \(Resource { id, open, events }) -> liftEffect P.do
  isOpen <- Ref.read open
  unless isOpen (throw "resource consumed twice")
  Ref.write false open
  record events ("consume:" <> show id)
  P.pure (Unsafe.unsafePair unit unit))

runScoped :: forall a. Audit ->
  (forall scope. Do.Program (Resource scope) Unit a) -> Aff a
runScoped audit program = bracket (acquire audit) cleanup \resource -> P.do
  pair <- Unsafe.unsafeRun (Do.toArrow program) resource
  case Unsafe.unsafeUnpair pair of
    Tuple _ value -> P.pure value

checkEvents :: String -> Audit -> Array String -> Aff Unit
checkEvents label { events } expected = P.do
  actual <- liftEffect (Ref.read events)
  liftEffect (assertEqual label expected actual)

qualifiedSpec :: Aff Unit
qualifiedSpec = P.do
  audit@{ events } <- liftEffect newAudit
  actual <- runScoped audit (Do.do
    id <- readId
    seed <- Do.liftAff (P.do
      liftEffect (record events "effect:start")
      delay (Milliseconds 1.0)
      liftEffect (record events "effect:end")
      P.pure (id + 6))
    Do.liftAff (liftEffect (record events "discarded"))
    value <- if seed == 7 then addOne seed
      else Do.liftAff (throwError (error "unselected branch ran") :: Aff Int)
    consume
    Do.pure (value * 3))
  liftEffect (assertEqual "Sub.Aff.Do qualified dependent result" 24 actual)
  checkEvents "Sub.Aff.Do qualified effect order" audit
    [ "acquire:1", "read:1", "effect:start", "effect:end", "discarded"
    , "add:1:7", "consume:1", "release:1"
    ]

-- Importing the constrained bind/pure/discard also supports ordinary do.
-- The surrounding Aff code explicitly uses Prelude's qualified do instead.
ordinaryProgram :: forall scope. Do.Program (Resource scope) Unit Int
ordinaryProgram = do
  id <- readId
  consume
  pure (id + 40)

ordinarySpec :: Aff Unit
ordinarySpec = P.do
  audit <- liftEffect newAudit
  actual <- runScoped audit ordinaryProgram
  liftEffect (assertEqual "Sub.Aff.Do ordinary do result after consumption" 41 actual)
  checkEvents "Sub.Aff.Do ordinary do consumes once" audit
    [ "acquire:1", "read:1", "consume:1", "release:1" ]

replaySpec :: Aff Unit
replaySpec = P.do
  audit <- liftEffect newAudit
  executions <- liftEffect (Ref.new 0)
  let
    program :: forall scope. Do.Program (Resource scope) Unit Int
    program = Do.do
      Do.liftAff (liftEffect (Ref.modify_ (_ + 1) executions))
      id <- readId
      consume
      Do.pure id
    action = runScoped audit program
  checkEvents "Sub.Aff.Do construction does not acquire" audit []
  before <- liftEffect (Ref.read executions)
  liftEffect (assertEqual "Sub.Aff.Do construction does not run lifted Aff" 0 before)
  first <- action
  second <- action
  after <- liftEffect (Ref.read executions)
  liftEffect (assertEqual "Sub.Aff.Do replay receives fresh resources" (Tuple 1 2) (Tuple first second))
  liftEffect (assertEqual "Sub.Aff.Do lifted action runs once per replay" 2 after)
  checkEvents "Sub.Aff.Do replay lifecycle" audit
    [ "acquire:1", "read:1", "consume:1", "release:1"
    , "acquire:2", "read:2", "consume:2", "release:2"
    ]

failureSpec :: Aff Unit
failureSpec = P.do
  audit <- liftEffect newAudit
  failed <- attempt (runScoped audit (Do.do
    readId
    Do.liftAff (throwError (error "expected do failure") :: Aff Unit)
    addOne 99
    consume
    Do.pure unit))
  case failed of
    Left failure -> liftEffect (assertEqual "Sub.Aff.Do preserves failure" "expected do failure" (message failure))
    Right _ -> throwError (error "Sub.Aff.Do unexpectedly recovered from failure")
  checkEvents "Sub.Aff.Do failure skips continuation and runner cleans up" audit
    [ "acquire:1", "read:1", "abort:1", "release:1" ]

awaitReady :: Ref Boolean -> Int -> Aff Unit
awaitReady ready remaining = P.do
  started <- liftEffect (Ref.read ready)
  if started then P.pure unit
  else if remaining == 0 then throwError (error "Sub.Aff.Do cancellation probe did not start")
  else delay (Milliseconds 1.0) *> awaitReady ready (remaining - 1)

cancellationSpec :: Aff Unit
cancellationSpec = P.do
  audit <- liftEffect newAudit
  ready <- liftEffect (Ref.new false)
  -- The signal is set from inside the resource scope. The test therefore
  -- cancels a running program rather than racing cancellation with acquisition.
  bracket
    (forkAff (runScoped audit (Do.do
      readId
      Do.liftAff (liftEffect (Ref.write true ready))
      Do.liftAff (delay (Milliseconds 10000.0))
      addOne 99
      consume
      Do.pure unit)))
    (killFiber (error "release cancellation test fiber"))
    (\fiber -> P.do
      awaitReady ready 100
      killFiber (error "expected do cancellation") fiber
      checkEvents "Sub.Aff.Do cancellation skips continuation and runner cleans up" audit
        [ "acquire:1", "read:1", "abort:1", "release:1" ])

-- Await every assertion and fiber cleanup before the caller's success marker.
spec :: Aff Unit
spec = P.do
  qualifiedSpec
  ordinarySpec
  replaySpec
  failureSpec
  cancellationSpec
