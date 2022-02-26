use serde::{Deserialize, Serialize};

/// Generic Header Digests
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Digests {
    logs: Vec<Digest>,
}

impl Default for Digests {
    fn default() -> Self {
        Digests { logs: Vec::new() }
    }
}

impl Digests {
    /// Get reference to all digest items.
    pub fn logs(&self) -> &[Digest] {
        &self.logs
    }

    /// Push new digest item.
    pub fn push(&mut self, item: Digest) {
        self.logs.push(item);
    }

    /// Pop a digest item.
    pub fn pop(&mut self) -> Option<Digest> {
        self.logs.pop()
    }

    /// Get reference to the first digest item that matches the passed predicate.
    pub fn log<T: ?Sized, F: Fn(&Digest) -> Option<&T>>(&self, predicate: F) -> Option<&T> {
        self.logs().iter().find_map(predicate)
    }
}

/// Digest prevent code and state duplication
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", content = "c")] // short for output
pub enum Digest {
    /// consensus to runtime
    PreRuntime(PreDigest),
    /// runtime to consensus
    Consensus(BabeConsensusLog),
}

/// @TODO: WIP
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreDigest {
    Primary,
    SecondaryPlain,
    SecondaryVRF,
}

/// @TODO: WIP
/// A consensus log item for BABE.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BabeConsensusLog {
    NextEpochData,
    OnDisable,
    NextConfigData,
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        assert_eq!(1, 1)
    }
}
