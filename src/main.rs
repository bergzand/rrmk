#![no_std]
#![no_main]

use cortex_m::singleton;
use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Input, Pull};
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use embedded_hal_async::digital::Wait;
use panic_halt as _;

mod matrix;

static S1: Signal<CriticalSectionRawMutex, bool> = Signal::new();
static S2: Signal<CriticalSectionRawMutex, bool> = Signal::new();
static S3: Signal<CriticalSectionRawMutex, bool> = Signal::new();
static S4: Signal<CriticalSectionRawMutex, bool> = Signal::new();

#[embassy_executor::task(pool_size = 4)]
async fn button_task(n: usize, mut pin: matrix::KeyPin<'static>) {
    loop {
        let _ = pin.wait_for_low().await;
        info!("Button {:?} presssed!", n);
        let _ = pin.wait_for_high().await;
        info!("Button {:?} released!", n);
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting Keyboard Matrix");
    let p = embassy_nrf::init(Default::default());
    let output_rows = [
        Output::new(p.P0_02, Level::High, OutputDrive::Standard),
        Output::new(p.P1_15, Level::High, OutputDrive::Standard),
        Output::new(p.P0_28, Level::High, OutputDrive::Standard),
        Output::new(p.P0_29, Level::High, OutputDrive::Standard),
    ];

    let input_cols = [
        Input::new(p.P1_11, Pull::Up),
        Input::new(p.P1_10, Pull::Up),
        Input::new(p.P1_06, Pull::Up),
        Input::new(p.P1_04, Pull::Up),
        Input::new(p.P1_03, Pull::Up),
        Input::new(p.P1_02, Pull::Up),
        Input::new(p.P1_01, Pull::Up),
    ];

    let matrix: &'static mut matrix::Matrix<embassy_nrf::gpio::Input, embassy_nrf::gpio::Output, 7,4> =
        singleton!(: matrix::Matrix<embassy_nrf::gpio::Input, embassy_nrf::gpio::Output, 7,4> = matrix::Matrix::new(input_cols, output_rows)).unwrap();

    let btn1 = matrix.take_pin(0, 0, &S1);
    let btn2 = matrix.take_pin(0, 1, &S2);
    let btn3 = matrix.take_pin(1, 0, &S3);
    let btn4 = matrix.take_pin(1, 1, &S4);

    spawner.spawn(button_task(1, btn1)).unwrap();
    spawner.spawn(button_task(2, btn2)).unwrap();
    spawner.spawn(button_task(3, btn3)).unwrap();
    spawner.spawn(button_task(4, btn4)).unwrap();

    loop {
        Timer::after_millis(10).await;
        matrix.scan().await;
    }
}
