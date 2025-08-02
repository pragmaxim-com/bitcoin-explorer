use chain_syncer::api::{BlockPersistence, BlockProvider};
use chain_syncer::{info, settings};
use std::{env, fs, sync::Arc, time::Duration};

use bitcoin_explorer::block_persistence::BtcBlockPersistence;
use bitcoin_explorer::block_provider::BtcBlockProvider;
use bitcoin_explorer::btc_client::{BtcBlock, BtcClient};
use bitcoin_explorer::config::BitcoinConfig;
use bitcoin_explorer::model::{Block, Height};
use bitcoin_explorer::storage;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn criterion_benchmark(c: &mut Criterion) {
    let app_config = settings::AppConfig::new("config/settings").unwrap();
    let btc_config = BitcoinConfig::new("config/bitcoin").expect("Failed to load Bitcoin configuration");
    let db_name = format!("{}/{}", "btc_indexer", "benchmark");
    let db_path = env::temp_dir().join(&db_name);
    fs::remove_dir_all(&db_path).unwrap();
    let db = Arc::new(storage::get_db(db_path).expect("Failed to open database"));

    let btc_client = BtcClient::new(&btc_config).expect("Failed to create Bitcoin client");
    let fetching_par: usize = app_config.indexer.fetching_parallelism.clone().into();
    let block_provider: Arc<dyn BlockProvider<BtcBlock, Block>> =
        Arc::new(BtcBlockProvider::new(&btc_config, fetching_par).expect("Failed to create block provider"));
    let block_persistence: Arc<dyn BlockPersistence<Block>> = Arc::new(BtcBlockPersistence { db: Arc::clone(&db) });

    info!("Initiating download");
    let batch_size = 100_000;
    let start_height = 1;
    let end_height = start_height + batch_size;
    let mut btc_blocks: Vec<BtcBlock> = Vec::with_capacity(batch_size as usize);
    for height in start_height..end_height {
        btc_blocks.push(btc_client.get_block_by_height(Height(height)).unwrap());
    }

    info!("Initiating processing");
    let mut blocks = Vec::with_capacity(btc_blocks.len());
    for block in btc_blocks.iter() {
        let b = block_provider.process_block(block).expect("Failed to process block");
        blocks.push(b);
    }

    info!("Initiating indexing");
    let mut group = c.benchmark_group("processor");
    group.throughput(Throughput::Elements(batch_size as u64));
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_millis(1000));
    group.bench_function(BenchmarkId::from_parameter("indexing"), |bencher| {
        bencher.iter(|| {
            let xs = blocks.drain(0..10).collect();
            block_persistence.store_blocks(xs)
        });
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
