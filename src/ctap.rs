use ctap_types::authenticator::{Authenticator, Ctap1Authenticator, Ctap2Authenticator};
use ctap_types::ctap1::*;
use ctap_types::ctap2::*;
use ctap_types::Vec;

use core::sync::atomic::Ordering;
use defmt::info;
use embassy_time::{Duration, Timer};

use super::{LedState, BOOTSEL_BUTTON, LED_SIGNAL};

pub struct Ctap;

// For now instead of saving and reading keys from flash
// we'll just read and write to ram
impl Ctap {
    fn has_credential_id(
        &self,
        credential: &ctap_types::webauthn::PublicKeyCredentialDescriptorRef,
    ) -> bool {
        todo!()
    }
    fn get_credential_id(
        &self,
        credential: &ctap_types::webauthn::PublicKeyCredentialDescriptorRef,
    ) {
        todo!();
    }
}

impl Ctap1Authenticator for Ctap {
    fn register(
        &mut self,
        request: &register::Request<'_>,
    ) -> ctap_types::ctap1::Result<register::Response> {
        todo!()
    }

    fn authenticate(
        &mut self,
        request: &authenticate::Request<'_>,
    ) -> ctap_types::ctap1::Result<authenticate::Response> {
        todo!()
    }
}

impl Ctap2Authenticator for Ctap {
    fn get_info(&mut self) -> get_info::Response {
        info!("Getting authenticator info");
        let versions = [
            get_info::Version::Fido2_0,
            get_info::Version::Fido2_1,
            get_info::Version::Fido2_1Pre,
            // get_info::Version::U2fV2, // Currently I dont handle Ctap 1
        ];
        let aaguid = [0; 16]; // dont know what this is
        let resp_builder = get_info::ResponseBuilder {
            versions: Vec::from_slice(&versions).unwrap(),
            aaguid: ctap_types::Bytes::from_slice(&aaguid).unwrap(),
        };

        resp_builder.build()
    }

    fn make_credential(
        &mut self,
        request: &make_credential::Request,
    ) -> ctap_types::Result<make_credential::Response> {
        // if let Some(list) = &request.exclude_list {
        //     for cred in list {
        //         if self.has_credential_id(cred) && !self.get_credential_id(cred).rpld.is_empty() {
        //             LED_SIGNAL.signal(LedState::Confirm);
        //             while !BOOTSEL_BUTTON.load(Ordering::Relaxed) {
        //                 // Timer::after(Duration::from_millis(100)).await;
        //             }
        //             LED_SIGNAL.signal(LedState::Active);
        //             return ctap_types::Result::Err(ctap_types::ctap2::Error::CredentialExcluded);
        //         }
        //     }
        // }

        // // Check for supported Algos and return CTAP2_ERR_UNSUPPORTED_ALGORITHM if unsupported

        // LED_SIGNAL.signal(LedState::Idle);
        // // TODO: Remove this
        // ctap_types::Result::Err(ctap_types::ctap2::Error::Success)
        todo!();
    }

    fn get_assertion(
        &mut self,
        request: &get_assertion::Request,
    ) -> ctap_types::Result<get_assertion::Response> {
        todo!()
    }

    fn get_next_assertion(&mut self) -> ctap_types::Result<get_assertion::Response> {
        todo!()
    }

    fn reset(&mut self) -> ctap_types::Result<()> {
        todo!()
    }

    fn client_pin(
        &mut self,
        request: &client_pin::Request,
    ) -> ctap_types::Result<client_pin::Response> {
        todo!()
    }

    fn credential_management(
        &mut self,
        request: &credential_management::Request,
    ) -> ctap_types::Result<credential_management::Response> {
        todo!()
    }

    fn selection(&mut self) -> ctap_types::Result<()> {
        todo!()
    }

    fn vendor(&mut self, op: VendorOperation) -> ctap_types::Result<()> {
        todo!()
    }
}
