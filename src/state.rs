use fugit::MillisDurationU64;

use sfsm::{
    add_messages, add_state_machine, IsState, MessageError, PushMessage, ReceiveMessage, SfsmError,
    State, StateMachine, TransitGuard, Transition,
};

pub const TIME_RUNNING_MSECS: MillisDurationU64 = MillisDurationU64::secs(25);
pub const TIME_INBETWEEN_MSECS: MillisDurationU64 = MillisDurationU64::secs(10);
pub const TIME_INTERVAL_MSECS: MillisDurationU64 = MillisDurationU64::millis(100);

pub type StateDuration = MillisDurationU64;

add_state_machine!(pub PomoStateMachine, Running, [Running, InBetween, Paused], [
    Running => InBetween,
    InBetween => Running,
    Running => Paused,
    Paused => Running,
]);

add_messages!(PomoStateMachine, [
    DoPause -> Running,
    DoResume -> Paused,
]);

// -- Messages
pub struct DoPause;
pub struct DoResume;
// -- Messages

pub struct Running {
    remaining: StateDuration,
    do_pause: bool,
}

pub struct InBetween {
    remaining: StateDuration,
}

impl ReceiveMessage<DoPause> for Running {
    fn receive_message(&mut self, _message: DoPause) {
        self.do_pause = true;
    }
}

impl Running {
    pub fn new(remaining: StateDuration) -> Self {
        Self {
            remaining,
            do_pause: false,
        }
    }
}

impl State for Running {
    fn entry(&mut self) {
        defmt::println!("entering Running");
    }

    fn execute(&mut self) {
        if !self.do_pause {
            self.remaining -= TIME_INTERVAL_MSECS;
        }
    }
}

impl Transition<InBetween> for Running {
    fn guard(&self) -> TransitGuard {
        self.remaining.is_zero().into()
    }
}

impl Transition<Paused> for Running {
    fn guard(&self) -> TransitGuard {
        self.do_pause.into()
    }
}

pub struct Paused {
    remaining: StateDuration,
    do_resume: bool,
}

impl ReceiveMessage<DoResume> for Paused {
    fn receive_message(&mut self, _message: DoResume) {
        self.do_resume = true;
    }
}

impl State for Paused {
    fn entry(&mut self) {
        defmt::println!("entering Paused state");
    }
}

impl Transition<Running> for Paused {
    fn guard(&self) -> TransitGuard {
        self.do_resume.into()
    }
}

impl Into<Running> for Paused {
    fn into(self) -> Running {
        Running {
            remaining: self.remaining,
            do_pause: false,
        }
    }
}

impl Into<Paused> for Running {
    fn into(self) -> Paused {
        Paused {
            remaining: self.remaining,
            do_resume: false,
        }
    }
}

impl Into<InBetween> for Running {
    fn into(self) -> InBetween {
        InBetween {
            remaining: TIME_INBETWEEN_MSECS,
        }
    }
}

impl State for InBetween {
    fn entry(&mut self) {
        defmt::println!("entering InBetween");
    }

    fn execute(&mut self) {
        self.remaining -= TIME_INTERVAL_MSECS;
    }
}

impl Transition<Running> for InBetween {
    fn guard(&self) -> TransitGuard {
        self.remaining.is_zero().into()
    }
}

impl Into<Running> for InBetween {
    fn into(self) -> Running {
        Running {
            remaining: TIME_RUNNING_MSECS,
            do_pause: false,
        }
    }
}
