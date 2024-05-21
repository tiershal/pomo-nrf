#![no_main]
#![no_std]

use pomo_nrf as _;

use pomo_nrf::hal;

#[rtic::app(device = nrf52840_hal::pac, dispatchers = [SWI0_EGU0])]
mod app {
    use super::*;

    use embedded_hal::delay::DelayNs;

    use hal::Clocks;
    use rtic_monotonics::nrf::rtc::Rtc0;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        rtc0: Rtc0,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        defmt::println!("init");

        Clocks::new(ctx.device.CLOCK).start_lfclk();

        let token = rtic_monotonics::create_nrf_rtc0_monotonic_token!();
        Rtc0::start(ctx.device.RTC0, token);

        let rtc0 = Rtc0;

        on_tick::spawn().ok();

        (Shared {}, Local { rtc0 })
    }

    #[task(local = [rtc0])]
    async fn on_tick(ctx: on_tick::Context) {
        loop {
            defmt::println!("tick is happening");
            ctx.local.rtc0.delay_ms(1000);
        }
    }
}
