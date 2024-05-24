#![no_main]
#![no_std]

use pomo_nrf as _;

#[rtic::app(device = nrf52840_hal::pac, dispatchers = [SWI0_EGU0])]
mod app {

    use nrf52840_hal as hal;

    use hal::{gpiote::Gpiote, Clocks};
    use pomo_nrf::state::{
        DoPause, DoResume, Paused, PomoStateMachine, Running, TIME_INTERVAL_MSECS,
        TIME_RUNNING_MSECS,
    };
    use rtic_monotonics::nrf::rtc::Rtc0;
    use sfsm::{PushMessage, StateMachine};

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
            .start(Running::new(TIME_RUNNING_MSECS))
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

            Rtc0::delay(TIME_INTERVAL_MSECS.convert()).await;
        }
    }

    #[task(binds = GPIOTE, shared = [state_machine, gpiote], priority = 3)]
    fn on_button(mut ctx: on_button::Context) {
        // Once multiple buttons get implemented, we will need to handle which button has been pressed.
        // We might not want to support multiple buttons being pressed at the same time, and some buttons
        // might have priority in terms of state transitioning.
        ctx.shared.gpiote.lock(|gpiote| {
            gpiote.channel0().event().reset();
        });

        ctx.shared.state_machine.lock(|sm| {
            // Since the state machine can only be in one state at any given time, we could just fire all possible messages for this button
            // and the state we're in can handle the message safely.
            //
            // We don't need to care about the result of the message being sent.
            _ = PushMessage::<Running, DoPause>::push_message(sm, DoPause);
            _ = PushMessage::<Paused, DoResume>::push_message(sm, DoResume);
        });
    }
}
