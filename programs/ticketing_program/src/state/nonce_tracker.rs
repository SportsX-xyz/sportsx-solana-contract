use anchor_lang::prelude::*;

/// Circular buffer for nonce tracking with time-based expiration
#[account]
pub struct NonceTracker {
    /// Circular buffer of nonces (last 10 entries, good for testing)
    pub nonces: [u64; 10],
    
    /// Buyer address for each nonce (for collision prevention)
    pub buyers: [Pubkey; 10],
    
    /// Timestamps for each nonce entry
    pub timestamps: [i64; 10],
    
    /// Next index to write (circular)
    pub next_index: u16,
}

impl NonceTracker {
    pub const SEED_PREFIX: &'static [u8] = b"nonce_tracker";
    pub const BUFFER_SIZE: usize = 10;
    pub const EXPIRY_SECONDS: i64 = 600; // 10 minutes (long after 60s auth expiry)
    
    // Size: 8 (discriminator) + 10*8 (nonces) + 10*32 (buyers) + 10*8 (timestamps) + 2 (index)
    // = 8 + 80 + 320 + 80 + 2 = 490 bytes
    pub const SIZE: usize = 8 + (Self::BUFFER_SIZE * 8) + (Self::BUFFER_SIZE * 32) + (Self::BUFFER_SIZE * 8) + 2;
    
    /// Check if nonce+buyer combination is already used
    pub fn is_nonce_used(&self, nonce: u64, buyer: &Pubkey, current_time: i64) -> bool {
        for i in 0..Self::BUFFER_SIZE {
            // Only check non-expired entries
            if self.timestamps[i] + Self::EXPIRY_SECONDS > current_time {
                // Check both nonce AND buyer match
                if self.nonces[i] == nonce && self.buyers[i] == *buyer {
                    return true; // Already used by this buyer
                }
            }
        }
        false
    }
    
    /// Mark nonce+buyer as used with current timestamp
    pub fn mark_nonce_used(&mut self, nonce: u64, buyer: Pubkey, current_time: i64) {
        let idx = (self.next_index as usize) % Self::BUFFER_SIZE;
        self.nonces[idx] = nonce;
        self.buyers[idx] = buyer;
        self.timestamps[idx] = current_time;
        self.next_index = self.next_index.wrapping_add(1);
    }
    
    /// Get count of valid (non-expired) nonces
    pub fn count_valid_nonces(&self, current_time: i64) -> usize {
        self.timestamps
            .iter()
            .filter(|&&ts| ts + Self::EXPIRY_SECONDS > current_time)
            .count()
    }
}

