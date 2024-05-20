use crate::{
    multiaddr::Multiaddr,
    peer_store::{addr_manager::AddrManager, types::AddrInfo},
    PeerId,
};
use proptest::prelude::*;
use std::net::Ipv4Addr;

const MAX_FETCHED_ADDRS: usize = 1000;
const RANDOM_REMOVE_ADDRS: usize = 500;
proptest! {
    #[test]
    fn test_add_random_addrs(count in RANDOM_REMOVE_ADDRS..MAX_FETCHED_ADDRS) {
        fn new_addr(id: usize) -> AddrInfo{
            let ip = Ipv4Addr::from(((225 << 24) + id) as u32);
            let addr: Multiaddr = format!("/ip4/{}/tcp/42/p2p/{}", ip, PeerId::random().to_base58()).parse().unwrap();
            AddrInfo::new(
                addr,
                0,
                0,
                0
            )
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
}

#[test]
fn test_eee() {
    let mut addr_manager = AddrManager::default();

    let addr1 = {
        let peer_id =
            PeerId::from_bytes(vec![vec![0x12], vec![0x20], vec![1u8; 32]].concat()).unwrap();
        AddrInfo::new(
            format!("/ip4/127.0.0.1/tcp/42/p2p/{}", peer_id.to_base58())
                .parse()
                .unwrap(),
            0,
            0,
            0,
        )
    };
    let addr2 = {
        let peer_id =
            PeerId::from_bytes(vec![vec![0x12], vec![0x20], vec![1u8; 32]].concat()).unwrap();
        AddrInfo::new(
            format!("/ip4/127.0.0.1/tcp/43/p2p/{}", peer_id.to_base58())
                .parse()
                .unwrap(),
            0,
            0,
            0,
        )
    };

    addr_manager.add(addr1.clone());
    addr_manager.add(addr2.clone());
    assert_eq!(2, addr_manager.count());
    
    addr_manager.remove(&addr1.addr);
    assert_eq!(1, addr_manager.count());
}
