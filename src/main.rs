#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

#[global_allocator]
static HEAP: Heap = Heap::empty();
extern crate alloc;

use defmt::*;
use embassy_executor::Spawner;
// use embassy_rp::flash::Flash;
use core::sync::atomic::{AtomicBool, Ordering};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::BOOTSEL;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use embedded_alloc::Heap;
use static_cell::StaticCell;
use usbd_hid::descriptor::KeyboardUsage;
use {defmt_rtt as _, panic_probe as _};

mod usb;
use usb::create_usb_tasks;
mod ctap;
mod keys;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting");
    let p = embassy_rp::init(Default::default());

    spawner.spawn(bootsel_pressed(p.BOOTSEL)).unwrap();

    // Flash::new_blocking();

    // Get board specific pin
    let led_pin = {
        #[cfg(feature = "rp2040_board")]
        Output::new(p.PIN_25, Level::Low)
    };
    spawner.spawn(blinker(led_pin)).unwrap();

    // This is only meant for entering 2fa codes (typically 6 chars)
    static HID_KEYBOARD_CHANNEL: StaticCell<Channel<NoopRawMutex, KeyboardUsage, 12>> =
        StaticCell::new();
    let keyboard_channel =
        HID_KEYBOARD_CHANNEL.init(Channel::<NoopRawMutex, KeyboardUsage, 12>::new());

    let (usb_task, hid_keyboard_writer, hid_ctap_writer, hid_keyboard_reader, hid_ctap_reader) =
        create_usb_tasks(p.USB, keyboard_channel.receiver());

    spawner.spawn(usb_task).unwrap();
    spawner.spawn(hid_keyboard_writer).unwrap();
    spawner.spawn(hid_ctap_writer).unwrap();
    spawner.spawn(hid_keyboard_reader).unwrap();
    spawner.spawn(hid_ctap_reader).unwrap();
}

pub const BOOTSEL_BUTTON: AtomicBool = AtomicBool::new(false);

// This sucks because it has to pull the button status
// but without forcing people to bring their own button
// this is the only way
#[embassy_executor::task]
pub async fn bootsel_pressed(mut button: BOOTSEL) {
    // Pull the button status every hundred milliseconds
    loop {
        Timer::after(Duration::from_millis(100)).await;
        let status = BOOTSEL::is_pressed(&mut button);
        BOOTSEL_BUTTON.store(status, Ordering::Relaxed);
        println!("button changed {}", status);
    }
}

pub static LED_SIGNAL: Signal<CriticalSectionRawMutex, LedState> = Signal::new();

#[derive(Debug, Default, Format)]
pub enum LedState {
    Confirm, // Waiting for user to confirm
    #[default]
    Idle, // Pico goes to sleep
    Active,  // Awake and waiting for a command
    Processing, // Busy and cannot receive new commands
}

#[embassy_executor::task]
pub async fn blinker(mut led: Output<'static>) {
    let mut signal = LedState::default();

    loop {
        if let Some(new_signal) = LED_SIGNAL.try_take() {
            info!("Got new signal: {}", new_signal);
            signal = new_signal;
        }

        let (on_time, off_time) = match signal {
            LedState::Confirm => (Duration::from_secs(1), Duration::from_millis(100)),
            LedState::Idle => (Duration::from_millis(500), Duration::from_secs(1)),
            LedState::Active => (Duration::from_millis(200), Duration::from_millis(200)),
            LedState::Processing => (Duration::from_millis(50), Duration::from_millis(50)),
        };

        led.set_high();
        Timer::after(on_time).await;
        led.set_low();
        Timer::after(off_time).await;
    }
}
