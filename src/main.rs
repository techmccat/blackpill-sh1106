#![no_std]
#![no_main]

use fugit::{Duration, TimerDurationU32};
use panic_halt as _;
use stm32f4xx_hal as hal;

use cortex_m::asm;
use cortex_m_rt::entry;

use core::fmt::Write;

use embedded_graphics::{
    mono_font::{ascii::FONT_4X6, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::{ImageDrawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
    Drawable,
};
use linear_quadtree::dec::video::VideoSlice;

use hal::{i2c::I2c, prelude::*};
use sh1106::{prelude::*, Builder};

const VID: &[u8] = include_bytes!("../data/vid.bin");

#[entry]
fn main() -> ! {
    let _cp = cortex_m::Peripherals::take().unwrap();
    let sp = hal::pac::Peripherals::take().unwrap();

    let rcc = sp.RCC.constrain();
    let clocks = rcc.cfgr.use_hse(25.MHz()).sysclk(50.MHz()).freeze();

    let gpiob = sp.GPIOB.split();

    let mut display: GraphicsMode<_> = {
        let scl = gpiob.pb6.into_alternate().set_open_drain();
        let sda = gpiob.pb7.into_alternate().set_open_drain();

        let i2c = I2c::new(sp.I2C1, (scl, sda), 400.kHz(), &clocks);
        Builder::new().connect_i2c(i2c).into()
    };

    display.init().unwrap();
    display.flush().unwrap();

    let txt_style = MonoTextStyle::new(&FONT_4X6, BinaryColor::On);
    let mut fmt_buf = heapless::String::<20>::new();
    let bg_style = PrimitiveStyle::with_fill(BinaryColor::Off);
    let text_bg = Rectangle::new(Point::new(0, 0), Size::new(4 * 4, 8)).into_styled(bg_style);

    let mut counter = sp.TIM2.counter_us(&clocks);
    counter.start(3600.secs()).ok();
    let mut frames = 0u32;
    let mut total_draw_time = TimerDurationU32::<1_000_000>::from_ticks(0u32);

    for i in VideoSlice::new(VID) {
        let start = counter.now();
        let _ = i.draw(&mut display);
        frames += 1;

        let draw_time = counter
            .now()
            .checked_duration_since(start)
            .unwrap_or(Duration::<u32, 1, 1_000_000>::millis(0u32));
        total_draw_time += draw_time;

        let _ = text_bg.draw(&mut display);

        {
            write!(&mut fmt_buf, "{}ms", draw_time.to_millis()).ok();
            let flush_time_text =
                Text::with_baseline(&fmt_buf, Point::new(0, 1), txt_style, Baseline::Top);
            let _ = flush_time_text.draw(&mut display);
            fmt_buf.clear();
        }

        display.flush().unwrap();
    }

    let avg_draw_time = total_draw_time / frames;
    write!(&mut fmt_buf, "Avg: {}ms", avg_draw_time.to_millis()).ok();
    Rectangle::new(
        Point::new(0, 64 - 8),
        Size::new(fmt_buf.len() as u32 * 4, 8),
    )
    .into_styled(bg_style)
    .draw(&mut display)
    .ok();
    Text::with_baseline(
        &fmt_buf,
        Point::new(0, 64),
        txt_style,
        Baseline::Bottom,
    )
    .draw(&mut display)
    .ok();
    display.flush().unwrap();

    loop {
        asm::wfi();
    }
}
