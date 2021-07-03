use crate::{
    blockchain::Blockchain,
    burnfee::BurnFee,
    crypto::{hash, generate_random_bytes, SaitoHash, SaitoPublicKey, SaitoSignature, SaitoUTXOSetKey},
    merkle::MerkleTreeLayer,
    slip::{Slip, SlipType, SLIP_SIZE},
    time::create_timestamp,
    transaction::{Transaction, TransactionType, TRANSACTION_SIZE},
};
use ahash::AHashMap;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct BlockCore {
    id: u64,
    timestamp: u64,
    previous_block_hash: SaitoHash,
    #[serde_as(as = "[_; 33]")]
    creator: SaitoPublicKey, // public key of block creator
    merkle_root: SaitoHash, // merkle root of txs
    #[serde_as(as = "[_; 64]")]
    signature: SaitoSignature, // signature of block creator
    treasury: u64,
    burnfee: u64,
    difficulty: u64,
}

impl BlockCore {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u64,
        timestamp: u64,
        previous_block_hash: [u8; 32],
        creator: [u8; 33],
        merkle_root: [u8; 32],
        signature: [u8; 64],
        treasury: u64,
        burnfee: u64,
        difficulty: u64,
    ) -> Self {
        Self {
            id,
            timestamp,
            previous_block_hash,
            creator,
            merkle_root,
            signature,
            treasury,
            burnfee,
            difficulty,
        }
    }
}

impl Default for BlockCore {
    fn default() -> Self {
        Self::new(
            0,
            create_timestamp(),
            [0; 32],
            [0; 33],
            [0; 32],
            [0; 64],
            0,
            0,
            0,
        )
    }
}


//
// object used when generating and validation transactions, containing the
// information that is created selectively according to the transaction fees
// and the optional outbound payments.
//
#[derive(PartialEq, Debug, Clone)]
pub struct ConsensusValues {
    // expected transaction containing outbound payments
    pub fee_transaction: Option<Transaction>, 
    // number of FEE in transactions if exists
    pub ft_num: u8,
    // index of FEE in transactions if exists
    pub ft_idx: usize,
    // number of GT in transactions if exists
    pub gt_num: u8,
    // index of GT in transactions if exists
    pub gt_idx: usize,
}
impl ConsensusValues {
    #[allow(clippy::too_many_arguments)]
    pub fn new() -> ConsensusValues {
        ConsensusValues {
	    fee_transaction: None,
	    ft_num: 0,
	    ft_idx: usize::MAX,
	    gt_num: 0,
	    gt_idx: usize::MAX,
        }
    }
}





#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Block {
    /// Consensus Level Variables
    core: BlockCore,
    /// Transactions
    pub transactions: Vec<Transaction>,
    /// Self-Calculated / Validated
    hash: SaitoHash,
    /// total fees paid into block
    total_fees: u64,
    /// Is Block on longest chain
    lc: bool,
}

impl Block {
    pub fn new(core: BlockCore) -> Block {
        Block {
            core,
            transactions: vec![],
            hash: [0; 32],
            total_fees: 0,
            lc: false,
        }
    }

    pub fn get_transactions(&self) -> &Vec<Transaction> {
        &self.transactions
    }

    pub fn get_hash(&self) -> SaitoHash {
        self.hash
    }

    pub fn get_lc(&self) -> bool {
        self.lc
    }

    pub fn get_id(&self) -> u64 {
        self.core.id
    }

    pub fn get_timestamp(&self) -> u64 {
        self.core.timestamp
    }

    pub fn get_previous_block_hash(&self) -> SaitoHash {
        self.core.previous_block_hash
    }

    pub fn get_creator(&self) -> SaitoPublicKey {
        self.core.creator
    }

    pub fn get_merkle_root(&self) -> SaitoHash {
        self.core.merkle_root
    }

    pub fn get_signature(&self) -> SaitoSignature {
        self.core.signature
    }

    pub fn get_treasury(&self) -> u64 {
        self.core.treasury
    }

    pub fn get_burnfee(&self) -> u64 {
        self.core.burnfee
    }

