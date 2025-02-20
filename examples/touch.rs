#![no_main]
#![no_std]

/// A basic example to show the use of ft6x06 crate using STM32F412/3 board
/// This example shows how to get touch coordinates.
use cortex_m;
use cortex_m_rt::entry;
use rtt_target::{rprintln, rtt_init_print};
#[cfg(feature = "stm32f413")]
use stm32f4xx_hal::fmpi2c::FMPI2c;
#[cfg(feature = "stm32f412")]
use stm32f4xx_hal::i2c::I2c;
use stm32f4xx_hal::{pac, prelude::*, rcc::Rcc};

#[allow(unused_imports)]
use panic_semihosting;

extern crate ft6x06;

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Started");

    let perif = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    let rcc: Rcc = perif.RCC.constrain();

    let clocks = rcc.cfgr.sysclk(100.MHz()).freeze();
    let mut delay = cp.SYST.delay(&clocks);

    rprintln!("Connecting to I2c");

    // STM32F412 touchscreen controller uses I2C1 module from embedded-hal.
    #[cfg(feature = "stm32f412")]
    let mut i2c = {
        let gpiob = perif.GPIOB.split();
        I2c::new(
            perif.I2C1,
            (
                gpiob.pb6.into_alternate().set_open_drain(),
                gpiob.pb7.into_alternate().set_open_drain(),
            ),
            10.kHz(),
            &clocks,
        )
    };

    // STM32F413 shares the same I2C bus for both audio driver and touchscreen controller. FMPI2C module from embedded-hal is used.
    #[cfg(feature = "stm32f413")]
    let mut i2c = {
        let gpioc = perif.GPIOC.split();
        FMPI2c::new(
            perif.FMPI2C1,
            (
                gpioc.pc6.into_alternate().set_open_drain(),
                gpioc.pc7.into_alternate().set_open_drain(),
            ),
            5.kHz(),
        )
    };

    let mut touch = ft6x06::Ft6X06::new(&i2c, 0x38).unwrap();

    let tsc = touch.ts_calibration(&mut i2c, &mut delay);
    match tsc {
        Err(e) => rprintln!("Error {} from ts_calibration", e),
        Ok(u) => rprintln!("ts_calibration returned {}", u),
    }
    rprintln!("If nothing happens - touch the screen!");

    loop {
        let t = touch.detect_touch(&mut i2c);
        let mut num: u8 = 0;
        match t {
            Err(e) => rprintln!("Error {} from fetching number of touches", e),
            Ok(n) => {
                num = n;
                if num != 0 {
                    rprintln!("Number of touches: {}", num)
                };
            }
        }

        if num > 0 {
            let t = touch.get_touch(&mut i2c, 1);
            match t {
                Err(_e) => rprintln!("Error fetching touch data"),
                Ok(n) => rprintln!(
                    "Touch: {:>3}x{:>3} - weight: {:>3} misc: {}",
                    n.x,
                    n.y,
                    n.weight,
                    n.misc
                ),
            }
        }
    }
}

