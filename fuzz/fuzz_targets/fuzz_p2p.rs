#![no_main]

use libfuzzer_sys::fuzz_target;

use ckb_app_config::NetworkAlertConfig;
use ckb_fuzz::BufManager;
use ckb_network::{async_trait, bytes::Bytes, PeerIndex, SupportProtocols};
use ckb_shared::Shared;
use ckb_sync::SyncShared;
use futures::Future;
use std::{pin::Pin, sync::Arc, time::Duration};
use tokio::runtime::{Handle, Runtime};

struct TestProtocolCtx {
    protocol: ckb_network::ProtocolId,
}

#[async_trait]
impl ckb_network::CKBProtocolContext for TestProtocolCtx {
    fn ckb2023(&self) -> bool {
        false
    }
    async fn set_notify(&self, _interval: Duration, _token: u64) -> Result<(), ckb_network::Error> {
        Ok(())
    }
    /// Remove notify
    async fn remove_notify(&self, _token: u64) -> Result<(), ckb_network::Error> {
        Ok(())
    }
    /// Send message through quick queue
    async fn async_quick_send_message(
        &self,
        _proto_id: ckb_network::ProtocolId,
        _peer_index: PeerIndex,
        _data: Bytes,
    ) -> Result<(), ckb_network::Error> {
        Ok(())
    }
    /// Send message through quick queue
    async fn async_quick_send_message_to(
        &self,
        _peer_index: PeerIndex,
        _data: Bytes,
    ) -> Result<(), ckb_network::Error> {
        Ok(())
    }
    /// Filter broadcast message through quick queue
    async fn async_quick_filter_broadcast(
        &self,
        _target: ckb_network::TargetSession,
        _data: Bytes,
    ) -> Result<(), ckb_network::Error> {
        Ok(())
    }
    /// spawn a future task, if `blocking` is true we use tokio_threadpool::blocking to handle the task.
    async fn async_future_task(
        &self,
        _task: Pin<Box<dyn Future<Output = ()> + 'static + Send>>,
        _blocking: bool,
    ) -> Result<(), ckb_network::Error> {
        Ok(())
    }
    /// Send message
    async fn async_send_message(
        &self,
        _proto_id: ckb_network::ProtocolId,
        _peer_index: PeerIndex,
        _data: Bytes,
    ) -> Result<(), ckb_network::Error> {
        Ok(())
    }
    /// Send message
    async fn async_send_message_to(
        &self,
        _peer_index: PeerIndex,
        _data: Bytes,
    ) -> Result<(), ckb_network::Error> {
        Ok(())
    }
    /// Filter broadcast message
    async fn async_filter_broadcast(
        &self,
        _target: ckb_network::TargetSession,
        _data: Bytes,
    ) -> Result<(), ckb_network::Error> {
        Ok(())
    }
    /// Disconnect session
    async fn async_disconnect(
        &self,
        _peer_index: PeerIndex,
        _message: &str,
    ) -> Result<(), ckb_network::Error> {
        Ok(())
    }
    /// Send message through quick queue
    fn quick_send_message(
        &self,
        _proto_id: ckb_network::ProtocolId,
        _peer_index: PeerIndex,
        _data: Bytes,
    ) -> Result<(), ckb_network::Error> {
        Ok(())
    }
    /// Send message through quick queue
    fn quick_send_message_to(
        &self,
        _peer_index: PeerIndex,
        _data: Bytes,
    ) -> Result<(), ckb_network::Error> {
        Ok(())
    }
    /// Filter broadcast message through quick queue
    fn quick_filter_broadcast(
        &self,
        _target: ckb_network::TargetSession,
        _data: Bytes,
    ) -> Result<(), ckb_network::Error> {
        Ok(())
    }
    /// spawn a future task, if `blocking` is true we use tokio_threadpool::blocking to handle the task.
    fn future_task(
        &self,
        _task: Pin<Box<dyn Future<Output = ()> + 'static + Send>>,
        _blocking: bool,
    ) -> Result<(), ckb_network::Error> {
        //        task.await.expect("resolve future task ckb_network::Error");
        Ok(())
    }
    /// Send message
    fn send_message(
        &self,
        _proto_id: ckb_network::ProtocolId,
        _peer_index: PeerIndex,
        _data: Bytes,
    ) -> Result<(), ckb_network::Error> {
        Ok(())
    }
    /// Send message
    fn send_message_to(
        &self,
        _peer_index: PeerIndex,
        _data: Bytes,
    ) -> Result<(), ckb_network::Error> {
        Ok(())
    }
    /// Filter broadcast message
    fn filter_broadcast(
        &self,
        _target: ckb_network::TargetSession,
        _data: Bytes,
    ) -> Result<(), ckb_network::Error> {
        Ok(())
    }
    /// Disconnect session
    fn disconnect(&self, _peer_index: PeerIndex, _message: &str) -> Result<(), ckb_network::Error> {
        Ok(())
    }
    // Interact with NetworkState
    /// Get peer info
    fn get_peer(&self, _peer_index: PeerIndex) -> Option<ckb_network::Peer> {
        None
    }
    /// Modify peer info
    fn with_peer_mut(&self, _peer_index: PeerIndex, _f: Box<dyn FnOnce(&mut ckb_network::Peer)>) {}
    /// Get all session id
    fn connected_peers(&self) -> Vec<PeerIndex> {
        Vec::new()
    }
    /// Report peer behavior
    fn report_peer(&self, _peer_index: PeerIndex, _behaviour: ckb_network::Behaviour) {}
    /// Ban peer
    fn ban_peer(&self, _peer_index: PeerIndex, _duration: Duration, _reason: String) {}
    /// current protocol id
    fn protocol_id(&self) -> ckb_network::ProtocolId {
        self.protocol
    }
}

