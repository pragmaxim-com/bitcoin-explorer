use crate::model::{Block, BlockHash, BlockHeader, BlockHeight, InputPointer, InputRef, Transaction, TxPointer};
use chain_syncer::api::*;
use redbit::*;
use std::sync::Arc;
use redbit::redb::ReadTransaction;

pub struct BtcBlockPersistence {
    pub db: Arc<redb::Database>,
}

impl BtcBlockPersistence {
    fn populate_inputs(read_tx: &ReadTransaction, block: &mut Block) -> Result<(), ChainSyncError> {
        for tx in &mut block.transactions {
            for transient_input in tx.transient_inputs.iter_mut() {
                let tx_pointers =
                    Transaction::get_ids_by_hash(read_tx, &transient_input.tx_hash)?;

                match tx_pointers.first() {
                    Some(tx_pointer) => {
                        tx.inputs.push(InputRef {
                            id: InputPointer::from_parent(tx_pointer.clone(), transient_input.index as u16),
                        });
                    }
                    None => {
                        // coinbase tx
                        tx.inputs.push(InputRef {
                            id: InputPointer::from_parent(TxPointer::from_parent(BlockHeight(0), 0), 0)
                        })
                    }
                }
            }
        }
        Ok(())
    }
}

impl BlockPersistence<Block> for BtcBlockPersistence {
    fn get_last_header(&self) -> Result<Option<BlockHeader>, ChainSyncError> {
        let read_tx = self.db.begin_read()?;
        let last = BlockHeader::last(&read_tx)?;
        Ok(last)
    }

    fn get_header_by_hash(&self, hash: [u8; 32]) -> Result<Vec<BlockHeader>, ChainSyncError> {
        let read_tx = self.db.begin_read()?;
        let header = BlockHeader::get_by_hash(&read_tx, &BlockHash(hash))?;
        Ok(header)
    }

    fn store_blocks(&self, mut blocks: Vec<Block>) -> Result<(), ChainSyncError> {
        for block in &mut blocks {
            {
                let read_tx = self.db.begin_read()?;
                Self::populate_inputs(&read_tx, block)?;
            }
            {
                let write_ins = self.db.begin_write()?;
                Block::store(&write_ins, block)?;
                write_ins.commit()?;
            }
        }
        Ok(())
    }

    fn update_blocks(&self, mut blocks: Vec<Block>) -> Result<(), ChainSyncError> {
        let write_tx = self.db.begin_write()?;
        for block in &mut blocks {
            Block::delete(&write_tx, &block.id)?;
        }
        write_tx.commit()?;
        self.store_blocks(blocks)?;
        Ok(())
    }
}
