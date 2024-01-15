use micro_config::configs::FriProverTaskApplyConfig;
use micro_dal::ConnectionPool;
use micro_web3_decl::jsonrpsee::http_client::HttpClient;

pub struct Dashboard {
    pub pool: ConnectionPool,
    pub client: HttpClient,
    pub config: FriProverTaskApplyConfig,
}
