//! CKB launcher.
//!
//! ckb launcher is helps to launch ckb node.

use ckb_app_config::{
    BlockAssemblerConfig, ExitCode, RpcConfig, RpcModule, RunArgs, SupportProtocol,
};
use ckb_async_runtime::Handle;
use ckb_block_filter::filter::BlockFilter as BlockFilterService;
use ckb_build_info::Version;
use ckb_chain::chain::{ChainController, ChainService};
use ckb_channel::Receiver;
use ckb_jsonrpc_types::ScriptHashType;
use ckb_light_client_protocol_server::LightClientProtocol;
use ckb_logger::info;
use ckb_network::{
    observe_listen_port_occupancy, CKBProtocol, Flags, NetworkController, NetworkService,
    NetworkState, SupportProtocols,
};
use ckb_network_alert::alert_relayer::AlertRelayer;
use ckb_proposal_table::ProposalTable;
use ckb_resource::Resource;
use ckb_rpc::RpcServer;
use ckb_rpc::ServiceBuilder;
use ckb_shared::Shared;

use ckb_shared::shared_builder::{SharedBuilder, SharedPackage};
use ckb_store::{ChainDB, ChainStore};
use ckb_sync::{BlockFilter, NetTimeProtocol, Relayer, SyncShared, Synchronizer};
use ckb_tx_pool::service::TxVerificationResult;
use ckb_types::prelude::*;
use ckb_verification::GenesisVerifier;
use ckb_verification_traits::Verifier;
use std::sync::Arc;

const SECP256K1_BLAKE160_SIGHASH_ALL_ARG_LEN: usize = 20;

/// Ckb launcher is helps to launch ckb node.
pub struct Launcher {
    /// cli `run` subcommand parsed args
    pub args: RunArgs,
    /// ckb node version
    pub version: Version,
    /// ckb global runtime handle
    pub async_handle: Handle,
}

impl Launcher {
    /// Construct new Launcher from cli args
    pub fn new(args: RunArgs, version: Version, async_handle: Handle) -> Self {
        Launcher {
            args,
            version,
            async_handle,
        }
    }

    /// Sanitize block assembler config
    pub fn sanitize_block_assembler_config(
        &self,
    ) -> Result<Option<BlockAssemblerConfig>, ExitCode> {
        let block_assembler_config = match (
            self.args.config.rpc.miner_enable(),
            self.args.config.block_assembler.clone(),
        ) {
            (true, Some(mut block_assembler)) => {
                let check_lock_code_hash = |code_hash| -> Result<bool, ExitCode> {
                    let secp_cell_data =
                        Resource::bundled("specs/cells/secp256k1_blake160_sighash_all".to_string())
                            .get()
                            .map_err(|err| {
                                eprintln!(
                                    "Load specs/cells/secp256k1_blake160_sighash_all error: {err:?}"
                                );
                                ExitCode::Failure
                            })?;
                    let genesis_cellbase = &self.args.consensus.genesis_block().transactions()[0];
                    Ok(genesis_cellbase
                        .outputs()
                        .into_iter()
                        .zip(genesis_cellbase.outputs_data())
                        .any(|(output, data)| {
                            data.raw_data() == secp_cell_data.as_ref()
                                && output
                                    .type_()
                                    .to_opt()
                                    .map(|script| script.calc_script_hash())
                                    .as_ref()
                                    == Some(code_hash)
                        }))
                };
                if self.args.block_assembler_advanced
                    || (block_assembler.hash_type == ScriptHashType::Type
                        && block_assembler.args.len() == SECP256K1_BLAKE160_SIGHASH_ALL_ARG_LEN
                        && check_lock_code_hash(&block_assembler.code_hash.pack())?)
                {
                    if block_assembler.use_binary_version_as_message_prefix {
                        block_assembler.binary_version = self.version.long();
                    }
                    Some(block_assembler)
                } else {
                    info!(
                        "Miner is disabled because block assembler uses a non-recommended lock format. \
                         Edit ckb.toml or use `ckb run --ba-advanced` for other lock scripts"
                    );

                    None
                }
            }

            _ => {
                info!("Miner is disabled; edit ckb.toml to enable it");

                None
            }
        };
        Ok(block_assembler_config)
    }

    fn write_chain_spec_hash(&self, store: &ChainDB) -> Result<(), ExitCode> {
        store
            .put_chain_spec_hash(&self.args.chain_spec_hash)
            .map_err(|err| {
                eprintln!(
                    "store.put_chain_spec_hash {} error: {}",
                    self.args.chain_spec_hash, err
                );
                ExitCode::IO
            })
    }

    // internal check
    // panic immediately if migration_version is none
    fn assert_migrate_version_is_some(&self, shared: &Shared) {
        let store = shared.store();
        assert!(store.get_migration_version().is_some());
    }

