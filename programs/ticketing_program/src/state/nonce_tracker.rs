use anchor_lang::prelude::*;

#[account]
pub struct NonceTracker {
    /// Used nonces (for anti-replay protection)
    /// Note: In production, consider using a bloom filter or bitmap for efficiency
    pub used_nonces: Vec<u64>,
}

impl NonceTracker {
    pub const SEED_PREFIX: &'static [u8] = b"nonce_tracker";
    
    // Initial size: 8 (discriminator) + 4 (vec length) + 1000 * 8 (space for ~1000 nonces)
    pub const SIZE: usize = 8 + 4 + 8000;
    
    pub fn is_nonce_used(&self, nonce: u64) -> bool {
        self.used_nonces.contains(&nonce)
    }
    
    pub fn mark_nonce_used(&mut self, nonce: u64) {
        if !self.is_nonce_used(nonce) {
            self.used_nonces.push(nonce);
        }
    }
    
    /// Clean up old nonces to prevent unbounded growth
    /// This should be called periodically or implement a sliding window
    pub fn cleanup_old_nonces(&mut self, keep_last: usize) {
        if self.used_nonces.len() > keep_last {
            let start_index = self.used_nonces.len() - keep_last;
            self.used_nonces = self.used_nonces[start_index..].to_vec();
        }
    }
}