    pub fn get_difficulty(&self) -> u64 {
        self.core.difficulty
    }

    pub fn set_transactions(&mut self, transactions: &mut Vec<Transaction>) {
        self.transactions = transactions.to_vec();
    }

    //pub fn set_transactions(&mut self, transactions: Vec<Transaction>) {
    //    self.transactions = transactions;
    //}

    pub fn set_id(&mut self, id: u64) {
        self.core.id = id;
    }

    pub fn set_lc(&mut self, lc: bool) {
        self.lc = lc;
    }

    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.core.timestamp = timestamp;
    }

    pub fn set_previous_block_hash(&mut self, previous_block_hash: SaitoHash) {
        self.core.previous_block_hash = previous_block_hash;
    }

    pub fn set_creator(&mut self, creator: SaitoPublicKey) {
        self.core.creator = creator;
    }

    pub fn set_merkle_root(&mut self, merkle_root: SaitoHash) {
        self.core.merkle_root = merkle_root;
    }

    // TODO - signature needs to be generated from consensus vars
    pub fn set_signature(&mut self) {}

    pub fn set_treasury(&mut self, treasury: u64) {
        self.core.treasury = treasury;
    }

    pub fn set_burnfee(&mut self, burnfee: u64) {
        self.core.burnfee = burnfee;
    }

    pub fn set_difficulty(&mut self, difficulty: u64) {
        self.core.difficulty = difficulty;
    }

    pub fn set_hash(&mut self, hash: SaitoHash) {
        self.hash = hash;
    }

    pub fn add_transaction(&mut self, tx: Transaction) {
        self.transactions.push(tx);
    }

    // TODO
    //
    // hash is nor being serialized from the right data - requires
    // merkle_root as an input into the hash, and that is not yet
    // supported. this is a stub that uses the timestamp and the
    // id -- it exists so each block will still have a unique hash
    // for blockchain functions.
    //
    pub fn generate_hash(&self) -> SaitoHash {
        //
        // fastest known way that isn't bincode ??
        //
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.core.id.to_be_bytes());
        vbytes.extend(&self.core.timestamp.to_be_bytes());
        vbytes.extend(&self.core.previous_block_hash);
        vbytes.extend(&self.core.creator);
        vbytes.extend(&self.core.merkle_root);
        vbytes.extend(&self.core.signature);
        vbytes.extend(&self.core.treasury.to_be_bytes());
        vbytes.extend(&self.core.burnfee.to_be_bytes());
        vbytes.extend(&self.core.difficulty.to_be_bytes());

        hash(&vbytes)
    }

    /// Serialize a Block for transport or disk.
    /// [len of transactions - 4 bytes - u32]
    /// [id - 8 bytes - u64]
    /// [timestamp - 8 bytes - u64]
    /// [previous_block_hash - 32 bytes - SHA 256 hash]
    /// [creator - 33 bytes - Secp25k1 pubkey compact format]
    /// [merkle_root - 32 bytes - SHA 256 hash
    /// [signature - 64 bytes - Secp25k1 sig]
    /// [treasury - 8 bytes - u64]
    /// [burnfee - 8 bytes - u64]
    /// [difficulty - 8 bytes - u64]
    /// [transaction][transaction][transaction]...
    pub fn serialize_for_net(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&(self.transactions.iter().len() as u32).to_be_bytes());
        vbytes.extend(&self.core.id.to_be_bytes());
        vbytes.extend(&self.core.timestamp.to_be_bytes());
        vbytes.extend(&self.core.previous_block_hash);
        vbytes.extend(&self.core.creator);
        vbytes.extend(&self.core.merkle_root);
        vbytes.extend(&self.core.signature);
        vbytes.extend(&self.core.treasury.to_be_bytes());
        vbytes.extend(&self.core.burnfee.to_be_bytes());
        vbytes.extend(&self.core.difficulty.to_be_bytes());
        let mut serialized_txs = vec![];
        self.transactions.iter().for_each(|transaction| {
            serialized_txs.extend(transaction.serialize_for_net());
        });
        vbytes.extend(serialized_txs);
        vbytes
    }
    /// Deserialize from bytes to a Block.
    /// [len of transactions - 4 bytes - u32]
    /// [id - 8 bytes - u64]
    /// [timestamp - 8 bytes - u64]
    /// [previous_block_hash - 32 bytes - SHA 256 hash]
    /// [creator - 33 bytes - Secp25k1 pubkey compact format]
    /// [merkle_root - 32 bytes - SHA 256 hash
    /// [signature - 64 bytes - Secp25k1 sig]
    /// [treasury - 8 bytes - u64]
    /// [burnfee - 8 bytes - u64]
    /// [difficulty - 8 bytes - u64]
    /// [transaction][transaction][transaction]...
    pub fn deserialize_for_net(bytes: Vec<u8>) -> Block {
        let transactions_len: u32 = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        let id: u64 = u64::from_be_bytes(bytes[4..12].try_into().unwrap());
        let timestamp: u64 = u64::from_be_bytes(bytes[12..20].try_into().unwrap());
        let previous_block_hash: SaitoHash = bytes[20..52].try_into().unwrap();
        let creator: SaitoPublicKey = bytes[52..85].try_into().unwrap();
        let merkle_root: SaitoHash = bytes[85..117].try_into().unwrap();
        let signature: SaitoSignature = bytes[117..181].try_into().unwrap();

        let treasury: u64 = u64::from_be_bytes(bytes[181..189].try_into().unwrap());
        let burnfee: u64 = u64::from_be_bytes(bytes[189..197].try_into().unwrap());
        let difficulty: u64 = u64::from_be_bytes(bytes[197..205].try_into().unwrap());
        let mut transactions = vec![];
        let mut start_of_transaction_data = 205;
        for _n in 0..transactions_len {
            let inputs_len: u32 = u32::from_be_bytes(
                bytes[start_of_transaction_data..start_of_transaction_data + 4]
                    .try_into()
                    .unwrap(),
            );
            let outputs_len: u32 = u32::from_be_bytes(
                bytes[start_of_transaction_data + 4..start_of_transaction_data + 8]
                    .try_into()
                    .unwrap(),
            );
            let message_len: usize = u32::from_be_bytes(
                bytes[start_of_transaction_data + 8..start_of_transaction_data + 12]
                    .try_into()
                    .unwrap(),
            ) as usize;
            let end_of_transaction_data = start_of_transaction_data
                + TRANSACTION_SIZE
                + ((inputs_len + outputs_len) as usize * SLIP_SIZE)
                + message_len;
            let transaction = Transaction::deserialize_from_net(
                bytes[start_of_transaction_data..end_of_transaction_data].to_vec(),
            );
            transactions.push(transaction);
            start_of_transaction_data = end_of_transaction_data;
        }
        let mut block = Block::new(BlockCore::new(
            id,
            timestamp,
            previous_block_hash,
            creator,
            merkle_root,
            signature,
            treasury,
            burnfee,
            difficulty,
        ));
        block.set_transactions(&mut transactions);
        block
    }

    pub fn generate_merkle_root(&self) -> SaitoHash {
        let tx_sig_hashes: Vec<SaitoHash> = self
            .transactions
            .iter()
            .map(|tx| tx.get_hash_for_signature())
            .collect();

        /*** KEEPING FOR SPEED REFERENCE TESTS ***
                let mt = MerkleTree::from_vec(SHA256, tx_sig_hashes);
                mt.root_hash()
                    .clone()
                    .try_into()
                    .expect("Failed to unwrao merkle root")
        *****************************************/

        let mut mrv: Vec<MerkleTreeLayer> = vec![];

        //
        // or let's try another approach
        //
        let tsh_len = tx_sig_hashes.len();
        let mut leaf_depth = 0;

        for i in 0..tsh_len {
            if (i + 1) < tsh_len {
                mrv.push(MerkleTreeLayer::new(
                    tx_sig_hashes[i],
                    tx_sig_hashes[i + 1],
                    leaf_depth,
                ));
            } else {
                mrv.push(MerkleTreeLayer::new(tx_sig_hashes[i], [0; 32], leaf_depth));
            }
        }

        let mut start_point = 0;
        let mut stop_point = mrv.len();
        let mut keep_looping = true;

        while keep_looping {
            // processing new layer
            leaf_depth += 1;

            // hash the parent in parallel
            mrv[start_point..stop_point]
                .par_iter_mut()
                .all(|leaf| leaf.hash());

            let start_point_old = start_point;
            start_point = mrv.len();

            for i in (start_point_old..stop_point).step_by(2) {
                //println!("looping in hash loop with {:?}", i);
                if (i + 1) < stop_point {
                    mrv.push(MerkleTreeLayer::new(
                        mrv[i].get_hash(),
                        mrv[i + 1].get_hash(),
                        leaf_depth,
                    ));
                } else {
                    mrv.push(MerkleTreeLayer::new(mrv[i].get_hash(), [0; 32], leaf_depth));
                }
            }

            stop_point = mrv.len();
            keep_looping = start_point < stop_point - 1;
        }

        //
        // hash the final leaf
        //
        mrv[start_point].hash();
        mrv[start_point].get_hash()
    }


    //
    //
    //
    pub fn generate_consensus_data(&self, blockchain : &Blockchain) -> ConsensusValues{

        let mut cv = ConsensusValues::new();

        let mut gt_num: u8 = 0;
        let mut ft_num: u8 = 0;
	let mut gt_idx: usize = usize::MAX;
	let mut ft_idx: usize = usize::MAX;
	let mut total_fees = 0;
	let mut total_fees = 0;
	let mut miner_publickey: SaitoPublicKey = [0; 33];
	let mut router_publickey: SaitoPublicKey = [0; 33];

	let miner_random = hash(&generate_random_bytes(32));
println!("Use rnd-hash rather than golden ticket: {:?}", miner_random);


	//
	// calculate total fees in block
	//
	let mut idx: usize = 0;
        for transaction in &self.transactions {

	    // fee transaction
	    if !transaction.is_fee_transaction() {
	        total_fees += transaction.get_total_fees();
            } else {
	        ft_num += 1;
		ft_idx = idx;
	    }

	    // gt transaction
	    if transaction.is_golden_ticket() {
		gt_num += 1;
		gt_idx = idx;
            }

	    idx += 1;
        }


	if gt_num > 0 {

	    let mut fee_transaction = Transaction::default();
	    fee_transaction.set_transaction_type(TransactionType::Fee);

	    //
	    // calculate miner and router payments
	    //
	    let miner_payment = total_fees / 2;
	    let router_payment = total_fees - miner_payment;


            let mut input1 = Slip::default();
                    input1.set_publickey(miner_publickey);
                    input1.set_amount(0);
                    input1.set_slip_type(SlipType::MinerInput);

            let mut output1 = Slip::default();
                    output1.set_publickey([0; 33]);
                    output1.set_amount(miner_payment);
                    output1.set_slip_type(SlipType::MinerOutput);

            let mut input2 = Slip::default();
                    input2.set_publickey(router_publickey);
                    input2.set_amount(0);
                    input2.set_slip_type(SlipType::RouterInput);

            let mut output2 = Slip::default();
                    output2.set_publickey(router_publickey);
                    output2.set_amount(router_payment);
                    output2.set_slip_type(SlipType::RouterOutput);

            fee_transaction.add_input(input1);
            fee_transaction.add_output(output1);
            fee_transaction.add_input(input2);
            fee_transaction.add_output(output2);

	    //
	    // calculate routing work
	    //
println!("total fees in block: {}", self.total_fees);

	    //
	    // fee transaction added to consensus values
	    //
	    cv.fee_transaction = Some(fee_transaction);
	    cv.ft_idx = ft_idx;
	    cv.ft_num = ft_num;
	    cv.gt_idx = gt_idx;
	    cv.gt_num = gt_num;

	}

	// and return
	return cv;

    }



    pub fn on_chain_reorganization(
        &self,
        utxoset: &mut AHashMap<SaitoUTXOSetKey, u64>,
        longest_chain: bool,
    ) -> bool {
        for tx in &self.transactions {
            tx.on_chain_reorganization(utxoset, longest_chain, self.get_id());
        }
        true
    }

    //
    // before we validate the block we need to generate some information such
    // as the hash of the transaction message data that is used to generate
    // the signature. because this requires mutable access to the transactions
    // Rust forces us to do it in a separate function.
    //
    // we first calculate as much information as we can in parallel before 
    // sweeping through the transactions to find out what percentage of the 
    // cumulative block fees they contain.
    //
    pub fn pre_validation_calculations(&mut self) -> bool {

	//
	// PARALLEL PROCESSING of most data
	//
        let _transactions_pre_calculated = &self
            .transactions
            .par_iter_mut()
            .all(|tx| tx.pre_validation_calculations_parallelizable());

        //
        // CUMULATIVE FEES only AFTER parallel calculations
        //
	let mut cumulative_fees = 0;
        for transaction in &mut self.transactions {
            cumulative_fees = transaction.pre_validation_calculations_cumulative_fees(cumulative_fees);
        }

	//
	// update block with total fees
	//
	self.total_fees = cumulative_fees;

	true
    }

    pub fn validate(&self, blockchain: &Blockchain) -> bool {
        //
        // validate burn fee
        //
        {
            let previous_block = blockchain.blocks.get(&self.get_previous_block_hash());
            if !previous_block.is_none() {
                let new_burnfee: u64 =
                    BurnFee::return_burnfee_for_block_produced_at_current_timestamp_in_nolan(
                        previous_block.unwrap().get_burnfee(),
                        self.get_timestamp(),
                        previous_block.unwrap().get_timestamp(),
                    );
                if new_burnfee != self.get_burnfee() {
                    println!(
                        "ERROR: burn fee does not validate, expected: {}",
                        new_burnfee
                    );
                    return false;
                }
            } else {
                // TODO assert that this is the first (or second?) block! ?
            }
        }

        //
        // verify merkle root
        //
        if self.core.merkle_root == [0; 32] {
            println!("merkle root is false 1");
            return false;
        }

        //
        // verify merkle root
        //
        if self.core.merkle_root != self.generate_merkle_root() {
            println!("merkle root is false 2");
            return false;
        }

        //
        // validate fee-transaction (miner/router/staker) payments
        //
        //
	let cv = self.generate_consensus_data(&blockchain);


        //
        // VALIDATE transactions
        //
        let _transactions_valid = &self.transactions.par_iter().all(|tx| tx.validate());

        true
    }


}

