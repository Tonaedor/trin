pub mod cli;
pub mod jsonrpc;

pub use cli::PeertestConfig;
pub use jsonrpc::get_enode;

use std::sync::Arc;

use futures::future;

use trin_core::cli::TrinConfig;
use trin_core::jsonrpc::service::JsonRpcExiter;

pub struct PeertestNode {
    pub enr: String,
    pub exiter: Arc<JsonRpcExiter>,
}

pub struct AllPeertestNodes {
    pub bootnode: PeertestNode,
    pub nodes: Vec<PeertestNode>,
}

impl AllPeertestNodes {
    pub fn exit(&self) {
        self.bootnode.exiter.exit();
        self.nodes.iter().for_each(|node| node.exiter.exit());
    }
}

fn get_peertest_id_for_node(mut id: u8, bootnode_enr: Option<&String>) -> u16 {
    // Peertest Id for bootnode is 1 (identified by bootnode_enr == None)
    // For all other nodes (identified by bootnode_enr == Some()) Peertest ID needs to be
    // incremented by one to account for the bootnode
    if bootnode_enr.is_some() {
        id += 1;
    }
    id as u16
}

pub async fn launch_node(id: u8, bootnode_enr: Option<&String>) -> anyhow::Result<PeertestNode> {
    let id = get_peertest_id_for_node(id, bootnode_enr);

    // Run a client, as a buddy peer for ping tests, etc.
    let discovery_port: u16 = 9000 + id;
    let discovery_port: String = discovery_port.to_string();
    let web3_ipc_path = format!("/tmp/ethportal-peertest-buddy-{id}.ipc");
    let trin_config_args: Vec<&str> = match bootnode_enr {
        Some(enr) => vec![
            "trin",
            "--internal-ip",
            "--bootnodes",
            enr.as_str(),
            "--discovery-port",
            discovery_port.as_str(),
            "--web3-ipc-path",
            web3_ipc_path.as_str(),
        ],
        None => vec![
            "trin",
            "--internal-ip",
            "--discovery-port",
            discovery_port.as_str(),
            "--web3-ipc-path",
            web3_ipc_path.as_str(),
        ],
    };
    let trin_config = TrinConfig::new_from(trin_config_args.iter()).unwrap();
    let web3_ipc_path = trin_config.web3_ipc_path.clone();
    let exiter = trin::run_trin(trin_config, String::new()).await.unwrap();
    let enr = get_enode(&web3_ipc_path)?;

    Ok(PeertestNode { enr, exiter })
}

pub async fn launch_peertest_nodes(count: u8) -> AllPeertestNodes {
    let bootnode = launch_node(1, None).await.unwrap();
    let nodes = future::try_join_all(
        (1..count)
            .into_iter()
            .map(|id| launch_node(id, Some(&bootnode.enr))),
    )
    .await
    .unwrap();
    AllPeertestNodes { bootnode, nodes }
}
