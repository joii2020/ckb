# P2P 部分模糊测试

## 引自Notion
```
- [ ]  P2P Protocol 单独拆分
    - 搜索 `impl CKBProtocolHandler` 对应的是 CKB 上层的一些协议, NetTimeProtocol, BlockFilter, Relayer, Synchronizer, LightClientProtocol, AlertRelaer。其中的核心是 Relayer 和 BlockFilter。主要接口是 CKBProtocolHandler::received
    - 搜索 `impl.*ServiceProtocol` 是底层的一些协议，包括 Feeler, Ping, Discovery, Identity，其中比较重要的是 Discovery。
```

### CKBProtocolHandler

### NetTimeProtocol
struct: `sync/src/net_time_checker.rs:109`

`pub(crate) mod net_time_checker;`
外部无法访问


### BlockFilter *
struct: `sync/src/filter/mod.rs:22`
外部无法访问
`mod filter;`


### Relayer *
struct: `sync/src/relayer/mod.rs:74`
外部无法访问
`mod relayer;`


### Synchronizer
struct: `sync/src/synchronizer/mod.rs:287`
外部无法访问
`mod synchronizer;`


### LightClientProtocol
struct: `util/light-client-protocol-server/src/lib.rs:26`
new2: `util/light-client-protocol-server/src/tests/utils/chain.rs:117`


### AlertRelayer
struct: `util/network-alert/src/alert_relayer.rs:34`


## new
几乎所有的这些都在 `util/launcher` `Launcher::start_network_and_rpc` 中被new出来的。 
可以考虑通过调用 `ckb-launcher` 进行测试。


具体使用可以参考： `ckb-bin/src/subcommand/run.rs:11` 。 仿照 `run` 使用fuzz传入的数据构造构造参数。