impl Default for Block {
    fn default() -> Self {
        Self::new(BlockCore::default())
    }
}

//
// TODO
//
// temporary data-serialization of blocks so that we can save
// to disk. These should only be called through the serialization
// functions within the block class, so that all access is
// compartmentalized and we can move to custom serialization
//
impl From<Vec<u8>> for Block {
    fn from(data: Vec<u8>) -> Self {
        bincode::deserialize(&data[..]).unwrap()
    }
}

impl Into<Vec<u8>> for Block {
    fn into(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}

#[cfg(test)]

mod tests {

    use super::*;
    use crate::{
        slip::{Slip, SlipCore},
        time::create_timestamp,
        transaction::{TransactionCore, TransactionType},
        wallet::Wallet,
    };

    #[test]
    fn block_core_new_test() {
        let timestamp = create_timestamp();
        let block_core = BlockCore::new(0, timestamp, [0; 32], [0; 33], [0; 32], [0; 64], 0, 0, 0);

        assert_eq!(block_core.id, 0);
        assert_eq!(block_core.timestamp, timestamp);
        assert_eq!(block_core.previous_block_hash, [0; 32]);
        assert_eq!(block_core.creator, [0; 33]);
        assert_eq!(block_core.merkle_root, [0; 32]);
        assert_eq!(block_core.signature, [0; 64]);
        assert_eq!(block_core.treasury, 0);
        assert_eq!(block_core.burnfee, 0);
        assert_eq!(block_core.difficulty, 0);
    }

