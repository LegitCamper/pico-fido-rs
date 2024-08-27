use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Receiver;
use embassy_usb::class::hid::{HidReader, HidWriter, ReportId, RequestHandler};
use embassy_usb::control::OutResponse;
use usbd_hid::descriptor::KeyboardReport;

use defmt::*;

use super::KeyboardUsage;

// This is only meant for entering 2fa codes (typically 6 chars)
pub const HID_CHANNEL_LEN: usize = 12;

pub const HID_WRITER_BUF: usize = 8;
pub const HID_READER_BUF: usize = 1;

async fn send_key(writer: &mut HidWriter<'static, Driver<'static, USB>, HID_WRITER_BUF>, key: u8) {
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

#[embassy_executor::task]
pub async fn hid_writer(
    mut writer: HidWriter<'static, Driver<'static, USB>, HID_WRITER_BUF>,
    receiver: Receiver<'static, NoopRawMutex, KeyboardUsage, HID_CHANNEL_LEN>,
) {
    loop {
        let key = receiver.receive().await;
        send_key(&mut writer, key as u8).await;
        send_key(&mut writer, 0).await; // unpress key
    }
}

#[embassy_executor::task]
pub async fn hid_reader(reader: HidReader<'static, Driver<'static, USB>, HID_READER_BUF>) {
    let mut handler = HidRequestHandler {};
    reader.run(false, &mut handler).await;
}

struct HidRequestHandler {}

impl RequestHandler for HidRequestHandler {
    fn get_report(&mut self, _id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        None
    }

    fn set_report(&mut self, _id: ReportId, _data: &[u8]) -> OutResponse {
        OutResponse::Accepted
    }

    fn set_idle_ms(&mut self, _id: Option<ReportId>, _dur: u32) {}

    fn get_idle_ms(&mut self, _id: Option<ReportId>) -> Option<u32> {
        None
    }
}
