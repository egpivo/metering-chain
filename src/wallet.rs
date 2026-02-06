//! Infrastructure: keypair, address derivation (hex of pubkey), sign, verify.
//! Domain layer does not depend on this.

use crate::error::{Error, Result};
use crate::tx::transaction::PAYLOAD_VERSION_V2;
use crate::tx::validation::{
    build_signed_proof, delegation_claims_to_sign, DelegationProofMinimal,
};
use crate::tx::{SignedTx, Transaction};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const ADDRESS_PREFIX: &str = "0x";

/// Single wallet: address = hex(public key), secret key kept in memory.
pub struct Wallet {
    pub address: String,
    signing_key: SigningKey,
}

impl Wallet {
    pub fn new_random() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let address = public_key_to_address(signing_key.verifying_key().as_bytes());
        Wallet {
            address,
            signing_key,
        }
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    /// Sign canonical message bytes; returns 64-byte Ed25519 signature.
    pub fn sign_bytes(&self, message: &[u8]) -> Vec<u8> {
        let sig: Signature = self.signing_key.sign(message);
        sig.to_bytes().to_vec()
    }

    /// Build SignedTx (v1) with correct nonce and attach signature.
    pub fn sign_transaction(&self, nonce: u64, kind: Transaction) -> Result<SignedTx> {
        let tx = SignedTx::new(self.address.clone(), nonce, kind);
        let message = tx.message_to_sign()?;
        let signature = self.sign_bytes(&message);
        Ok(SignedTx {
            signature: Some(signature),
            ..tx
        })
    }

    /// Create signed delegation proof bytes (owner signs claims). Call as owner wallet.
    pub fn sign_delegation_proof(&self, claims: &DelegationProofMinimal) -> Vec<u8> {
        let message = delegation_claims_to_sign(claims);
        let signature = self.sign_bytes(&message);
        build_signed_proof(claims, signature)
    }

    /// Build SignedTx v2 for delegated consume: nonce_account=Some(owner), valid_at, delegation_proof.
    /// Signer is the delegate; owner is from kind. Caller must pass owner nonce and proof bytes.
    pub fn sign_transaction_v2(
        &self,
        nonce: u64,
        nonce_account: String,
        valid_at: u64,
        delegation_proof: Vec<u8>,
        kind: Transaction,
    ) -> Result<SignedTx> {
        let tx = SignedTx {
            payload_version: Some(PAYLOAD_VERSION_V2),
            signer: self.address.clone(),
            nonce,
            nonce_account: Some(nonce_account),
            valid_at: Some(valid_at),
            delegation_proof: Some(delegation_proof),
            kind,
            signature: None,
        };
        let message = tx.message_to_sign()?;
        let signature = self.sign_bytes(&message);
        Ok(SignedTx {
            signature: Some(signature),
            ..tx
        })
    }

    fn to_stored(&self) -> StoredWallet {
        StoredWallet {
            address: self.address.clone(),
            public_key_hex: hex::encode(self.signing_key.verifying_key().as_bytes()),
            secret_key_hex: hex::encode(self.signing_key.to_bytes()),
        }
    }
}

/// Address = 0x + hex(32-byte public key).
pub fn public_key_to_address(pubkey: &[u8]) -> String {
    format!("{}{}", ADDRESS_PREFIX, hex::encode(pubkey))
}

/// Decode address to 32-byte public key. Returns None if not a valid hex pubkey.
pub fn address_to_public_key(address: &str) -> Option<[u8; 32]> {
    let hex_part = address.strip_prefix(ADDRESS_PREFIX).unwrap_or(address);
    let bytes = hex::decode(hex_part).ok()?;
    let arr: [u8; 32] = bytes.try_into().ok()?;
    Some(arr)
}

/// Hard gate: delegated Consume must use payload_version=2. Call before signature verification.
pub fn enforce_delegated_consume_v2(tx: &SignedTx) -> Result<()> {
    if tx.is_delegated_consume() {
        if tx.effective_payload_version() != PAYLOAD_VERSION_V2 {
            return Err(Error::DelegatedConsumeRequiresV2);
        }
    }
    Ok(())
}

