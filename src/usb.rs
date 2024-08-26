use embassy_executor::SpawnToken;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_rp::{bind_interrupts, usb::InterruptHandler};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Receiver;
use embassy_time::Timer;
use embassy_usb::class::hid::{HidReader, HidWriter, ReadError};
use embassy_usb::class::hid::{HidReaderWriter, ReportId, RequestHandler, State};
use embassy_usb::control::OutResponse;
use embassy_usb::{Builder, Config, Handler, UsbDevice};

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};
use defmt::*;
use static_cell::StaticCell;
use usbd_hid::descriptor::{CtapReport, KeyboardReport, KeyboardUsage, SerializedDescriptor};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

const HID_WRITER_BUF: usize = 8;
const HID_READER_BUF: usize = 1;
const CTAP_WRITER_BUF: usize = 8;
// https://fidoalliance.org/specs/fido-v2.0-id-20180227/fido-client-to-authenticator-protocol-v2.0-id-20180227.html#usb-message-and-packet-structure
// length is 64 - 7 + 128 * (64 - 5) = 7609 bytes.
const CTAP_READER_BUF: usize = 7609;

#[embassy_executor::task]
async fn run_usb(mut usb: UsbDevice<'static, Driver<'static, USB>>) {
    usb.run().await;
}

#[embassy_executor::task]
async fn usb_hid_keyboard_in(
    mut writer: HidWriter<'static, Driver<'static, USB>, HID_WRITER_BUF>,
    receiver: Receiver<'static, NoopRawMutex, KeyboardUsage, 12>,
) {
    async fn send_key(
        writer: &mut HidWriter<'static, Driver<'static, USB>, HID_WRITER_BUF>,
        key: u8,
    ) {
        let report = KeyboardReport {
            modifier: 0,
            reserved: 0,
            leds: 0,
            keycodes: [key, 0, 0, 0, 0, 0],
        };
        // Send the report.
        if let Err(e) = writer.write_serialize(&report).await {
            warn!("Failed to send report: {:?}", e)
        }
    }

    loop {
        let key = receiver.receive().await;
        send_key(&mut writer, key as u8).await;
        send_key(&mut writer, 0).await; // unpress key
    }
}

#[embassy_executor::task]
async fn usb_ctap_in(mut writer: HidWriter<'static, Driver<'static, USB>, CTAP_WRITER_BUF>) {
    loop {
        let report = CtapReport {
            data_in: [0; 64],
            data_out: [0; 64],
        };
        // Send the report.
        if let Err(e) = writer.write_serialize(&report).await {
            warn!("Failed to send report: {:?}", e)
        }
    }
}

#[embassy_executor::task]
async fn usb_hid_keyboard_out(reader: HidReader<'static, Driver<'static, USB>, HID_READER_BUF>) {
    let mut request_handler = UsbRequestHandler {};
    reader.run(false, &mut request_handler).await;
}

#[embassy_executor::task]
async fn usb_ctap_out(mut reader: HidReader<'static, Driver<'static, USB>, CTAP_READER_BUF>) {
    let mut buf = [0; CTAP_READER_BUF];
    // This is only used when receivng Sync
    // (The message(s) was too big to fit in one packget)
    let mut multi_buf: Vec<u8> = Vec::new();

    fn handle_response(buf: &[u8]) {}

    loop {
        let resp = reader.read(&mut buf).await;
        match resp {
            Ok(_) => (),
            Err(ReadError::BufferOverflow) => warn!("Usb got BufferOverflow (Buffer too small)"),
            Err(ReadError::Disabled) => warn!("Ctap usb reader got Disabled)"),
            Err(ReadError::Sync(_length_buf)) => {
                multi_buf.extend_from_slice(&buf);
            }
        };
        if multi_buf.is_empty() {
            handle_response(&buf);
        } else {
            handle_response(&multi_buf);
            multi_buf.drain(..);
        }
    }
}

pub fn create_usb_tasks(
    usb: USB,
    keyboard_reciever: Receiver<'static, NoopRawMutex, KeyboardUsage, 12>,
) -> (
    SpawnToken<impl Sized>,
    SpawnToken<impl Sized>,
    SpawnToken<impl Sized>,
    SpawnToken<impl Sized>,
    SpawnToken<impl Sized>,
    // SpawnToken<impl Sized>,
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

    static DEVICE_HANDLER: StaticCell<UsbHandler> = StaticCell::new();
    builder.handler(DEVICE_HANDLER.init(UsbHandler::new()));

    // Tell usb what our composite classes are
    builder.function(0x03, 0x00, 0x00); // HID
    builder.function(0x0B, 0x00, 0x00); // CCID

    // Create the usb classes

    let hid_keyboard = {
        let config = embassy_usb::class::hid::Config {
            report_descriptor: KeyboardReport::desc(),
            request_handler: None,
            poll_ms: 60,
            max_packet_size: 64,
        };
        static STATE: StaticCell<State> = StaticCell::new();
        // TODO: This could probably just be a HidWriter
        HidReaderWriter::<_, 1, 8>::new(&mut builder, STATE.init(State::new()), config)
    };
    let (hid_keyboard_reader, hid_keyboard_writer) = hid_keyboard.split();

    let hid_ctap = {
        let config = embassy_usb::class::hid::Config {
            report_descriptor: CtapReport::desc(),
            request_handler: None,
            poll_ms: 60,
            max_packet_size: 64,
        };
        static STATE: StaticCell<State> = StaticCell::new();
        HidReaderWriter::<_, CTAP_READER_BUF, CTAP_WRITER_BUF>::new(
            &mut builder,
            STATE.init(State::new()),
            config,
        )
    };
    let (hid_ctap_reader, hid_ctap_writer) = hid_ctap.split();

    // CREATE THE CCID CONFIG AND READER/WRITER HERE

    // Build the builder.
    let usb = builder.build();

    // return usb tasks
    (
        run_usb(usb),
        usb_hid_keyboard_in(hid_keyboard_writer, keyboard_reciever),
        usb_ctap_in(hid_ctap_writer),
        usb_hid_keyboard_out(hid_keyboard_reader),
        usb_ctap_out(hid_ctap_reader),
        // usb_hid_ctap_run(hid_ctap),
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
