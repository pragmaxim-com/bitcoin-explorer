use super::btc_client::BtcBlock;
use crate::model::{
    Address, Block, BlockHash, BlockHeader, BlockHeight, BlockTimestamp, ScriptHash, TempInputRef, Transaction, TxHash, TxPointer, Utxo, UtxoPointer,
};
use chain_syncer::api::{BlockProcessor, ChainSyncError};
use chain_syncer::model::BoxWeight;
pub use redbit::*;

pub struct BtcBlockProcessor {}

impl BtcBlockProcessor {
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

impl BlockProcessor<Block> for BtcBlockProcessor {
    type FromBlock = BtcBlock;

    fn process_block(&self, block: &Self::FromBlock) -> Result<Block, ChainSyncError> {
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
}
