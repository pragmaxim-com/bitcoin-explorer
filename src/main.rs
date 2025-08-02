use anyhow::Result;
use chain_syncer::api::{BlockPersistence, BlockProvider};
use chain_syncer::scheduler::Scheduler;
use chain_syncer::settings::{AppConfig, HttpSettings, IndexerSettings};
use chain_syncer::{combine, info};
use futures::future::ready;
use redbit::redb::Database;
use redbit::*;
use std::env;
use std::sync::Arc;
use tower_http::cors;
use bitcoin_explorer::block_persistence::BtcBlockPersistence;
use bitcoin_explorer::block_provider::BtcBlockProvider;
use bitcoin_explorer::btc_client::BtcBlock;
use bitcoin_explorer::config::BitcoinConfig;
use bitcoin_explorer::model::Block;
use bitcoin_explorer::storage;

async fn maybe_run_server(http_conf: &HttpSettings, db: Arc<Database>) -> () {
    if http_conf.enable {
        info!("Starting http server at {}", http_conf.bind_address);
        let cors = cors::CorsLayer::new()
            .allow_origin(cors::Any) // or use a specific origin: `AllowOrigin::exact("http://localhost:5173".parse().unwrap())`
            .allow_methods(cors::Any)
            .allow_headers(cors::Any);
        serve(RequestState { db: Arc::clone(&db) }, http_conf.bind_address, None, Some(cors)).await
    } else {
        ready(()).await
    }
}

async fn maybe_run_indexing(index_config: &IndexerSettings, scheduler: Scheduler<BtcBlock, Block>) -> () {
    if index_config.enable {
        info!("Starting indexing process");
        scheduler.schedule(&index_config).await
    } else {
        ready(()).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let app_config = AppConfig::new("config/settings")?;
    let btc_config = BitcoinConfig::new("config/bitcoin")?;
    let db_path: String = format!("{}/{}/{}", app_config.indexer.db_path, "main", "btc");
    let full_db_path = env::home_dir().unwrap().join(&db_path);
    let db = Arc::new(storage::get_db(full_db_path)?);
    let fetching_par: usize = app_config.indexer.fetching_parallelism.clone().into();

    let block_provider: Arc<dyn BlockProvider<BtcBlock, Block>> = Arc::new(BtcBlockProvider::new(&btc_config, fetching_par)?);
    let block_persistence: Arc<dyn BlockPersistence<Block>> = Arc::new(BtcBlockPersistence { db: Arc::clone(&db) });
    let scheduler: Scheduler<BtcBlock, Block> = Scheduler::new(block_provider, block_persistence);

    let indexing_f = maybe_run_indexing(&app_config.indexer, scheduler);
    let server_f = maybe_run_server(&app_config.http, Arc::clone(&db));
    combine::futures(indexing_f, server_f).await;
    Ok(())
}
