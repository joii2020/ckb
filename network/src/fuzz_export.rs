use p2p::bytes::{Bytes, BytesMut};

use crate::{
    compress::{compress, decompress, Message},
    errors::{Error, PeerError},
    multiaddr::Multiaddr,
    peer_registry::PeerRegistry,
    peer_store::{addr_manager::AddrManager, types::AddrInfo, PeerStore},
    PeerId, SessionType,
};
use std::net::Ipv4Addr;

pub fn fuzz_compress(data: &[u8]) {
    let raw_data = Message::from_raw(Bytes::from(data.to_vec())).compress();
    let msg = Message::from_compressed(BytesMut::from(raw_data.as_ref()));
    assert!(!msg.compress_flag());
    let demsg = msg.decompress().unwrap();
    assert_eq!(Bytes::from(data.to_vec()), demsg);

    let cmp_data = compress(raw_data.clone());
    let demsg = decompress(BytesMut::from(cmp_data.as_ref())).unwrap();
    assert_eq!(raw_data, demsg);
}

pub fn fuzz_decompress(data: &[u8]) {
    let _demsg = decompress(BytesMut::from(data));
}

const RANDOM_REMOVE_ADDRS: usize = 500;

pub fn fuzz_address(data: &[u8]) {
    let mut count = {
        let mut d2 = [0u8; 8];
        if data.len() <= 8 {
            d2[0..data.len()].copy_from_slice(data);
        } else {
            d2.copy_from_slice(&data[0..8]);
        }
        usize::from_le_bytes(d2)
    };

    count = count % 500 + 500;

    // TODO
    fn new_addr(id: usize) -> AddrInfo {
        let ip = Ipv4Addr::from(((225 << 24) + id) as u32);
        let addr: Multiaddr = format!("/ip4/{}/tcp/42/p2p/{}", ip, PeerId::random().to_base58())
            .parse()
            .unwrap();
        AddrInfo::new(addr, 0, 0, 0)
    }
    let mut addr_manager: AddrManager = Default::default();
    for i in 0..count {
        addr_manager.add(new_addr(i + 1));
    }
    // randomly remove addrs
    let removed_addrs = addr_manager.fetch_random(RANDOM_REMOVE_ADDRS, |_| true);
    assert_eq!(removed_addrs.len(), RANDOM_REMOVE_ADDRS);
    for addr in &removed_addrs {
        addr_manager.remove(&addr.addr);
    }
    assert_eq!(addr_manager.count(), count - RANDOM_REMOVE_ADDRS);
    // add back removed addrs
    for addr in removed_addrs {
        addr_manager.add(addr);
    }
    let addrs = addr_manager.fetch_random(count + 1, |_| true);
    assert_eq!(addrs.len(), count);
}

const SHA256_CODE: u16 = 0x12;
const SHA256_SIZE: u8 = 32;

pub fn fuzz_peer_registry(data: &[u8]) {
    fn new_multiaddr(d: &[u8]) -> Multiaddr {
        let peer_id = PeerId::from_bytes(
            vec![vec![SHA256_CODE as u8], vec![SHA256_SIZE], d.to_vec()].concat(),
        )
        .unwrap();

        // println!("fmt: {}", peer_id.to_base58());

        format!("/ip4/127.0.0.1/tcp/43/p2p/{}", peer_id.to_base58())
            .parse::<Multiaddr>()
            .unwrap()
    }

    let remote_addr = if data.len() < 32 {
        let mut buf = [0u8; 32];
        buf[..data.len()].copy_from_slice(data);
        new_multiaddr(&buf)
    } else {
        new_multiaddr(&data[..32])
    };

    let addrs = {
        if data.len() <= 32 {
            vec![new_multiaddr(&[0u8; 32])]
        } else {
            let mut addrs = Vec::<[u8; 32]>::new();
            let index = 0;
            while data.len() >= (index + 1) * 32 {

            }

            addrs.iter().map(|f| new_multiaddr(f)).collect()
        }
    };

    // TODO
    // let remote_addr = new_multiaddr(&buf[..32]);
    // let addr2 = new_multiaddr(&buf[32..]);

    // if addr1 == addr2 {
    //     return;
    // }

    // let mut peer_store = PeerStore::default();
    // let whitelist_addr = addr1;
    // let session_id = 1.into();

    // // whitelist_only mode: only accept whitelist_peer
    // let mut peers = PeerRegistry::new(3, 3, true, vec![whitelist_addr.clone()]);
    // let err = peers
    //     .accept_peer(
    //         remote_addr,
    //         session_id,
    //         SessionType::Inbound,
    //         &mut peer_store,
    //     )
    //     .unwrap_err();
    // assert_eq!(
    //     format!("{err}"),
    //     format!("{}", Error::Peer(PeerError::NonReserved))
    // );

    // peers
    //     .accept_peer(
    //         whitelist_addr,
    //         session_id,
    //         SessionType::Inbound,
    //         &mut peer_store,
    //     )
    //     .expect("accept");
}
