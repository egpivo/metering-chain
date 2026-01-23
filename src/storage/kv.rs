use crate::storage::Storage;
use crate::state::State;
use crate::tx::SignedTx;
use crate::error::{Error, Result};
use crate::config::Config;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write, BufReader};
use std::path::PathBuf;
use std::fs;

/// File-based storage implementation using append-only logs and snapshots.
///
/// Files:
/// - `tx.log`: Append-only transaction log (bincode serialized)
/// - `state.bin`: State snapshot (bincode serialized State + u64 last_tx_id)
/// - `state.bin.tmp`: Temporary file for atomic snapshot writes
pub struct FileStorage {
    tx_log_path: PathBuf,
    state_path: PathBuf,
    state_tmp_path: PathBuf,
}

impl FileStorage {
    /// Create a new FileStorage with paths from config
    pub fn new(config: &Config) -> Self {
        FileStorage {
            tx_log_path: config.get_tx_log_path(),
            state_path: config.get_state_path(),
            state_tmp_path: config.get_state_path().with_extension("bin.tmp"),
        }
    }

    /// Create FileStorage with custom paths (for testing)
    pub fn with_paths(tx_log_path: PathBuf, state_path: PathBuf) -> Self {
        let state_tmp_path = state_path.with_extension("bin.tmp");
        FileStorage {
            tx_log_path,
            state_path,
            state_tmp_path,
        }
    }

    /// Ensure the data directory exists
    fn ensure_dir(&self) -> Result<()> {
        if let Some(parent) = self.tx_log_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| Error::StateError(format!("Failed to create data directory: {}", e)))?;
        }
        Ok(())
    }

}

impl Storage for FileStorage {
    fn append_tx(&mut self, tx: &SignedTx) -> Result<()> {
        self.ensure_dir()?;

        // Serialize transaction
        let tx_bytes = bincode::serialize(tx)
            .map_err(|e| Error::StateError(format!("Failed to serialize transaction: {}", e)))?;

        // Open file in append mode
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.tx_log_path)
            .map_err(|e| Error::StateError(format!("Failed to open tx log for append: {}", e)))?;

        // Write length prefix (u64 little-endian) + transaction data
        let len = tx_bytes.len() as u64;
        file.write_all(&len.to_le_bytes())
            .map_err(|e| Error::StateError(format!("Failed to write tx length: {}", e)))?;
        file.write_all(&tx_bytes)
            .map_err(|e| Error::StateError(format!("Failed to write tx data: {}", e)))?;

        // Fsync for crash safety (append-only semantics)
        file.sync_all()
            .map_err(|e| Error::StateError(format!("Failed to fsync tx log: {}", e)))?;