    #[test]
    fn block_core_default_test() {
        let timestamp = create_timestamp();
        let block_core = BlockCore::default();

        assert_eq!(block_core.id, 0);
        assert_eq!(block_core.timestamp, timestamp);
        assert_eq!(block_core.previous_block_hash, [0; 32]);
        assert_eq!(block_core.creator, [0; 33]);
        assert_eq!(block_core.merkle_root, [0; 32]);
        assert_eq!(block_core.signature, [0; 64]);
        assert_eq!(block_core.treasury, 0);
        assert_eq!(block_core.burnfee, 0);
        assert_eq!(block_core.difficulty, 0);
    }

    #[test]
    fn block_new_test() {
        let timestamp = create_timestamp();
        let core = BlockCore::default();
        let block = Block::new(core);

        assert_eq!(block.core.id, 0);
        assert_eq!(block.core.timestamp, timestamp);
        assert_eq!(block.core.previous_block_hash, [0; 32]);
        assert_eq!(block.core.creator, [0; 33]);
        assert_eq!(block.core.merkle_root, [0; 32]);
        assert_eq!(block.core.signature, [0; 64]);
        assert_eq!(block.core.treasury, 0);
        assert_eq!(block.core.burnfee, 0);
        assert_eq!(block.core.difficulty, 0);
    }

