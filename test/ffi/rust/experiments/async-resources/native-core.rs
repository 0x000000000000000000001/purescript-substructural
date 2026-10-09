use std::future::Future;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome { Success(i64), Failed, Cancelled }
#[derive(Clone, Copy, PartialEq)]
enum Decision { Waiting, Success, Failure, Cancel }

// Neither Clone nor Copy: the pending operation reads these actual bytes.
struct Session { bytes: Vec<u8>, drops: Arc<AtomicUsize> }
impl Drop for Session {
    fn drop(&mut self) { self.drops.fetch_add(1, Ordering::SeqCst); }
}
struct State {
    session: Option<Session>, acquired: usize, released: usize,
    registered: bool, started: bool, stopped: bool,
    callback_attempted: bool, cancelled: bool, decision: Decision,
    operation_waker: Option<Waker>, watchers: [Vec<Waker>; 3],
    future_drops: usize, delivered: usize, suppressed: usize,
    duplicate_callbacks: usize, cancel_requests: usize, continued: usize,
}
pub struct Control { seed: u8, drops: Arc<AtomicUsize>, state: Mutex<State> }
#[derive(Debug)]
pub struct Snapshot {
    pub acquired: usize, pub released: usize, pub dropped: usize,
    pub future_drops: usize, pub delivered: usize, pub suppressed: usize,
    pub duplicate_callbacks: usize, pub cancel_requests: usize, pub continued: usize,
}
fn wake(wakers: Vec<Waker>) { for waker in wakers { waker.wake(); } }

impl Control {
    pub fn new(seed: u8) -> Arc<Self> {
        Arc::new(Self { seed, drops: Arc::new(AtomicUsize::new(0)), state: Mutex::new(State {
            session: None, acquired: 0, released: 0, registered: false,
            started: false, stopped: false, callback_attempted: false, cancelled: false,
            decision: Decision::Waiting, operation_waker: None,
            watchers: [Vec::new(), Vec::new(), Vec::new()], future_drops: 0,
            delivered: 0, suppressed: 0, duplicate_callbacks: 0,
            cancel_requests: 0, continued: 0,
        }) })
    }
    pub fn acquire(&self) -> Result<(), &'static str> {
        let mut state = self.state.lock().unwrap();
        if state.acquired != 0 { return Err("control already acquired"); }
        state.session = Some(Session {
            bytes: vec![self.seed, self.seed + 1, self.seed + 2], drops: self.drops.clone(),
        });
        state.acquired += 1;
        Ok(())
    }
    pub fn release(&self) -> Result<(), &'static str> {
        let session = {
            let mut state = self.state.lock().unwrap();
            if state.registered && !state.stopped { return Err("native operation still active"); }
            let session = state.session.take().ok_or("resource already released")?;
            state.released += 1;
            session
        };
        drop(session);
        Ok(())
    }
    pub fn complete(&self, fail: bool) -> bool {
        let waker = {
            let mut state = self.state.lock().unwrap();
            if state.cancelled || state.decision != Decision::Waiting || state.session.is_none() { return false; }
            state.decision = if fail { Decision::Failure } else { Decision::Success };
            state.operation_waker.take()
        };
        if let Some(waker) = waker { waker.wake(); }
        true
    }
    pub fn cancel(&self) {
        let waker = {
            let mut state = self.state.lock().unwrap();
            state.cancel_requests += 1;
            state.cancelled = true;
            if state.decision == Decision::Waiting { state.decision = Decision::Cancel; }
            state.operation_waker.take()
        };
        if let Some(waker) = waker { waker.wake(); }
    }
    // Completion can arrive after cancellation/release. Only inert state is
    // inspected here; the callback never reads the released resource bytes.
    pub fn deliver(&self, outcome: Outcome) -> bool {
        let (accepted, wakers) = {
            let mut state = self.state.lock().unwrap();
            if state.callback_attempted { state.duplicate_callbacks += 1; return false; }
            state.callback_attempted = true;
            let accepted = !state.cancelled && outcome != Outcome::Cancelled;
            if accepted { state.delivered += 1; } else { state.suppressed += 1; }
            (accepted, std::mem::take(&mut state.watchers[2]))
        };
        wake(wakers);
        accepted
    }
    pub fn continued(&self) { self.state.lock().unwrap().continued += 1; }
    pub fn snapshot(&self) -> Snapshot {
        let s = self.state.lock().unwrap();
        Snapshot { acquired: s.acquired, released: s.released,
            dropped: self.drops.load(Ordering::SeqCst), future_drops: s.future_drops,
            delivered: s.delivered, suppressed: s.suppressed, duplicate_callbacks: s.duplicate_callbacks,
            cancel_requests: s.cancel_requests, continued: s.continued }
    }
}

pub struct Operation { control: Arc<Control>, _pin: PhantomPinned }
impl Operation {
    pub fn new(control: Arc<Control>) -> Result<Self, &'static str> {
        {
            let mut state = control.state.lock().unwrap();
            if state.session.is_none() { return Err("resource already released"); }
            if state.registered { return Err("operation already started"); }
            state.registered = true;
        }
        Ok(Self { control, _pin: PhantomPinned })
    }
}
impl Future for Operation {
    type Output = Outcome;
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Outcome> {
        let (result, wakers) = {
            let mut s = self.as_ref().get_ref().control.state.lock().unwrap();
            let wakers = if !s.started {
                s.started = true;
                std::mem::take(&mut s.watchers[0])
            } else { Vec::new() };
            let result = match s.decision {
                Decision::Waiting => { s.operation_waker = Some(context.waker().clone()); Poll::Pending },
                Decision::Success => Poll::Ready(Outcome::Success(s.session.as_ref()
                    .expect("release waits for future drop").bytes.iter().map(|b| i64::from(*b)).sum())),
                Decision::Failure => Poll::Ready(Outcome::Failed),
                Decision::Cancel => Poll::Ready(Outcome::Cancelled),
            };
            (result, wakers)
        };
        wake(wakers);
        result
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        let wakers = {
            let mut s = self.control.state.lock().unwrap();
            s.stopped = true;
            s.future_drops += 1;
            std::mem::take(&mut s.watchers[1])
        };
        wake(wakers);
    }
}
pub struct Event { pub control: Arc<Control>, pub kind: usize }
impl Future for Event {
    type Output = ();
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        let mut s = self.control.state.lock().unwrap();
        let ready = match self.kind {
            0 => s.started, 1 => !s.registered || s.stopped, 2 => s.callback_attempted,
            _ => panic!("unknown event"),
        };
        if ready { Poll::Ready(()) } else {
            let watchers = &mut s.watchers[self.kind];
            if !watchers.iter().any(|w| w.will_wake(context.waker())) { watchers.push(context.waker().clone()); }
            Poll::Pending
        }
    }
}
