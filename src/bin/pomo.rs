#![no_main]
#![no_std]

use pomo_nrf as _;

#[rtic::app(device = nrf52840_hal::pac, dispatchers = [SWI0_EGU0])]
mod app {

    use nrf52840_hal as hal;

    use fugit::ExtU64;
    use sfsm::{
        add_messages, add_state_machine, IsState, MessageError, PushMessage, ReceiveMessage,
        SfsmError, State, StateMachine, TransitGuard, Transition,
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
                do_pause: false,
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

        ctx.shared.state_machine.lock(|sm| {
            // Since the state machine can only be in one state at any given time, we could just fire all possible messages for this button
            // and the state we're in can handle the message safely.
            //
            // We don't need to care about the result of the message being sent.
            _ = PushMessage::<Running, DoPause>::push_message(sm, DoPause);
            _ = PushMessage::<Paused, DoResume>::push_message(sm, DoResume);
        });
    }

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
    struct DoPause;
    struct DoResume;
    // -- Messages

    pub struct Running {
        remaining: u32,
        do_pause: bool,
    }

    pub struct InBetween {
        remaining: u32,
    }

    impl ReceiveMessage<DoPause> for Running {
        fn receive_message(&mut self, _message: DoPause) {
            self.do_pause = true;
        }
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

    impl Transition<Paused> for Running {
        fn guard(&self) -> TransitGuard {
            self.do_pause.into()
        }
    }

    pub struct Paused {
        remaining: u32,
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
                do_pause: false,
            }
        }
    }
}
