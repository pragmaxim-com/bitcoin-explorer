use crate::config::BitcoinConfig;
use crate::model::{BlockHash, BlockHeight, ExplorerError};
use bitcoin::hashes::Hash;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use std::sync::Arc;

// Bitcoin block wrapper
#[derive(Debug, Clone)]
pub struct BtcBlock {
    pub height: BlockHeight,
    pub underlying: bitcoin::Block,
}

pub struct BtcClient {
    rpc_client: Arc<Client>,
}

impl BtcClient {
    pub fn new(bitcoin_config: &BitcoinConfig) -> Result<Self, ExplorerError> {
        let user_pass = Auth::UserPass(bitcoin_config.api_username.to_string(), bitcoin_config.api_password.to_string());
        let client = Client::new(bitcoin_config.api_host.as_str(), user_pass)?;
        let rpc_client = Arc::new(client);
        Ok(BtcClient { rpc_client })
    }
}

impl BtcClient {
    pub fn get_best_block(&self) -> Result<BtcBlock, ExplorerError> {
        let best_block_hash = self.rpc_client.get_best_block_hash()?;
        let best_block = self.rpc_client.get_block(&best_block_hash)?;
        let height = self.get_block_height(&best_block)?;
        Ok(BtcBlock { height, underlying: best_block })
    }

    pub fn get_block_by_hash(&self, hash: BlockHash) -> Result<BtcBlock, ExplorerError> {
        let bitcoin_hash = bitcoin::BlockHash::from_raw_hash(Hash::from_byte_array(hash.0));
        let block = self.rpc_client.get_block(&bitcoin_hash)?;
        let height = self.get_block_height(&block)?;
        Ok(BtcBlock { height, underlying: block })
    }

    pub fn get_block_by_height(&self, height: BlockHeight) -> Result<BtcBlock, ExplorerError> {
        let block_hash = self.rpc_client.get_block_hash(height.0 as u64)?;
        let block = self.rpc_client.get_block(&block_hash)?;
        Ok(BtcBlock { height, underlying: block })
    }

    fn get_block_height(&self, block: &bitcoin::Block) -> Result<BlockHeight, ExplorerError> {
        // Try to get height using fast method (BIP34)
        if let Ok(height) = block.bip34_block_height() {
            return Ok(BlockHeight(height as u32));
        }
        // Fallback to fetching block header for height
        let block_hash = block.block_hash();
        let verbose_block = self.rpc_client.get_block_info(&block_hash)?;
        Ok(BlockHeight(verbose_block.height as u32))
    }
}
