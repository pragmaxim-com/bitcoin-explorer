## Bitcoin Explorer

Bitcoin explorer on top of [redbit](https://github.com/pragmaxim-com/redbit) and [chain-syncer](https://github.com/pragmaxim-com/chain-syncer)

It uses tiny `block_height/tx_index/utxo_index/[asset_index]` dictionary pointers to big hashes, ie. not a single hash is duplicated,
which allows for much better space efficiency and syncing speed with local node and an SSD.

> Note that indexing speed in logs is the **average**, the first ~ 100k blocks with just one Tx are indexed at ~ `300 Inputs+outputs+assets / second`.
> Indexing is optimized for the big blocks where the throughput reaches ~ `3 000 Inputs+outputs+assets / second` if node and indexer each uses its own SSD.

Chain tip is "eventually consistent" through fork competition, ie. forks get settled eventually and superseded forks are deleted from DB.

### Installation (Debian/Ubuntu)

```
sudo apt-get install rustup
```

### Usage

Run bitcoin node locally, rpc at port 8332 can be changed in `config/bitcoin.toml`, for example:
```
cat ~/snap/bitcoin-core/common/.bitcoin/bitcoin.conf | grep rpc
rpcthreads=40
rpcworkqueue=512
rpcuser=foo
rpcpassword=bar
rpcallowip=10.0.1.0/24
rpcport=8332
rpcbind=0.0.0.0

bitcoin-core.daemon -daemon
bitcoin-core.cli getblockchaininfo
tail -f ~/snap/bitcoin-core/common/.bitcoin/debug.log
```
export secrets or set them in `.env` file :
```
export BITCOIN__API_USERNAME="foo"
export BITCOIN__API_PASSWORD="bar"
```
Then : 
```
cargo run
```

Indexing might crash especially on laptops with Node running locally and not being synced yet.
In that case, set `fetching_parallelism = "low"` to not put the Node and Laptop under heavy pressure.

### Rest API

http://localhost:8000/swagger-ui/

Querying currently times out during historical indexing. So use it only at the chain tip sync phase
or when indexing is disabled `indexer.enable = false` and we only run http server to query over existing data.

### UI 

See [redbit-ui](https://github.com/pragmaxim-com/redbit-ui) 