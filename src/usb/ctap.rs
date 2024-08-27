use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::{Receiver, Sender};
use embassy_usb::class::hid::{HidReader, HidWriter, ReadError, ReportId, RequestHandler};
use embassy_usb::control::OutResponse;

use alloc::vec::Vec;
use ctap_types::Rpc;
use defmt::*;
use usbd_hid::descriptor::CtapReport;

use super::{Ctap, CtapMessage};

pub const CTAP_CHANNEL_LEN: usize = 10;

pub const CTAP_WRITER_BUF: usize = 8;
pub const CTAP_READER_BUF: usize = 7609;

async fn handle_response(
    ctap: &mut Ctap,
    sender: &mut Sender<'static, NoopRawMutex, CtapMessage, CTAP_CHANNEL_LEN>,
    buf: &[u8],
) {
    if let Ok(request) = ctap_types::ctap2::Request::deserialize(buf) {
        let result = Rpc::call(ctap, &request);
        match result {
            Ok(result) => {
                let mut buf = ctap_types::Vec::<_, 64>::new();
                result.serialize(&mut buf);
                sender.send(buf.into_array().unwrap()).await;
            }
            Err(err) => {
                let mut buf = [0; 64];
                buf[0] = err as u8;
                sender.send(buf).await;
            }
        }
        return;
    }
    // TODO: Handle CTAP1 requests
    // if let Ok(request) = ctap_types::ctap1::Request::deserialize(buf) {
    //     // HANDLE CTAP1 requests here
    //     return;
    // }
    warn!("CTAP Request could not be deserialized as CTAP1 or CTAP2");
}

#[embassy_executor::task]
pub async fn ctap_writer(
    mut writer: HidWriter<'static, Driver<'static, USB>, CTAP_WRITER_BUF>,
    receiver: Receiver<'static, NoopRawMutex, CtapMessage, CTAP_CHANNEL_LEN>,
) {
    loop {
        let rep = receiver.receive().await;
        info!("Writing ctap response to host");
        let report = CtapReport {
            data_in: rep,
            data_out: [0; 64], // THIS IS NOT NEEDED?
        };
        // Send the report.
        if let Err(e) = writer.write_serialize(&report).await {
            warn!("Failed to send report: {:?}", e)
        }
    }
}

// #[embassy_executor::task]
// pub async fn ctap_reader(
//     mut reader: HidReader<'static, Driver<'static, USB>, CTAP_READER_BUF>,
//     mut sender: Sender<'static, NoopRawMutex, CtapMessage, CTAP_CHANNEL_LEN>,
// ) {
//     let mut buf = [0; CTAP_READER_BUF];
//     // This is only used when receivng Sync
//     // (The message(s) was too big to fit in one packget)
//     let mut multi_buf: Vec<u8> = Vec::new();

//     let mut ctap = Ctap;

//     loop {
//         let resp = reader.read(&mut buf).await;
//         match resp {
//             Ok(_) => {
//                 if multi_buf.is_empty() {
//                     handle_response(&mut ctap, &mut sender, &buf).await;
//                 } else {
//                     handle_response(&mut ctap, &mut sender, &multi_buf).await;
//                     multi_buf.drain(..);
//                 }
//             }
//             Err(ReadError::BufferOverflow) => warn!("Usb got BufferOverflow (Buffer too small)"),
//             Err(ReadError::Disabled) => warn!("Ctap usb reader got Disabled)"),
//             Err(ReadError::Sync(range)) => {
//                 multi_buf.extend_from_slice(&buf[range]);
//             }
//         };
//     }
// }

#[embassy_executor::task]
pub async fn ctap_reader(
    reader: HidReader<'static, Driver<'static, USB>, CTAP_READER_BUF>,
    ctap_send: Sender<'static, NoopRawMutex, CtapMessage, CTAP_CHANNEL_LEN>,
) {
    let mut handler = CtapRequestHandler(ctap_send);
    reader.run(false, &mut handler).await;
}

struct CtapRequestHandler(Sender<'static, NoopRawMutex, CtapMessage, CTAP_CHANNEL_LEN>);

impl RequestHandler for CtapRequestHandler {
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