fn get_proto_type(data: &mut BufManager) -> Result<SupportProtocols, ()> {
    if data.is_end() {
        return Err(());
    }
    let id = data.get::<u8>();

    // SupportProtocols::Sync => 100,
    // SupportProtocols::RelayV2 => 101,
    // SupportProtocols::RelayV3 => 103,
    // SupportProtocols::Time => 102,
    // SupportProtocols::Alert => 110,
    // SupportProtocols::LightClient => 120,
    // SupportProtocols::Filter => 121,

    match id {
        100 => Ok(SupportProtocols::Sync),
        101 => Ok(SupportProtocols::RelayV2),
        103 => Ok(SupportProtocols::RelayV3),
        102 => Ok(SupportProtocols::Time),
        110 => Ok(SupportProtocols::Alert),
        120 => Ok(SupportProtocols::LightClient),
        121 => Ok(SupportProtocols::Filter),

        _ => Err(()),
    }
}

fn get_shared(data: &mut BufManager, handle: &Handle) -> Result<Shared, ()> {
    if data.is_end() {
        return Err(());
    }
    let builder = ckb_shared::shared_builder::SharedBuilder::with_temp_db();
    // let builder = ckb_shared::shared_builder::SharedBuilder::new(
    //     "ckb",
    //     std::path::Path::new("./"),
    //     &ckb_app_config::DBConfig::default(),
    //     None,
    //     ckb_async_runtime::Handle::new(handle.clone(), None),
    //     ckb_chain_spec::consensus::Consensus::default(),
    // )
    // .unwrap();

    let r = builder.build();

    if r.is_err() {
        return Err(());
    }

    Ok(r.unwrap().0)
}

fn get_sync_shared(data: &mut BufManager, handle: &Handle) -> Result<SyncShared, ()> {
    if data.is_end() {
        return Err(());
    }

    let shared = get_shared(data, handle)?;

    let sync_config = ckb_app_config::SyncConfig::default();
    let (_, relay_tx_receiver) = ckb_channel::bounded(0);

    Ok(SyncShared::new(
        shared.clone(),
        sync_config,
        relay_tx_receiver,
    ))
}

fn get_version(data: &mut BufManager) -> Result<ckb_build_info::Version, ()> {
    if data.is_end() {
        return Err(());
    }

    let mut ver = ckb_build_info::Version::default();
    ver.major = data.get();
    ver.minor = data.get();
    ver.patch = data.get();
    Ok(ver)
}

fn get_network_alert_config(data: &mut BufManager) -> Result<NetworkAlertConfig, ()> {
    if data.is_end() {
        return Err(());
    }

    let cfg = NetworkAlertConfig::default();
    Ok(cfg)
}

fn run(data: &[u8]) -> Result<(), ()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    let mut data = BufManager::new(data);

    let t = get_proto_type(&mut data)?;

    let sync_shared = match t {
        SupportProtocols::Time => None,
        _ => Some(Arc::new(get_sync_shared(&mut data, rt.handle())?)),
    };

    let (version, alert_cfg) = match t {
        SupportProtocols::Alert => (
            Some(get_version(&mut data)?),
            Some(get_network_alert_config(&mut data)?),
        ),
        _ => (None, None),
    };

    let proto = ckb_launcher::new_ckb_protocol(t, sync_shared, version, alert_cfg);
    if proto.is_none() {
        return Err(());
    }
    let mut proto = proto.unwrap();

    rt.block_on(async {
        let nc = Arc::new(TestProtocolCtx { protocol: 0.into() });

        let _r = proto.init(nc.clone()).await;
        proto.connected(nc.clone(), 0.into(), "").await;
        proto
            .received(nc.clone(), 0.into(), Bytes::from(data.other()))
            .await;
        proto.disconnected(nc, 0.into()).await;
    });
    Ok(())
}

fuzz_target!(|data: &[u8]| {
    let _r = run(data);
});
