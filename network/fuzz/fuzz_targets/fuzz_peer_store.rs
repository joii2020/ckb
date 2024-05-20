#![no_main]

use libfuzzer_sys::fuzz_target;

use ckb_network::bytes::{Bytes, BytesMut};
use ckb_network::peer_store::PeerStore;

fuzz_target!(|data: &[u8]| {
    let mut peer_store: PeerStore = Default::default();

    

    // add_connected_peer
    // add_addr
    // add_outbound_addr
    // update_outbound_addr_last_connected_ms
    // fetch_addrs_to_attempt
    // fetch_addrs_to_feeler
    // fetch_random_addrs

});