    #[test]
    fn block_default_test() {
        let timestamp = create_timestamp();
        let block = Block::default();

        assert_eq!(block.core.id, 0);
        assert_eq!(block.core.timestamp, timestamp);
        assert_eq!(block.core.previous_block_hash, [0; 32]);
        assert_eq!(block.core.creator, [0; 33]);
        assert_eq!(block.core.merkle_root, [0; 32]);
        assert_eq!(block.core.signature, [0; 64]);
        assert_eq!(block.core.treasury, 0);
        assert_eq!(block.core.burnfee, 0);
        assert_eq!(block.core.difficulty, 0);
    }

    #[test]
    fn block_serialize_for_net_test() {
        let mock_input = Slip::new(SlipCore::default());
        let mock_output = Slip::new(SlipCore::default());
        let mock_tx = Transaction::new(TransactionCore::new(
            create_timestamp(),
            vec![mock_input.clone()],
            vec![mock_output.clone()],
            vec![104, 101, 108, 108, 111],
            TransactionType::Normal,
            [1; 64],
        ));
        let mock_tx2 = Transaction::new(TransactionCore::new(
            create_timestamp(),
            vec![mock_input.clone()],
            vec![mock_output.clone()],
            vec![],
            TransactionType::Normal,
            [2; 64],
        ));
        let timestamp = create_timestamp();
        let mut block = Block::new(BlockCore::new(
            1, timestamp, [1; 32], [2; 33], [3; 32], [4; 64], 1, 2, 3,
        ));
        block.set_transactions(&mut vec![mock_tx, mock_tx2]);
        let serialized_block = block.serialize_for_net();
        let deserialized_block = Block::deserialize_for_net(serialized_block);
        assert_eq!(block, deserialized_block);
        assert_eq!(deserialized_block.get_id(), 1);
        assert_eq!(deserialized_block.get_timestamp(), timestamp);
        assert_eq!(deserialized_block.get_previous_block_hash(), [1; 32]);
        assert_eq!(deserialized_block.get_creator(), [2; 33]);
        assert_eq!(deserialized_block.get_merkle_root(), [3; 32]);
        assert_eq!(deserialized_block.get_signature(), [4; 64]);
        assert_eq!(deserialized_block.get_treasury(), 1);
        assert_eq!(deserialized_block.get_burnfee(), 2);
        assert_eq!(deserialized_block.get_difficulty(), 3);
    }

    #[test]
    fn block_merkle_root_test() {
        let mut block = Block::default();
        let wallet = Wallet::new();

        let mut transactions = (0..5)
            .into_iter()
            .map(|_| {
                let mut transaction = Transaction::default();
                transaction.sign(wallet.get_privatekey());
                transaction
            })
            .collect();

        block.set_transactions(&mut transactions);

        assert!(block.generate_merkle_root().len() == 32);
    }
}