        Ok(())
    }

    fn load_state(&self) -> Result<Option<(State, u64)>> {
        if !self.state_path.exists() {
            return Ok(None);
        }

        let mut file = File::open(&self.state_path)
            .map_err(|e| Error::StateError(format!("Failed to open state file: {}", e)))?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|e| Error::StateError(format!("Failed to read state file: {}", e)))?;

        // Deserialize: State + last_tx_id (u64)
        // Format: [State bytes][last_tx_id: u64]
        if data.len() < 8 {
            return Err(Error::StateError("State file too short".to_string()));
        }

        // Extract last_tx_id (last 8 bytes)
        let last_tx_id_bytes = &data[data.len() - 8..];
        let last_tx_id = u64::from_le_bytes([
            last_tx_id_bytes[0], last_tx_id_bytes[1], last_tx_id_bytes[2], last_tx_id_bytes[3],
            last_tx_id_bytes[4], last_tx_id_bytes[5], last_tx_id_bytes[6], last_tx_id_bytes[7],
        ]);

        // Deserialize State (everything except last 8 bytes)
        let state_bytes = &data[..data.len() - 8];
        let state: State = bincode::deserialize(state_bytes)
            .map_err(|e| Error::StateError(format!("Failed to deserialize state: {}", e)))?;

        Ok(Some((state, last_tx_id)))
    }

    fn persist_state(&mut self, state: &State, last_tx_id: u64) -> Result<()> {
        self.ensure_dir()?;

        // Serialize state
        let state_bytes = bincode::serialize(state)
            .map_err(|e| Error::StateError(format!("Failed to serialize state: {}", e)))?;

        // Write to temporary file
        let mut file = File::create(&self.state_tmp_path)
            .map_err(|e| Error::StateError(format!("Failed to create temp state file: {}", e)))?;

        // Write state + last_tx_id
        file.write_all(&state_bytes)
            .map_err(|e| Error::StateError(format!("Failed to write state: {}", e)))?;
        file.write_all(&last_tx_id.to_le_bytes())
            .map_err(|e| Error::StateError(format!("Failed to write last_tx_id: {}", e)))?;

        // Fsync before rename (crash safety)
        file.sync_all()
            .map_err(|e| Error::StateError(format!("Failed to fsync temp state file: {}", e)))?;
        drop(file); // Close file before rename

        // Atomic rename (crash-safe snapshot)
        fs::rename(&self.state_tmp_path, &self.state_path)
            .map_err(|e| Error::StateError(format!("Failed to rename temp state file: {}", e)))?;

        // Fsync parent directory (ensure rename is persisted)
        if let Some(parent) = self.state_path.parent() {
            let parent_file = File::open(parent)
                .map_err(|e| Error::StateError(format!("Failed to open parent directory: {}", e)))?;
            parent_file.sync_all()
                .map_err(|e| Error::StateError(format!("Failed to fsync parent directory: {}", e)))?;
        }

        Ok(())
    }

    fn load_txs_from(&self, from_tx_id: u64) -> Result<Vec<SignedTx>> {
        if !self.tx_log_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.tx_log_path)
            .map_err(|e| Error::StateError(format!("Failed to open tx log: {}", e)))?;
        let mut reader = BufReader::new(file);

        let mut transactions = Vec::new();
        let mut current_id = 0u64;

        loop {
            // Read length prefix
            let mut len_buf = [0u8; 8];
            match reader.read_exact(&mut len_buf) {
                Ok(_) => {
                    let len = u64::from_le_bytes(len_buf) as usize;
                    let mut tx_buf = vec![0u8; len];
                    reader.read_exact(&mut tx_buf)
                        .map_err(|e| Error::StateError(format!("Failed to read tx data: {}", e)))?;

                    // Only include transactions from from_tx_id onwards
                    if current_id >= from_tx_id {
                        let tx: SignedTx = bincode::deserialize(&tx_buf)
                            .map_err(|e| Error::StateError(format!("Failed to deserialize tx: {}", e)))?;
                        transactions.push(tx);
                    }

                    current_id += 1;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(e) => {
                    return Err(Error::StateError(format!("Failed to read tx log: {}", e)));
                }
            }
        }

        Ok(transactions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::State;
    use crate::tx::{Transaction, SignedTx};
    use tempfile::TempDir;

    fn create_test_storage() -> (FileStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let tx_log_path = temp_dir.path().join("tx.log");
        let state_path = temp_dir.path().join("state.bin");
        let storage = FileStorage::with_paths(tx_log_path, state_path);
        (storage, temp_dir)
    }

    #[test]
    fn test_append_and_load_tx() {
        let (mut storage, _temp_dir) = create_test_storage();

        let tx = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::Mint {
                to: "bob".to_string(),
                amount: 100,
            },
        );

        storage.append_tx(&tx).unwrap();
        let txs = storage.load_txs_from(0).unwrap();
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].signer, "alice");
    }

    #[test]
    fn test_load_txs_from() {
        let (mut storage, _temp_dir) = create_test_storage();

        // Append multiple transactions
        for i in 0..5 {
            let tx = SignedTx::new(
                "alice".to_string(),
                i,
                Transaction::Mint {
                    to: "bob".to_string(),
                    amount: 100 + i,
                },
            );
            storage.append_tx(&tx).unwrap();
        }

        // Load from tx_id 2
        let txs = storage.load_txs_from(2).unwrap();
        assert_eq!(txs.len(), 3); // tx_ids 2, 3, 4
    }

    #[test]
    fn test_persist_and_load_state() {
        let (mut storage, _temp_dir) = create_test_storage();

        let mut state = State::new();
        state.accounts.insert("alice".to_string(), crate::state::Account::with_balance(1000));

        storage.persist_state(&state, 5).unwrap();

        let loaded = storage.load_state().unwrap();
        assert!(loaded.is_some());
        let (loaded_state, last_tx_id) = loaded.unwrap();
        assert_eq!(last_tx_id, 5);
        assert_eq!(loaded_state.accounts.len(), 1);
        assert_eq!(loaded_state.get_account("alice").unwrap().balance(), 1000);
    }

    #[test]
    fn test_load_state_none() {
        let (storage, _temp_dir) = create_test_storage();
        let loaded = storage.load_state().unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_append_multiple_txs() {
        let (mut storage, _temp_dir) = create_test_storage();

        let tx1 = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::Mint {
                to: "bob".to_string(),
                amount: 100,
            },
        );
        let tx2 = SignedTx::new(
            "bob".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "bob".to_string(),
                service_id: "storage".to_string(),
                deposit: 50,
            },
        );

        storage.append_tx(&tx1).unwrap();
        storage.append_tx(&tx2).unwrap();

        let txs = storage.load_txs_from(0).unwrap();
        assert_eq!(txs.len(), 2);
    }
}
