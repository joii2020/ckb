#![no_main]

use libfuzzer_sys::fuzz_target;

use ckb_network::fuzz_export::*;

fuzz_target!(|data: &[u8]| fuzz_peer_registry(data));
