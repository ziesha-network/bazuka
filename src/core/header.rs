use crate::core::U256;
use primitive_types::U256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    pub parent_hash: [u8; 32],
    pub number: U256,
    pub state_root: [u8; 32],
    pub hash: [u8; 32],
    pub digests: Digests,
}

impl Default for Header {
    fn default() -> Self {
        Header {
            parent_hash: [0; 32],
            number: Default::default(),
            state_root: [0; 32],
            hash: [0; 32],
            digests: Default::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Digests {
    pub logs: Vec<DigestItem>,
}

impl Default for Digests {
    fn default() -> Self {
        DigestItem { logs: Vec::new() }
    }
}

impl Digests {
    pub fn logs(&self) -> &[DigestItem] {
        &self.logs
    }

    pub fn push(&mut self, item: DigestItem) {
        self.logs.push(item)
    }

    pub fn pop(&mut self) -> Option<DigestItem> {
        self.logs.pop()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "t", content = "c")]
pub enum DigestItem {
    PreRuntime(Vec<u8>),
    Consensus(Vec<u8>),
    Seal(Vec<u8>),
}

impl DigestItem {
    pub fn content(&self) -> &[u8] {
        match self {
            DigestItem::PreRuntime(v) => &v,
            _ => {}
        }
    }
}
