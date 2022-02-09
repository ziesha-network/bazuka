use std::collections::BTreeMap;
use std::ops::{Add, Sub};

use num_traits::{One, Zero};

use crate::consensus::babe::Epoch;
use crate::consensus::forktree::ForkTree;
use crate::consensus::slots::Slot;
use crate::consensus::Error;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub enum EpochIdentifierPosition {
    Genesis0,
    Genesis1,
    Regular,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct EpochIdentifier<Hash, Num> {
    pub position: EpochIdentifierPosition,
    pub hash: Hash,
    pub num: Num,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", content = "c")]
pub enum PersistedEpoch<E> {
    Genesis(E, E),
    Regular(E),
}

impl<E> PersistedEpoch<E> {
    pub fn is_geneis(&self) -> bool {
        matches!(self, Self::Genesis(_, _))
    }
}

impl<E> PersistedEpoch<E> {
    /// Map the epoch to a different type using a conversion function.
    pub fn map<B, F, Hash, Number>(self, h: &Hash, n: &Number, f: &mut F) -> PersistedEpoch<B>
    where
        F: FnMut(&Hash, &Number, E) -> B,
    {
        match self {
            PersistedEpoch::Genesis(epoch_0, epoch_1) => {
                PersistedEpoch::Genesis(f(h, n, epoch_0), f(h, n, epoch_1))
            }
            PersistedEpoch::Regular(epoch_n) => PersistedEpoch::Regular(f(h, n, epoch_n)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpochHeader {
    pub start_slot: Slot,
    pub end_slot: Slot,
}

impl<'a> From<&'a Epoch> for EpochHeader {
    fn from(e: &'a Epoch) -> Self {
        EpochHeader {
            start_slot: e.start_slot(),
            end_slot: e.end_slot(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EpochGap<Hash, Num> {
    current_hash: Hash,
    current_num: Num,
    current_epoch: PersistedEpoch<Epoch>,
    next: Option<(Hash, Num, Epoch)>,
}

impl<Hash, Num> EpochGap<Hash, Num>
where
    Hash: Copy + PartialEq + std::fmt::Debug,
    Num: Copy + PartialEq + std::fmt::Debug,
{
    fn matches(&self, slot: Slot) -> Option<(Hash, Num, EpochHeader, EpochIdentifierPosition)> {
        match (&self.current_hash, &self.current_num, &self.current_epoch) {
            (_, _, PersistedEpoch::Genesis(epoch_0, _))
                if slot >= epoch_0.start_slot() && slot <= epoch_0.end_slot() =>
            {
                return Some((
                    self.current.0,
                    self.current.1,
                    epoch_0.into(),
                    EpochIdentifierPosition::Genesis0,
                ))
            }
            (_, _, PersistedEpoch::Genesis(_, epoch_1))
                if slot >= epoch_1.start_slot && slot < epoch_1.end_slot() =>
            {
                return Some((
                    self.current.0,
                    self.current.1,
                    epoch_1.into(),
                    EpochIdentifierPosition::Genesis1,
                ))
            }
            (_, _, PersistedEpoch::Regular(epoch_n)) => {
                return Some((
                    self.current.0,
                    self.current.1,
                    epoch_n.into(),
                    EpochIdentifierPosition::Regular,
                ))
            }
            _ => {}
        }
        match &self.next {
            Some((h, n, epoch_n)) if slot >= epoch_n.start_slot() && slot < epoch_n.end_slot() => {
                Some((*h, *n, epoch_n.into(), EpochIdentifierPosition::Regular))
            }
            _ => None,
        }
    }

    pub fn epoch(&self, id: &EpochIdentifier<Hash, Num>) -> Option<&Epoch> {
        match (&self.current, &self.next) {
            ((h, n, e), _) if h == &id.hash && n == &id.number => match e {
                PersistedEpoch::Genesis(ref epoch_0, _)
                    if id.position == EpochIdentifierPosition::Genesis0 =>
                {
                    Some(epoch_0)
                }
                PersistedEpoch::Genesis(_, ref epoch_1)
                    if id.position == EpochIdentifierPosition::Genesis1 =>
                {
                    Some(epoch_1)
                }
                PersistedEpoch::Regular(ref epoch_n)
                    if id.position == EpochIdentifierPosition::Regular =>
                {
                    Some(epoch_n)
                }
                _ => None,
            },
            (_, Some((h, n, e)))
                if h == &id.hash
                    && n == &id.number
                    && id.position == EpochIdentifierPosition::Regular =>
            {
                Some(e)
            }
            _ => None,
        }
    }

    fn import(&mut self, slot: E::Slot, hash: Hash, number: Num, epoch: E) -> Result<(), E> {
        match (&mut self.current, &mut self.next) {
            ((_, _, PersistedEpoch::Genesis(_, epoch_1)), _) if slot == epoch_1.end_slot() => {
                self.next = Some((hash, number, epoch));
                Ok(())
            }
            (_, Some((_, _, epoch_n))) if slot == epoch_n.end_slot() => {
                let (cur_h, cur_n, cur_epoch) =
                    self.next.take().expect("Already matched as `Some`");
                self.current = (cur_h, cur_n, PersistedEpoch::Regular(cur_epoch));
                self.next = Some((hash, number, epoch));
                Ok(())
            }
            _ => Err(epoch),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum PersistedEpochHeader {
    /// Genesis persisted epoch header. epoch_0, epoch_1.
    Genesis(EpochHeader, EpochHeader),
    /// Regular persisted epoch header. epoch_n.
    Regular(EpochHeader),
}

impl Clone for PersistedEpochHeader {
    fn clone(&self) -> Self {
        match self {
            Self::Genesis(epoch_0, epoch_1) => Self::Genesis(epoch_0.clone(), epoch_1.clone()),
            Self::Regular(epoch_n) => Self::Regular(epoch_n.clone()),
        }
    }
}

impl PersistedEpochHeader {
    /// Map the epoch header to a different type.
    pub fn map(self) -> PersistedEpochHeader {
        match self {
            PersistedEpochHeader::Genesis(epoch_0, epoch_1) => PersistedEpochHeader::Genesis(
                EpochHeader {
                    start_slot: epoch_0.start_slot,
                    end_slot: epoch_0.end_slot,
                },
                EpochHeader {
                    start_slot: epoch_1.start_slot,
                    end_slot: epoch_1.end_slot,
                },
            ),
            PersistedEpochHeader::Regular(epoch_n) => PersistedEpochHeader::Regular(EpochHeader {
                start_slot: epoch_n.start_slot,
                end_slot: epoch_n.end_slot,
            }),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ViableEpochDescriptor<Hash, Number> {
    /// The epoch is an unimported genesis, with given start slot number.
    UnimportedGenesis(Slot),
    /// The epoch is signaled and has been imported, with given identifier and header.
    Signaled(EpochIdentifier<Hash, Number>, EpochHeader),
}

#[derive(Clone, Encode, Decode, Debug)]
pub struct EpochChanges<Hash, Number> {
    inner: ForkTree<Hash, Number, PersistedEpochHeader>,
    epochs: BTreeMap<(Hash, Number), PersistedEpoch<Epoch>>,
    gap: Option<EpochGap<Hash, Number>>,
}

impl<Hash, Number> Default for EpochChanges<Hash, Number>
where
    Hash: PartialEq + Ord,
    Number: Ord,
{
    fn default() -> Self {
        EpochChanges {
            inner: ForkTree::new(),
            epochs: BTreeMap::new(),
            gap: None,
        }
    }
}

impl<Hash, Number> EpochChanges<Hash, Number>
where
    Hash: PartialEq + Ord + AsRef<[u8]> + AsMut<[u8]> + Copy + std::fmt::Debug,
    Number: Ord + One + Zero + Add<Output = Number> + Sub<Output = Number> + Copy + std::fmt::Debug,
{
    /// Create a new epoch change.
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebalances the tree of epoch changes so that it is sorted by length of
    /// fork (longest fork first).
    pub fn rebalance(&mut self) {
        self.inner.rebalance()
    }

    /// Clear gap epochs if any.
    pub fn clear_gap(&mut self) {
        self.gap = None;
    }

    pub fn map<F>(self, mut f: F) -> EpochChanges<Hash, Number>
    where
        F: FnMut(&Hash, &Number, E) -> B,
    {
        EpochChanges {
            inner: self
                .inner
                .map(&mut |_, _, header: PersistedEpochHeader| header.map()),
            gap: self.gap.map(
                |EpochGap {
                     current_hash: h,
                     current_num: n,
                     current_epoch: header,
                     next,
                 }| EpochGap {
                    current_hash: h,
                    current_num: n,
                    current_epoch: header.map(&h, &n, &mut f),
                    next: next.map(|(h, n, e)| (h, n, f(&h, &n, e))),
                },
            ),
            epochs: self
                .epochs
                .into_iter()
                .map(|((hash, number), epoch)| ((hash, number), epoch.map(&hash, &number, &mut f)))
                .collect(),
        }
    }

    pub fn epoch(&self, id: &EpochIdentifier<Hash, Number>) -> Option<&Epoch> {
        if let Some(e) = &self.gap.as_ref().and_then(|gap| gap.epoch(id)) {
            return Some(e);
        }
        self.epochs
            .get(&(id.hash, id.number))
            .and_then(|v| match v {
                PersistedEpoch::Genesis(ref epoch_0, _)
                    if id.position == EpochIdentifierPosition::Genesis0 =>
                {
                    Some(epoch_0)
                }
                PersistedEpoch::Genesis(_, ref epoch_1)
                    if id.position == EpochIdentifierPosition::Genesis1 =>
                {
                    Some(epoch_1)
                }
                PersistedEpoch::Regular(ref epoch_n)
                    if id.position == EpochIdentifierPosition::Regular =>
                {
                    Some(epoch_n)
                }
                _ => None,
            })
    }

    // Finds the epoch for a child of the given block, assuming the given slot number.
    pub fn epoch_descriptor_for_child_of(
        &self,
        parent_hash: &Hash,
        parent_num: Number,
        slot: Slot,
    ) -> Result<Option<ViableEpochDescriptor<Hash, Number>>, Error> {
        let mut h = parent_hash.clone();
        h.as_mut()[0] ^= 0b10000000;

        if parent_number == Zero::zero() {
            // need to insert the genesis epoch.
            return Ok(Some(ViableEpochDescriptor::UnimportedGenesis(slot)));
        }

        if let Some(gap) = &self.gap {
            if let Some((hash, number, hdr, position)) = gap.matches(slot) {
                return Ok(Some(ViableEpochDescriptor::Signaled(
                    EpochIdentifier {
                        position,
                        hash,
                        num: number,
                    },
                    hdr,
                )));
            }
        }

        let predicate = |epoch: &PersistedEpochHeader| match *epoch {
            PersistedEpochHeader::Genesis(ref epoch_0, _) => epoch_0.start_slot <= slot,
            PersistedEpochHeader::Regular(ref epoch_n) => epoch_n.start_slot <= slot,
        };

        let is_descendent_of = |hash_0: &Hash, hash_1: &Hash| -> Result<bool, Error> {
            todo!("check hash_1 is descendent of hash_0");
            return Ok(false);
        };

        self.inner
            .find_node_where(
                &h,
                &(parent_number + One::one()),
                &is_descendent_of,
                &predicate,
            )
            .map(|n| {
                n.map(|node| {
                    (
                        match node.data {
                            // Ok, we found our node.
                            // and here we figure out which of the internal epochs
                            // of a genesis node to use based on their start slot.
                            PersistedEpochHeader::Genesis(ref epoch_0, ref epoch_1) => {
                                if epoch_1.start_slot <= slot {
                                    (EpochIdentifierPosition::Genesis1, epoch_1.clone())
                                } else {
                                    (EpochIdentifierPosition::Genesis0, epoch_0.clone())
                                }
                            }
                            PersistedEpochHeader::Regular(ref epoch_n) => {
                                (EpochIdentifierPosition::Regular, epoch_n.clone())
                            }
                        },
                        node,
                    )
                })
                .map(|((position, header), node)| {
                    ViableEpochDescriptor::Signaled(
                        EpochIdentifier {
                            position,
                            hash: node.hash,
                            num: node.number,
                        },
                        header,
                    )
                })
            });

        // need check
        todo!()
    }
}
