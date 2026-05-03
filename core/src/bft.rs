#![allow(dead_code)]

//! In-Kernel Byzantine Fault Tolerance (BFT) Consensus
//! Embeds a mathematical consensus state machine directly into the lock-free 
//! `io_uring` event bus to manage the Hive network without blocking inference.

use core::sync::atomic::{AtomicUsize, AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BftPhase {
    Idle,
    PrePrepare,
    Prepare,
    Commit,
}

/// Represents a payload traversing the consensus network.
#[derive(Debug, Clone)]
pub struct BftMessage {
    pub view_num: u64,
    pub seq_num: u64,
    pub sender_id: u32,
    pub phase: BftPhase,
    pub payload_hash: [u8; 32],
}

/// The core lock-free state machine for consensus.
pub struct BftEngine {
    current_view: AtomicU64,
    current_seq: AtomicU64,
    
    // Atomic bit-maps to track 2f+1 quorums efficiently without mutexes.
    // Each bit represents a vote from a specific node ID (up to 64 nodes in this stub).
    prepare_quorum: AtomicU64,
    commit_quorum: AtomicU64,
    
    // Total number of nodes in the local swarm for calculating 2f+1.
    swarm_size: AtomicUsize,
}

impl BftEngine {
    pub fn new(initial_swarm_size: usize) -> Self {
        Self {
            current_view: AtomicU64::new(0),
            current_seq: AtomicU64::new(0),
            prepare_quorum: AtomicU64::new(0),
            commit_quorum: AtomicU64::new(0),
            swarm_size: AtomicUsize::new(initial_swarm_size),
        }
    }

    /// Processes an incoming message off the lock-free ring buffer.
    pub fn process_message(&self, msg: BftMessage) -> Result<(), &'static str> {
        let current_v = self.current_view.load(Ordering::Relaxed);
        let current_s = self.current_seq.load(Ordering::Relaxed);

        if msg.view_num != current_v || msg.seq_num != current_s {
            // Ignore messages from old views/sequences
            return Err("Stale message discarded");
        }

        match msg.phase {
            BftPhase::PrePrepare => {
                // If valid, node broadcasts a Prepare message
                Ok(())
            }
            BftPhase::Prepare => {
                self.record_vote(&self.prepare_quorum, msg.sender_id);
                if self.check_quorum(&self.prepare_quorum) {
                    // Transition to Commit phase
                    Ok(())
                } else {
                    Ok(())
                }
            }
            BftPhase::Commit => {
                self.record_vote(&self.commit_quorum, msg.sender_id);
                if self.check_quorum(&self.commit_quorum) {
                    // Consensus reached! Apply to LSM tree.
                    // This triggers the `StateApplied` event for the AI core.
                    self.reset_quorums();
                    self.current_seq.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                } else {
                    Ok(())
                }
            }
            BftPhase::Idle => Ok(()),
        }
    }

    fn record_vote(&self, quorum_map: &AtomicU64, node_id: u32) {
        if node_id < 64 {
            let bit = 1u64 << node_id;
            quorum_map.fetch_or(bit, Ordering::SeqCst);
        }
    }

    fn check_quorum(&self, quorum_map: &AtomicU64) -> bool {
        let votes = quorum_map.load(Ordering::SeqCst).count_ones() as usize;
        let n = self.swarm_size.load(Ordering::Relaxed);
        let f = (n - 1) / 3;
        let required_quorum = 2 * f + 1;
        
        votes >= required_quorum
    }

    fn reset_quorums(&self) {
        self.prepare_quorum.store(0, Ordering::SeqCst);
        self.commit_quorum.store(0, Ordering::SeqCst);
    }
}
