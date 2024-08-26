use embassy_rp::clocks::RoscRng;
use p256::{ecdh::EphemeralSecret, EncodedPoint, PublicKey};
use rand::{CryptoRng, Rng, RngCore};
// use rand_core::{CryptoRng, RngCore};

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

pub struct Key;

impl Key {
    pub fn create_new_key() -> EphemeralSecret {
        EphemeralSecret::random(&mut CryptRng::new())
    }
}
