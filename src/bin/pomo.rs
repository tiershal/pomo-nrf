#![no_main]
#![no_std]

use pomo_nrf as _;

use pomo_nrf::hal;

#[rtic::app(device = nrf52840_hal::pac, dispatchers = [SWI0_EGU0])]
mod app {
    use super::*;

    use fugit::ExtU64;
    use sfsm::{
        add_state_machine, IsState, SfsmError, State, StateMachine, TransitGuard, Transition,
    };

    use hal::Clocks;
    use rtic_monotonics::nrf::rtc::Rtc0;

    #[shared]
    struct Shared {
        state_machine: PomoStateMachine,
    }

    #[local]
    struct Local {}

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        defmt::println!("init");

        Clocks::new(ctx.device.CLOCK).start_lfclk();

        let token = rtic_monotonics::create_nrf_rtc0_monotonic_token!();
        Rtc0::start(ctx.device.RTC0, token);

        let mut state_machine = PomoStateMachine::new();
        state_machine.start(Running { remaining: 25 }).unwrap();

        on_tick::spawn().ok();

        (Shared { state_machine }, Local {})
    }

    #[task(shared = [state_machine])]
    async fn on_tick(mut ctx: on_tick::Context) {
        loop {
            ctx.shared.state_machine.lock(|sm| {
                sm.step().unwrap();
            });

            Rtc0::delay(1000u64.millis()).await;
        }
    }

    add_state_machine!(pub PomoStateMachine, Running, [Running, InBetween], [
        Running => InBetween,
        InBetween => Running,
    ]);

    pub struct Running {
        remaining: u32,
    }

    pub struct InBetween {
        remaining: u32,
    }

    impl State for Running {
        fn entry(&mut self) {
            defmt::println!("entering Running");
        }

        fn execute(&mut self) {
            self.remaining = self.remaining.saturating_sub(1);
            defmt::println!("seconds remaining: {}", self.remaining);
        }
    }

    impl Transition<InBetween> for Running {
        fn guard(&self) -> TransitGuard {
            match self.remaining {
                0 => TransitGuard::Transit,
                _ => TransitGuard::Remain,
            }
        }
    }

    impl Into<InBetween> for Running {
        fn into(self) -> InBetween {
            InBetween { remaining: 10 }
        }
    }

    impl State for InBetween {
        fn entry(&mut self) {
            defmt::println!("entering InBetween");
        }

        fn execute(&mut self) {
            self.remaining = self.remaining.saturating_sub(1);
            defmt::println!("seconds remaining: {}", self.remaining);
        }
    }

    impl Transition<Running> for InBetween {
        fn guard(&self) -> TransitGuard {
            match self.remaining {
                0 => TransitGuard::Transit,
                _ => TransitGuard::Remain,
            }
        }
    }

    impl Into<Running> for InBetween {
        fn into(self) -> Running {
            Running { remaining: 25 }
        }
    }
}
