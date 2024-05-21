#![no_main]

use libfuzzer_sys::fuzz_target;

use ckb_network::peer_store::PeerStore;
use ckb_network::{multiaddr::MultiAddr, Flags, PeerId};
use rand::{thread_rng, RngCore};

fn get_buf(len: usize, data: &[u8], offset: &mut usize) -> Vec<u8> {
    if data.len() >= *offset + len && *offset != data.len() {
        let r = data[*offset..*offset + len].to_vec();
        *offset += len;
        r
    } else {
        let mut r = Vec::<u8>::with_capacity(len);
        r.resize(len, 0);
        r[0..(data.len() - *offset)].copy_from_slice(&data[*offset..]);
        *offset = data.len();
        r
    }
}

fn new_multi_addr(data: &[u8], offset: &mut usize) -> (MultiAddr, Flags) {
    let d2 = &data[*offset..];

    let flags = unsafe { Flags::from_bits_unchecked(d2[0] as u64) };
    *offset += 1;

    let addr_flag = get_buf(1, data, offset)[0];
    let mut addr_str = if addr_flag & 0b1 != 1 {
        let buf = get_buf(16, data, offset);
        format!(
            "/ip6/{}",
            std::net::Ipv6Addr::from(u128::from_le_bytes(buf.try_into().unwrap())).to_string()
        )
    } else {
        let buf = get_buf(4, data, offset);

        format!(
            "/ip4/{}",
            std::net::Ipv4Addr::from(u32::from_le_bytes(buf.try_into().unwrap())).to_string()
        )
    };

    addr_str += {
        let buf = get_buf(2, data, offset);
        &format!("/tcp/{}", u16::from_le_bytes(buf.try_into().unwrap()))
    };

    addr_str += {
        let buf = get_buf(32, data, offset);

        &format!(
            "/p2p/{}",
            PeerId::from_bytes(vec![vec![0x12], vec![0x20], buf].concat())
                .unwrap()
                .to_base58()
        )
    };

    (addr_str.parse().unwrap(), flags)
}

fn add_basic_addr(data: &[u8], offset: &mut usize, peer_store: &mut PeerStore) {
    let num = u32::from_le_bytes(get_buf(4, data, offset).try_into().unwrap());

    let num = num % 16 + (16384);
    let mut rng = thread_rng();
    let now_ms = ckb_systemtime::unix_time_as_millis();

    for i in 0..num {
        let addr = format!(
            "/ip4/{}/tcp/43/p2p/{}",
            std::net::Ipv4Addr::from(i as u32).to_string(),
            PeerId::random().to_base58()
        )
        .parse()
        .unwrap();
        let _ = peer_store.add_addr_fuzz(
            addr,
            Flags::all(),
            now_ms - (rng.next_u32() as u64),
            rng.next_u32(),
        );
    }
}

fuzz_target!(|data: &[u8]| {
    let mut offset = 0;
    let now_ms = ckb_systemtime::unix_time_as_millis();

    let mut peer_store: PeerStore = Default::default();

    // basic addr:
    add_basic_addr(data, &mut offset, &mut peer_store);

    while offset != data.len() {
        let (addr, flag) = new_multi_addr(data, &mut offset);
        let last_connected_time =
            now_ms + u16::from_le_bytes(get_buf(2, data, &mut offset).try_into().unwrap()) as u64;
        let attempts_count = u32::from_le_bytes(get_buf(4, data, &mut offset).try_into().unwrap());
        peer_store
            .add_addr_fuzz(addr, flag, last_connected_time, attempts_count)
            .expect("msg");
    }
});
