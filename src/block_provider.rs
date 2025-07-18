use crate::btc_client::{BtcBlock, BtcClient};
use crate::config::BitcoinConfig;
use crate::model::{
    Address, Block, BlockHash, BlockHeader, BlockHeight, BlockTimestamp, ExplorerError, ScriptHash, TempInputRef, Transaction, TxHash, TxPointer,
    Utxo, UtxoPointer,
};
use async_trait::async_trait;
use chain_syncer::api::{BlockProvider, ChainSyncError};
use chain_syncer::info;
use chain_syncer::monitor::BoxWeight;
use futures::stream::StreamExt;
use futures::Stream;
use redbit::*;
use std::{pin::Pin, sync::Arc};

pub struct BtcBlockProvider {
    pub client: Arc<BtcClient>,
    pub fetching_par: usize,
}

impl BtcBlockProvider {
    pub fn new(bitcoin_config: &BitcoinConfig, fetching_par: usize) -> Result<Self, ExplorerError> {
        Ok(BtcBlockProvider { client: Arc::new(BtcClient::new(bitcoin_config)?), fetching_par })
    }
    fn process_inputs(&self, ins: &[bitcoin::TxIn]) -> Vec<TempInputRef> {
        ins.iter()
            .map(|input| {
                let tx_hash = TxHash(*input.previous_output.txid.as_ref());
                TempInputRef { tx_hash, index: input.previous_output.vout }
            })
            .collect()
    }
    fn process_outputs(&self, outs: &[bitcoin::TxOut], tx_pointer: TxPointer) -> (BoxWeight, Vec<Utxo>) {
        let mut result_outs = Vec::with_capacity(outs.len());
        for (out_index, out) in outs.iter().enumerate() {
            let address_opt = if let Ok(address) = bitcoin::Address::from_script(out.script_pubkey.as_script(), bitcoin::Network::Bitcoin) {
                Some(address.to_string().into_bytes())
            } else {
                out.script_pubkey
                    .p2pk_public_key()
                    .map(|pk| bitcoin::Address::p2pkh(pk.pubkey_hash(), bitcoin::Network::Bitcoin).to_string().into_bytes())
            };
            result_outs.push(Utxo {
                id: UtxoPointer::from_parent(tx_pointer.clone(), out_index as u16),
                amount: out.value.to_sat().into(),
                script_hash: ScriptHash(out.script_pubkey.as_bytes().to_vec()),
                address: Address(address_opt.unwrap_or_default()),
            })
        }
        (result_outs.len(), result_outs)
    }
    fn process_tx(&self, height: BlockHeight, tx_index: u16, tx: &bitcoin::Transaction) -> Transaction {
        let tx_pointer = TxPointer::from_parent(height, tx_index);
        let (_, outputs) = self.process_outputs(&tx.output, tx_pointer.clone());
        Transaction {
            id: tx_pointer.clone(),
            hash: TxHash(*tx.compute_txid().as_ref()),
            utxos: outputs,
            inputs: vec![],
            transient_inputs: self.process_inputs(&tx.input),
        }
    }
}

#[async_trait]
impl BlockProvider<BtcBlock, Block> for BtcBlockProvider {
    fn process_block(&self, block: &BtcBlock) -> Result<Block, ChainSyncError> {
        let header = BlockHeader {
            id: block.height.clone(),
            timestamp: BlockTimestamp(block.underlying.header.time),
            hash: BlockHash(*block.underlying.block_hash().as_ref()),
            prev_hash: BlockHash(*block.underlying.header.prev_blockhash.as_ref()),
        };

        let mut block_weight = 0;
        Ok(Block {
            id: block.height.clone(),
            header,
            transactions: block
                .underlying
                .txdata
                .iter()
                .enumerate()
                .map(|(tx_index, tx)| {
                    block_weight += tx.input.len() + tx.output.len();
                    self.process_tx(block.height.clone(), tx_index as u16, &tx)
                })
                .collect(),
            weight: block_weight as u32, // TODO usize
        })
    }

    fn get_processed_block(&self, header: BlockHeader) -> Result<Block, ChainSyncError> {
        let block = self.client.get_block_by_hash(header.hash)?;
        self.process_block(&block)
    }

    async fn get_chain_tip(&self) -> Result<BlockHeader, ChainSyncError> {
        let best_block = self.client.get_best_block()?;
        let processed_block = self.process_block(&best_block)?;
        Ok(processed_block.header)
    }

    async fn stream(
        &self,
        chain_tip_header: BlockHeader,
        last_header: Option<BlockHeader>,
    ) -> Pin<Box<dyn Stream<Item = BtcBlock> + Send + 'life0>> {
        let last_height = last_header.map_or(0, |h| h.id.0);
        info!("Indexing from {:?} to {:?}", last_height, chain_tip_header);
        let heights = last_height..=chain_tip_header.id.0;
        tokio_stream::iter(heights)
            .map(|height| {
                let client = Arc::clone(&self.client);
                tokio::task::spawn_blocking(move || client.get_block_by_height(BlockHeight(height)).expect("Failed to get block by height"))
            })
            .buffered(self.fetching_par)
            .map(|res| match res {
                Ok(block) => block,
                Err(e) => panic!("Error: {:?}", e), // TODO lousy error handling
            })
            .boxed()
    }
}