/// Verify SignedTx: Phase 2/3 requires valid signature. Enforces delegated-consume v2 gate, then version-aware message verify.
pub fn verify_signature(tx: &SignedTx) -> Result<()> {
    enforce_delegated_consume_v2(tx)?;
    let sig_bytes = tx.signature.as_ref().ok_or_else(|| {
        Error::SignatureVerification("Signed transaction required (Phase 2)".to_string())
    })?;
    let pubkey_bytes = address_to_public_key(&tx.signer).ok_or_else(|| {
        Error::SignatureVerification(format!(
            "Invalid address format (expected hex pubkey): {}",
            tx.signer
        ))
    })?;
    let message = tx.message_to_sign()?;
    let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)
        .map_err(|e| Error::SignatureVerification(e.to_string()))?;
    let arr: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| Error::SignatureVerification("Invalid signature length".to_string()))?;
    let sig = Signature::from_bytes(&arr);
    verifying_key
        .verify(&message, &sig)
        .map_err(|e| Error::SignatureVerification(e.to_string()))?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct StoredWallet {
    address: String,
    public_key_hex: String,
    secret_key_hex: String,
}

/// Wallet store with optional JSON file persistence (Phase 2 MVP: unencrypted).
pub struct Wallets {
    by_address: HashMap<String, Wallet>,
    file_path: PathBuf,
}

impl Wallets {
    pub fn new(file_path: PathBuf) -> Self {
        let mut w = Wallets {
            by_address: HashMap::new(),
            file_path,
        };
        let _ = w.load_from_file();
        w
    }

    pub fn create_wallet(&mut self) -> Result<String> {
        let wallet = Wallet::new_random();
        let address = wallet.address().to_string();
        self.by_address.insert(address.clone(), wallet);
        self.save_to_file()?;
        Ok(address)
    }

    pub fn get_addresses(&self) -> Vec<String> {
        self.by_address.keys().cloned().collect()
    }

    pub fn get_wallet(&self, address: &str) -> Option<&Wallet> {
        self.by_address.get(address)
    }

    pub fn sign_transaction(
        &self,
        address: &str,
        nonce: u64,
        kind: Transaction,
    ) -> Result<SignedTx> {
        let wallet = self
            .by_address
            .get(address)
            .ok_or_else(|| Error::InvalidTransaction(format!("Wallet not found: {}", address)))?;
        wallet.sign_transaction(nonce, kind)
    }

    /// Delegated sign: signer is address, nonce_account (owner) and nonce from coordinator, valid_at and proof from caller.
    pub fn sign_transaction_v2(
        &self,
        address: &str,
        nonce: u64,
        nonce_account: String,
        valid_at: u64,
        delegation_proof: Vec<u8>,
        kind: Transaction,
    ) -> Result<SignedTx> {
        let wallet = self
            .by_address
            .get(address)
            .ok_or_else(|| Error::InvalidTransaction(format!("Wallet not found: {}", address)))?;
        wallet.sign_transaction_v2(nonce, nonce_account, valid_at, delegation_proof, kind)
    }

    fn load_from_file(&mut self) -> Result<()> {
        let path = &self.file_path;
        if !path.exists() {
            return Ok(());
        }
        let s = fs::read_to_string(path)
            .map_err(|e| Error::InvalidTransaction(format!("Failed to read wallets: {}", e)))?;
        let stored: Vec<StoredWallet> = serde_json::from_str(&s)
            .map_err(|e| Error::InvalidTransaction(format!("Invalid wallets JSON: {}", e)))?;
        for sw in stored {
            let secret_bytes: [u8; 32] = hex::decode(&sw.secret_key_hex)
                .ok()
                .and_then(|v| v.try_into().ok())
                .ok_or_else(|| Error::InvalidTransaction("Invalid secret_key_hex".to_string()))?;
            let signing_key = SigningKey::from_bytes(&secret_bytes);
            let address = public_key_to_address(signing_key.verifying_key().as_bytes());
            if address != sw.address {
                continue;
            }
            self.by_address.insert(
                address.clone(),
                Wallet {
                    address,
                    signing_key,
                },
            );
        }
        Ok(())
    }

    fn save_to_file(&self) -> Result<()> {
        let parent = self.file_path.parent().unwrap_or(std::path::Path::new("."));
        fs::create_dir_all(parent)
            .map_err(|e| Error::StateError(format!("Failed to create wallets dir: {}", e)))?;
        let stored: Vec<StoredWallet> = self.by_address.values().map(Wallet::to_stored).collect();
        let s = serde_json::to_string_pretty(&stored)
            .map_err(|e| Error::StateError(format!("Failed to serialize wallets: {}", e)))?;
        fs::write(&self.file_path, s)
            .map_err(|e| Error::StateError(format!("Failed to write wallets: {}", e)))?;
        Ok(())
    }
}
