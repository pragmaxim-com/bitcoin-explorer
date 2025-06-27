use crate::block_processor::BtcBlockProcessor;
use crate::btc_client::BtcClient;
use crate::config::BitcoinConfig;
use crate::model::{Block, BlockHeader, BlockHeight, ExplorerError};
use async_trait::async_trait;
use chain_syncer::api::{BlockProcessor, BlockProvider, ChainSyncError};
use chain_syncer::info;
use chain_syncer::model::BatchWeight;
use futures::Stream;
use futures::stream::StreamExt;
use min_batch::ext::MinBatchExt;
use std::{pin::Pin, sync::Arc};

pub struct BtcBlockProvider {
    pub client: Arc<BtcClient>,
    pub processor: Arc<BtcBlockProcessor>,
}

impl BtcBlockProvider {
    pub fn new(bitcoin_config: &BitcoinConfig) -> Result<Self, ExplorerError> {
        Ok(BtcBlockProvider { client: Arc::new(BtcClient::new(bitcoin_config)?), processor: Arc::new(BtcBlockProcessor {}) })
    }
}

#[async_trait]
impl BlockProvider<Block> for BtcBlockProvider {
    fn get_processed_block(&self, header: BlockHeader) -> Result<Block, ChainSyncError> {
        let block = self.client.get_block_by_hash(header.hash)?;
        self.processor.process_block(&block)
    }

    async fn get_chain_tip(&self) -> Result<BlockHeader, ChainSyncError> {
        let best_block = self.client.get_best_block()?;
        let processed_block = self.processor.process_block(&best_block)?;
        Ok(processed_block.header)
    }

    async fn stream(
        &self,
        chain_tip_header: BlockHeader,
        last_header: Option<BlockHeader>,
        min_batch_size: usize,
        fetching_par: usize,
        processing_par: usize,
    ) -> Pin<Box<dyn Stream<Item = (Vec<Block>, BatchWeight)> + Send + 'life0>> {
        let last_height = last_header.map_or(0, |h| h.id.0);
        info!("Indexing from {:?} to {:?}", last_height, chain_tip_header);
        let heights = last_height..=chain_tip_header.id.0;

        tokio_stream::iter(heights)
            .map(|height| {
                let client = Arc::clone(&self.client);
                tokio::task::spawn_blocking(move || client.get_block_by_height(BlockHeight(height)).expect("Failed to get block by height"))
            })
            .buffered(fetching_par)
            .map(|res| match res {
                Ok(block) => self.processor.process_block(&block).expect("Failed to process block"),
                Err(e) => panic!("Error: {:?}", e),
            })
            /*            .buffered(processing_par)
                        .map(|res| match res {
                            Ok(block) => block,
                            Err(e) => panic!("Error: {:?}", e),
                        })
            */
            .min_batch_with_weight(min_batch_size, |block| block.weight as usize)
            .boxed()
    }
}
