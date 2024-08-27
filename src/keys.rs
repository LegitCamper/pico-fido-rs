use embassy_rp::clocks::RoscRng;
use embassy_rp::flash::Async;
use embassy_rp::flash::Flash;
use embassy_rp::peripherals::FLASH;

use defmt::*;
use k256::{ecdh::EphemeralSecret, EncodedPoint, PublicKey};
use rand::{CryptoRng, Rng, RngCore};
use serde::{Deserialize, Serialize};

use super::{ADDR_OFFSET, FLASH_SIZE};

// THIS IS A VERY VERY BAD SOURCE OF RANDOMNESS
// but will work for now
pub struct CryptRng(u32);

impl CryptRng {
    pub fn new() -> Self {
        let mut rng = RoscRng;
        CryptRng(rng.gen())
    }
}

impl RngCore for CryptRng {
    fn next_u32(&mut self) -> u32 {
        RoscRng::next_u32(&mut RoscRng)
    }

    fn next_u64(&mut self) -> u64 {
        RoscRng::next_u64(&mut RoscRng)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        RoscRng::fill_bytes(&mut RoscRng, dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        RoscRng::try_fill_bytes(&mut RoscRng, dest)
    }
}

impl CryptoRng for CryptRng {}

pub struct CtapCredential {
    id: [u8; 64],
    name: [u8; 64],
}

impl CtapCredential {
    pub fn new(id: &[u8], name: &[u8]) {
        if id.len() > 64 {
            warn!("Given ID was bigger than 64 bytes")
        }
        if id.len() > 64 {
            warn!("Given ID was bigger than 64 bytes")
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Keys {
    // // This will live for the live of the device unless RESET
    // secret: EphemeralSecret,
    // // resident keys that have been stored
    // rks: CtapCredential,
}

impl Keys {
    pub fn new(mut flash: Flash<FLASH, Async, FLASH_SIZE>) -> Self {
        // TODO: read from flash and fetch stored secret
        Keys {
            // secret: Self::create_new_key(),
        }
    }

    fn create_new_key() -> EphemeralSecret {
        EphemeralSecret::random(&mut CryptRng::new())
    }
}
