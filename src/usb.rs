use embassy_executor::SpawnToken;
use embassy_rp::clocks::RoscRng;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_rp::{bind_interrupts, usb::InterruptHandler};
use embassy_time::Timer;
use embassy_usb::class::hid::HidReader;
use embassy_usb::class::hid::HidWriter;
use embassy_usb::class::hid::{HidReaderWriter, ReportId, RequestHandler, State};
use embassy_usb::control::OutResponse;
use embassy_usb::{Builder, Config, Handler, UsbDevice};
use static_cell::StaticCell;
use usbd_hid::descriptor::{MouseReport, SerializedDescriptor};

use core::sync::atomic::{AtomicBool, Ordering};
use defmt::*;
use rand::Rng;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

const HID_WRITER_WRITE_N: usize = 8;
const HID_READ_READ_N: usize = 1;

#[embassy_executor::task]
async fn run_usb(mut usb: UsbDevice<'static, Driver<'static, USB>>) {
    usb.run().await;
}

#[embassy_executor::task]
async fn usb_hid_in(mut writer: HidWriter<'static, Driver<'static, USB>, HID_WRITER_WRITE_N>) {
    let mut rng = RoscRng;

    loop {
        _ = Timer::after_secs(1).await;
        let report = MouseReport {
            buttons: 0,
            x: rng.gen_range(-100..100), // random small x movement
            y: rng.gen_range(-100..100), // random small y movement
            wheel: 0,
            pan: 0,
        };
        // Send the report.
        match writer.write_serialize(&report).await {
            Ok(()) => {}
            Err(e) => warn!("Failed to send report: {:?}", e),
        }
    }
}

#[embassy_executor::task]
async fn usb_hid_out(reader: HidReader<'static, Driver<'static, USB>, HID_READ_READ_N>) {
    let mut request_handler = UsbRequestHandler {};
    reader.run(false, &mut request_handler).await;
}

pub fn create_usb_tasks(
    usb: USB,
) -> (
    SpawnToken<impl Sized>,
    SpawnToken<impl Sized>,
    SpawnToken<impl Sized>,
) {
    let driver = Driver::new(usb, Irqs);

    // These are what is reconized by ctap apps like yubikey
    // and may need to be changed to be reconized
    // Create embassy-usb Config - VID, PID
    let mut config = Config::new(0xc0de, 0xcafe);
    // Configures usb as a composite device (uses more than one protocol)
    config.composite_with_iads = true;
    config.device_class = 0xEF; // composite class (multiple usb protocols)
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.manufacturer = Some("LegtCamper");
    config.product = Some("Pico Fido");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    // You can also add a Microsoft OS descriptor.
    static MSOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

    let mut builder = Builder::new(
        driver,
        config,
        CONFIG_DESCRIPTOR.init([0; 256]),
        BOS_DESCRIPTOR.init([0; 256]),
        MSOS_DESCRIPTOR.init([0; 256]),
        CONTROL_BUF.init([0; 64]),
    );

    // Tell usb what our composite classes are
    builder.function(0x03, 0x00, 0x00); // HID
    builder.function(0x0B, 0x00, 0x00); // CCID

    static DEVICE_HANDLER: StaticCell<UsbHandler> = StaticCell::new();
    builder.handler(DEVICE_HANDLER.init(UsbHandler::new()));

    static STATE: StaticCell<State> = StaticCell::new();

    // Create an hid class config
    let config = embassy_usb::class::hid::Config {
        report_descriptor: MouseReport::desc(),
        request_handler: None,
        poll_ms: 60,
        max_packet_size: 64,
    };
    let hid = HidReaderWriter::<_, 1, 8>::new(&mut builder, STATE.init(State::new()), config);

    // CREATE THE CCID CONFIG AND READER/WRITER HERE

    // Build the builder.
    let usb = builder.build();

    let (hid_reader, hid_writer) = hid.split();

    // return usb tasks
    (
        run_usb(usb),
        usb_hid_in(hid_writer),
        usb_hid_out(hid_reader),
    )
}

struct UsbRequestHandler {}

impl RequestHandler for UsbRequestHandler {
    fn get_report(&mut self, id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        info!("Get report for {:?}", id);
        None
    }

    fn set_report(&mut self, id: ReportId, data: &[u8]) -> OutResponse {
        info!("Set report for {:?}: {=[u8]}", id, data);
        OutResponse::Accepted
    }

    fn set_idle_ms(&mut self, id: Option<ReportId>, dur: u32) {
        info!("Set idle rate for {:?} to {:?}", id, dur);
    }

    fn get_idle_ms(&mut self, id: Option<ReportId>) -> Option<u32> {
        info!("Get idle rate for {:?}", id);
        None
    }
}

// Handles control events not handled by the usb stack
struct UsbHandler {
    configured: AtomicBool,
}

impl UsbHandler {
    fn new() -> Self {
        UsbHandler {
            configured: AtomicBool::new(false),
        }
    }
}

impl Handler for UsbHandler {
    fn enabled(&mut self, enabled: bool) {
        self.configured.store(false, Ordering::Relaxed);
        if enabled {
            info!("Device enabled");
        } else {
            info!("Device disabled");
        }
    }

    fn reset(&mut self) {
        self.configured.store(false, Ordering::Relaxed);
        info!("Bus reset, the Vbus current limit is 100mA");
    }

    fn addressed(&mut self, addr: u8) {
        self.configured.store(false, Ordering::Relaxed);
        info!("USB address set to: {}", addr);
    }

    fn configured(&mut self, configured: bool) {
        self.configured.store(configured, Ordering::Relaxed);
        if configured {
            info!(
                "Device configured, it may now draw up to the configured current limit from Vbus."
            )
        } else {
            info!("Device is no longer configured, the Vbus current limit is 100mA.");
        }
    }
}

mod fido_class {}