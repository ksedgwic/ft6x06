#![no_main]
#![no_std]
#![allow(unused_variables)]

// #![deny(warnings)]
/// A simple example to connect to the FT6x06 crate on stm32f412/3 board and get the touch data to evaluate
/// which option is clicked by the user. This can be considered as interface
/// for user to give an input.
use cortex_m;
use cortex_m_rt::entry;
use rtt_target::{rprintln, rtt_init_print};
#[cfg(feature = "stm32f412")]
use stm32f4xx_hal::fsmc_lcd::ChipSelect1;
#[cfg(feature = "stm32f413")]
use stm32f4xx_hal::fsmc_lcd::ChipSelect3;
use stm32f4xx_hal::{
    fsmc_lcd::{FsmcLcd, LcdPins, Timing},
    pac,
    prelude::*,
    rcc::Rcc,
};

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::Text,
};

#[cfg(feature = "stm32f413")]
use stm32f4xx_hal::fmpi2c::FMPI2c;
#[cfg(feature = "stm32f412")]
use stm32f4xx_hal::i2c::I2c;

#[allow(unused_imports)]
use panic_semihosting;

use ft6x06;
use st7789::*;

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Started");

    let p = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    let rcc: Rcc = p.RCC.constrain();

    let clocks = rcc.cfgr.sysclk(100.MHz()).freeze();
    let mut delay = cp.SYST.delay(&clocks);

    let gpiob = p.GPIOB.split();
    let gpioc = p.GPIOC.split();
    let gpiod = p.GPIOD.split();
    let gpioe = p.GPIOE.split();
    let gpiof = p.GPIOF.split();
    let gpiog = p.GPIOG.split();

    let lcd_pins = LcdPins {
        data: (
            gpiod.pd14.into_alternate(),
            gpiod.pd15.into_alternate(),
            gpiod.pd0.into_alternate(),
            gpiod.pd1.into_alternate(),
            gpioe.pe7.into_alternate(),
            gpioe.pe8.into_alternate(),
            gpioe.pe9.into_alternate(),
            gpioe.pe10.into_alternate(),
            gpioe.pe11.into_alternate(),
            gpioe.pe12.into_alternate(),
            gpioe.pe13.into_alternate(),
            gpioe.pe14.into_alternate(),
            gpioe.pe15.into_alternate(),
            gpiod.pd8.into_alternate(),
            gpiod.pd9.into_alternate(),
            gpiod.pd10.into_alternate(),
        ),
        address: gpiof.pf0.into_alternate(),
        read_enable: gpiod.pd4.into_alternate(),
        write_enable: gpiod.pd5.into_alternate(),
        #[cfg(feature = "stm32f413")]
        chip_select: ChipSelect3(gpiog.pg10.into_alternate()),
        #[cfg(feature = "stm32f412")]
        chip_select: ChipSelect1(gpiod.pd7.into_alternate()),
    };

    // Setup the RESET pin
    #[cfg(feature = "stm32f413")]
    let rst = gpiob.pb13.into_push_pull_output();
    // Enable backlight
    #[cfg(feature = "stm32f413")]
    let mut backlight_control = gpioe.pe5.into_push_pull_output();

    #[cfg(feature = "stm32f412")]
    let rst = gpiod.pd11.into_push_pull_output();
    #[cfg(feature = "stm32f412")]
    let mut backlight_control = gpiof.pf5.into_push_pull_output();

    backlight_control.set_high();
    // We're not using the "tearing" signal from the display
    let mut _te = gpiob.pb14.into_floating_input();

    // Set up timing
    let write_timing = Timing::default().data(3).address_setup(3).bus_turnaround(0);
    let read_timing = Timing::default().data(8).address_setup(8).bus_turnaround(0);

    // Initialise FSMC memory provider
    let (_fsmc, interface) = FsmcLcd::new(p.FSMC, lcd_pins, &read_timing, &write_timing);

    // Pass display-interface instance ST7789 driver to setup a new display
    let mut disp = ST7789::new(interface, rst, 240, 240);

    // Initialise the display and clear the screen
    disp.init(&mut delay).unwrap();
    disp.set_orientation(Orientation::Portrait).unwrap();
    rprintln!("{}", disp.orientation() as u8);
    disp.clear(Rgb565::BLACK).unwrap();

    let style = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb565::RED)
        .stroke_width(3)
        .fill_color(Rgb565::BLACK)
        .build();

    Rectangle::new(Point::new(140, 80), Size::new(80, 80))
        .into_styled(style)
        .draw(&mut disp)
        .unwrap();

    Rectangle::new(Point::new(20, 80), Size::new(80, 80))
        .into_styled(style)
        .draw(&mut disp)
        .unwrap();

    // Create a new character style
    let text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);

    Text::new("No", Point::new(180, 120), text_style)
        .draw(&mut disp)
        .unwrap();
    Text::new("Yes", Point::new(60, 120), text_style)
        .draw(&mut disp)
        .unwrap();

    rprintln!("Connecting to I2c");

    #[cfg(feature = "stm32f412")]
    let mut i2c = {
        I2c::new(
            p.I2C1,
            (
                gpiob.pb6.into_alternate().set_open_drain(),
                gpiob.pb7.into_alternate().set_open_drain(),
            ),
            400.kHz(),
            &clocks,
        )
    };

    #[cfg(feature = "stm32f413")]
    let mut i2c = {
        FMPI2c::new(
            p.FMPI2C1,
            (
                gpioc.pc6.into_alternate().set_open_drain(),
                gpioc.pc7.into_alternate().set_open_drain(),
            ),
            10.kHz(),
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
                Ok(n) => {
                    if n.x > 80 && n.x < 160 {
                        if n.y < 100 && n.y > 20 {
                            rprintln!("You pressed Yes");
                        } else if n.y < 200 && n.y > 120 {
                            rprintln!("You pressed No");
                        } else {
                            rprintln!("Press a key");
                        }
                    } else {
                        rprintln!("Press a key");
                    }
                }
            }
        }
    }
}