    fn check_spec(&self, shared: &Shared) -> Result<(), ExitCode> {
        let store = shared.store();
        let stored_spec_hash = store.get_chain_spec_hash();

        if stored_spec_hash.is_none() {
            // fresh yet
            self.write_chain_spec_hash(store)?;
            info!("Touch chain spec hash: {}", self.args.chain_spec_hash);
        } else if stored_spec_hash.as_ref() == Some(&self.args.chain_spec_hash) {
            // stored == configured
            // do nothing
        } else if self.args.overwrite_chain_spec {
            // stored != configured with --overwrite-spec
            self.write_chain_spec_hash(store)?;
            info!(
                "Overwrite chain spec hash from {} to {}",
                stored_spec_hash.expect("checked"),
                self.args.overwrite_chain_spec,
            );
        } else if self.args.skip_chain_spec_check {
            // stored != configured with --skip-spec-check
            // do nothing
        } else {
            // stored != configured
            eprintln!(
                "chain_spec_hash mismatch: Config({}), storage({}). \
                    If the two chains are compatible, pass command line argument --skip-spec-check; \
                    otherwise, pass --overwrite-spec to enforce overriding the stored chain spec with the configured one.",
                self.args.chain_spec_hash, stored_spec_hash.expect("checked")
            );
            return Err(ExitCode::Config);
        }
        Ok(())
    }

    fn verify_genesis(&self, shared: &Shared) -> Result<(), ExitCode> {
        GenesisVerifier::new()
            .verify(shared.consensus())
            .map_err(|err| {
                eprintln!("genesis error: {err}");
                ExitCode::Config
            })
    }

    /// Build shared
    pub fn build_shared(
        &self,
        block_assembler_config: Option<BlockAssemblerConfig>,
    ) -> Result<(Shared, SharedPackage), ExitCode> {
        self.async_handle.block_on(observe_listen_port_occupancy(
            &self.args.config.network.listen_addresses,
        ))?;

        let shared_builder = SharedBuilder::new(
            &self.args.config.bin_name,
            self.args.config.root_dir.as_path(),
            &self.args.config.db,
            Some(self.args.config.ancient.clone()),
            self.async_handle.clone(),
            self.args.consensus.clone(),
        )?;

        let (shared, pack) = shared_builder
            .tx_pool_config(self.args.config.tx_pool.clone())
            .notify_config(self.args.config.notify.clone())
            .store_config(self.args.config.store)
            .block_assembler_config(block_assembler_config)
            .build()?;

        // internal check migrate_version
        self.assert_migrate_version_is_some(&shared);

        // Verify genesis every time starting node
        self.verify_genesis(&shared)?;
        self.check_spec(&shared)?;

        Ok((shared, pack))
    }

    /// Check whether the data already exists in the database before starting
    pub fn check_assume_valid_target(&mut self, shared: &Shared) {
        if let Some(ref target) = self.args.config.network.sync.assume_valid_target {
            if shared.snapshot().block_exists(&target.pack()) {
                info!("assume valid target is already in db, CKB will do full verification from now on");
                self.args.config.network.sync.assume_valid_target.take();
            }
        }
    }

    /// Start chain service, return ChainController
    pub fn start_chain_service(&self, shared: &Shared, table: ProposalTable) -> ChainController {
        let chain_service = ChainService::new(shared.clone(), table);
        let chain_controller = chain_service.start(Some("ChainService"));
        info!("chain genesis hash: {:#x}", shared.genesis_hash());
        chain_controller
    }

    fn adjust_rpc_config(&self) -> RpcConfig {
        let mut config = self.args.config.rpc.clone();
        if self.args.indexer && !config.indexer_enable() {
            config.modules.push(RpcModule::Indexer);
        }
        if self.args.rich_indexer && !config.rich_indexer_enable() {
            config.modules.push(RpcModule::RichIndexer);
        }
        config
    }

    /// Check indexer config
    pub fn check_indexer_config(&self) -> Result<(), ExitCode> {
        // check if indexer and rich-indexer are both set
        if (self.args.indexer || self.args.config.rpc.indexer_enable())
            && (self.args.rich_indexer || self.args.config.rpc.rich_indexer_enable())
        {
            eprintln!("Config Error: indexer and rich-indexer cannot be both set");
            return Err(ExitCode::Config);
        }
        Ok(())
    }

    /// start block filter service
    pub fn start_block_filter(&self, shared: &Shared) {
        if self
            .args
            .config
            .network
            .support_protocols
            .contains(&SupportProtocol::Filter)
        {
            BlockFilterService::new(shared.clone()).start();
        }
    }

