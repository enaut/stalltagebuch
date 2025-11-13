/// CRDT service: Hybrid Logical Clock (HLC) and basic CRDT operations
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Hybrid Logical Clock for total ordering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HybridLogicalClock {
    pub ts: i64,              // Wall-clock timestamp (milliseconds since epoch)
    pub logical_counter: u32, // Logical counter for causality
    pub device_id: String,    // Tie-breaker for deterministic ordering
}

impl HybridLogicalClock {
    /// Create a new HLC with current wall-clock time
    pub fn new(device_id: String) -> Self {
        Self {
            ts: chrono::Utc::now().timestamp_millis(),
            logical_counter: 0,
            device_id,
        }
    }

    /// Advance HLC (increment logical counter or update timestamp)
    pub fn tick(&mut self) {
        let now = chrono::Utc::now().timestamp_millis();
        if now > self.ts {
            self.ts = now;
            self.logical_counter = 0;
        } else {
            self.logical_counter += 1;
        }
    }

    /// Update HLC based on received remote clock (preserves monotonicity)
    pub fn update(&mut self, remote: &HybridLogicalClock) {
        let now = chrono::Utc::now().timestamp_millis();
        let max_ts = self.ts.max(remote.ts).max(now);

        if max_ts == self.ts && max_ts == remote.ts {
            self.logical_counter = self.logical_counter.max(remote.logical_counter) + 1;
        } else if max_ts == self.ts {
            self.logical_counter += 1;
        } else if max_ts == remote.ts {
            self.logical_counter = remote.logical_counter + 1;
        } else {
            self.logical_counter = 0;
        }

        self.ts = max_ts;
    }
}

impl PartialOrd for HybridLogicalClock {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HybridLogicalClock {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ts
            .cmp(&other.ts)
            .then_with(|| self.logical_counter.cmp(&other.logical_counter))
            .then_with(|| self.device_id.cmp(&other.device_id))
    }
}

/// CRDT operation types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CrdtOp {
    /// Last-Writer-Wins register (for scalar fields)
    LwwSet {
        field: String,
        value: serde_json::Value,
    },
    /// OR-Set add (for collections)
    OrAdd {
        field: String,
        element: String,
        element_id: String, // unique tag for this add
    },
    /// OR-Set remove (for collections)
    OrRemove { field: String, element_id: String },
    /// PN-Counter increment (for additive counters)
    PnIncrement { field: String, delta: i32 },
    /// Tombstone (soft delete)
    Delete,
}

/// NDJSON operation log entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Operation {
    pub op_id: String,       // ULID of this operation
    pub entity_type: String, // quail | event | egg_record | photo_meta
    pub entity_id: String,   // UUID of the entity
    pub clock: HybridLogicalClock,
    pub op: CrdtOp,
}

impl Operation {
    /// Create a new operation with a fresh HLC tick
    pub fn new(entity_type: String, entity_id: String, device_id: String, op: CrdtOp) -> Self {
        let op_id = ulid::Ulid::new().to_string();
        let mut clock = HybridLogicalClock::new(device_id);
        clock.tick();

        Self {
            op_id,
            entity_type,
            entity_id,
            clock,
            op,
        }
    }
}

/// Apply LWW merge: compare clocks and keep the winner
pub fn lww_merge<T: Clone>(
    local_value: &T,
    local_clock: &HybridLogicalClock,
    remote_value: &T,
    remote_clock: &HybridLogicalClock,
) -> (T, HybridLogicalClock) {
    if remote_clock > local_clock {
        (remote_value.clone(), remote_clock.clone())
    } else {
        (local_value.clone(), local_clock.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hlc_ordering() {
        let mut clock1 = HybridLogicalClock::new("device1".to_string());
        let mut clock2 = HybridLogicalClock::new("device2".to_string());

        clock1.tick();
        clock2.tick();

        // Assuming clock2 ticks slightly later or same time
        assert!(clock1 <= clock2);
    }

    #[test]
    fn test_hlc_update() {
        let mut local = HybridLogicalClock {
            ts: 1000,
            logical_counter: 5,
            device_id: "device1".to_string(),
        };

        let remote = HybridLogicalClock {
            ts: 1000,
            logical_counter: 3,
            device_id: "device2".to_string(),
        };

        local.update(&remote);

        // Should increment logical counter since ts is same
        assert!(local.logical_counter > 5);
    }

    #[test]
    fn test_lww_merge() {
        let clock1 = HybridLogicalClock {
            ts: 1000,
            logical_counter: 0,
            device_id: "device1".to_string(),
        };

        let clock2 = HybridLogicalClock {
            ts: 2000,
            logical_counter: 0,
            device_id: "device2".to_string(),
        };

        let (winner, winner_clock) = lww_merge(&"old", &clock1, &"new", &clock2);

        assert_eq!(winner, "new");
        assert_eq!(winner_clock, clock2);
    }
}
