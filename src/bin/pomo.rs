#![no_main]
#![no_std]

use pomo_nrf as _;

#[rtic::app(device = nrf52840_hal::pac, dispatchers = [SWI0_EGU0])]
mod app {

    use nrf52840_hal as hal;

    use fugit::ExtU64;
    use sfsm::{
        add_state_machine, IsState, SfsmError, State, StateMachine, TransitGuard, Transition,
    };

    use hal::{gpiote::Gpiote, Clocks};
    use rtic_monotonics::nrf::rtc::Rtc0;

    const TIME_RUNNING_SECS: u32 = 25;
    const TIME_INBETWEEN_SECS: u32 = 10;

    #[shared]
    struct Shared {
        state_machine: PomoStateMachine,
        gpiote: Gpiote,
    }

    #[local]
    struct Local {}

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        Clocks::new(ctx.device.CLOCK).start_lfclk();

        let token = rtic_monotonics::create_nrf_rtc0_monotonic_token!();
        Rtc0::start(ctx.device.RTC0, token);

        let mut state_machine = PomoStateMachine::new();
        state_machine
            .start(Running {
                remaining: TIME_RUNNING_SECS,
            })
            .unwrap();

        let p0 = hal::gpio::p0::Parts::new(ctx.device.P0);
        let button = p0.p0_11.into_pullup_input().degrade();

        let gpiote = Gpiote::new(ctx.device.GPIOTE);
        gpiote
            .channel0()
            .input_pin(&button)
            .hi_to_lo()
            .enable_interrupt();
        gpiote.channel0().set();

        on_tick::spawn().ok();

        (
            Shared {
                state_machine,
                gpiote,
            },
            Local {},
        )
    }

    #[task(shared = [state_machine], priority = 1)]
    async fn on_tick(mut ctx: on_tick::Context) {
        loop {
            ctx.shared.state_machine.lock(|sm| {
                sm.step().unwrap();
            });

            Rtc0::delay(1000u64.millis()).await;
        }
    }

    #[task(binds = GPIOTE, shared = [state_machine, gpiote], priority = 3)]
    fn on_button(mut ctx: on_button::Context) {
        ctx.shared.gpiote.lock(|gpiote| {
            gpiote.channel0().event().reset();
        });

        defmt::println!("pressed");
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
            InBetween {
                remaining: TIME_INBETWEEN_SECS,
            }
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
            Running {
                remaining: TIME_RUNNING_SECS,
            }
        }
    }
}