    /// Start network service and rpc serve
    pub fn start_network_and_rpc(
        &self,
        shared: &Shared,
        chain_controller: ChainController,
        miner_enable: bool,
        relay_tx_receiver: Receiver<TxVerificationResult>,
    ) -> NetworkController {
        let sync_shared = Arc::new(SyncShared::with_tmpdir(
            shared.clone(),
            self.args.config.network.sync.clone(),
            self.args.config.tmp_dir.as_ref(),
            relay_tx_receiver,
        ));
        let fork_enable = {
            let epoch = shared.snapshot().tip_header().epoch().number();
            shared
                .consensus()
                .hardfork_switch
                .ckb2023
                .is_vm_version_2_and_syscalls_3_enabled(epoch)
        };
        let network_state = Arc::new(
            NetworkState::from_config(self.args.config.network.clone())
                .map(|t| NetworkState::ckb2023(t, fork_enable))
                .expect("Init network state failed"),
        );

        // Sync is a core protocol, user cannot disable it via config
        let synchronizer = Synchronizer::new(chain_controller.clone(), Arc::clone(&sync_shared));
        let mut protocols = vec![CKBProtocol::new_with_support_protocol(
            SupportProtocols::Sync,
            Box::new(synchronizer),
            Arc::clone(&network_state),
        )];

        let support_protocols = &self.args.config.network.support_protocols;
        let mut flags = Flags::all();

        if support_protocols.contains(&SupportProtocol::Relay) {
            let relayer = Relayer::new(chain_controller.clone(), Arc::clone(&sync_shared));

            protocols.push(CKBProtocol::new_with_support_protocol(
                SupportProtocols::RelayV3,
                Box::new(relayer.clone().v3()),
                Arc::clone(&network_state),
            ));
            if !fork_enable {
                protocols.push(CKBProtocol::new_with_support_protocol(
                    SupportProtocols::RelayV2,
                    Box::new(relayer),
                    Arc::clone(&network_state),
                ))
            }
        } else {
            flags.remove(Flags::RELAY);
        }

        if support_protocols.contains(&SupportProtocol::Filter) {
            let filter = BlockFilter::new(Arc::clone(&sync_shared));

            protocols.push(CKBProtocol::new_with_support_protocol(
                SupportProtocols::Filter,
                Box::new(filter),
                Arc::clone(&network_state),
            ));
        } else {
            flags.remove(Flags::BLOCK_FILTER);
        }

        if support_protocols.contains(&SupportProtocol::Time) {
            let net_timer = NetTimeProtocol::default();
            protocols.push(CKBProtocol::new_with_support_protocol(
                SupportProtocols::Time,
                Box::new(net_timer),
                Arc::clone(&network_state),
            ));
        }

        if support_protocols.contains(&SupportProtocol::LightClient) {
            let light_client = LightClientProtocol::new(shared.clone());
            protocols.push(CKBProtocol::new_with_support_protocol(
                SupportProtocols::LightClient,
                Box::new(light_client),
                Arc::clone(&network_state),
            ));
        } else {
            flags.remove(Flags::LIGHT_CLIENT);
        }

        let alert_signature_config = self.args.config.alert_signature.clone().unwrap_or_default();
        let alert_relayer = AlertRelayer::new(
            self.version.short(),
            shared.notify_controller().clone(),
            alert_signature_config,
        );

        let alert_notifier = Arc::clone(alert_relayer.notifier());
        let alert_verifier = Arc::clone(alert_relayer.verifier());
        if support_protocols.contains(&SupportProtocol::Alert) {
            protocols.push(CKBProtocol::new_with_support_protocol(
                SupportProtocols::Alert,
                Box::new(alert_relayer),
                Arc::clone(&network_state),
            ));
        }

        let required_protocol_ids = vec![SupportProtocols::Sync.protocol_id()];

        let network_controller = NetworkService::new(
            Arc::clone(&network_state),
            protocols,
            required_protocol_ids,
            (
                shared.consensus().identify_name(),
                self.version.to_string(),
                flags,
            ),
        )
        .start(shared.async_handle())
        .expect("Start network service failed");

        let rpc_config = self.adjust_rpc_config();
        let mut builder = ServiceBuilder::new(&rpc_config)
            .enable_chain(shared.clone())
            .enable_pool(
                shared.clone(),
                rpc_config
                    .extra_well_known_lock_scripts
                    .iter()
                    .map(|script| script.clone().into())
                    .collect(),
                rpc_config
                    .extra_well_known_type_scripts
                    .iter()
                    .map(|script| script.clone().into())
                    .collect(),
            )
            .enable_miner(
                shared.clone(),
                network_controller.clone(),
                chain_controller.clone(),
                miner_enable,
            )
            .enable_net(network_controller.clone(), sync_shared)
            .enable_stats(shared.clone(), Arc::clone(&alert_notifier))
            .enable_experiment(shared.clone())
            .enable_integration_test(shared.clone(), network_controller.clone(), chain_controller)
            .enable_alert(alert_verifier, alert_notifier, network_controller.clone())
            .enable_indexer(
                shared.clone(),
                &self.args.config.db,
                &self.args.config.indexer,
            )
            .enable_debug();
        builder.enable_subscription(shared.clone());
        let io_handler = builder.build();

        let async_handle = shared.async_handle();
        let _rpc = RpcServer::new(rpc_config, io_handler, async_handle.clone());

        network_controller
    }
}
