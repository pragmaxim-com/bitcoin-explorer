use bitcoin_explorer::block_provider::BtcBlockProvider;
use bitcoin_explorer::btc_client::{BtcBlock, BtcClient};
use bitcoin_explorer::config::BitcoinConfig;
use bitcoin_explorer::model::{Block, Height};
use chain_syncer::api::BlockProvider;
use chain_syncer::info;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;
use std::time::Duration;

fn criterion_benchmark(c: &mut Criterion) {
    let btc_config = BitcoinConfig::new("config/bitcoin").expect("Failed to load Bitcoin configuration");

    let btc_client = BtcClient::new(&btc_config).expect("Failed to create Bitcoin client");
    info!("Initiating download");
    let batch_size = 50000;
    let start_height = 1;
    let end_height = start_height + batch_size;
    let mut blocks: Vec<BtcBlock> = Vec::with_capacity(batch_size as usize);
    for height in start_height..end_height {
        blocks.push(btc_client.get_block_by_height(Height(height)).unwrap());
    }

    let provider: Arc<dyn BlockProvider<BtcBlock, Block>> =
        Arc::new(BtcBlockProvider::new(&btc_config, 10).expect("Failed to create block provider"));

    info!("Initiating processing");
    let mut group = c.benchmark_group("processor");
    group.throughput(Throughput::Elements(batch_size as u64));
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_millis(1000));
    group.bench_function(BenchmarkId::from_parameter("processor"), |bencher| {
        bencher.iter(|| {
            blocks.pop().iter().for_each(|b| {
                provider.process_block(&b).expect("Failed to process block");
                ()
            })
        });
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
