use std::sync::RwLock;

use anyhow::Result;
use lru::LruCache;
use primitive_types::U256;

use crate::core::header::Header;

const LRU_SIZE: usize = 4_096;

pub trait BackendHeader {
    fn get_header_meta(&self, hash: &[u8; 32]) -> Result<HeaderMeta>;
    fn insert_header_meta(&self, hash: [u8; 32], header: HeaderMeta);
    fn remove_header(&self, hash: &[u8; 32]);
}

pub struct HeaderMetaCache(RwLock<LruCache<[u8; 32], HeaderMeta>>);

impl HeaderMetaCache {
    pub fn new(capacity: usize) -> HeaderMetaCache {
        HeaderMetaCache(RwLock::new(LruCache::new(capacity)))
    }
}

impl HeaderMetaCache {
    pub fn get_header_meta(&self, hash: &[u8; 32]) -> Result<Option<HeaderMeta>> {
        Ok(self.0.write()?.get(hash).cloned())
    }

    pub fn insert_header_meta(&self, hash: [u8; 32], meta: HeaderMeta) -> Result<()> {
        self.0.write()?.put(hash, meta);
        Ok(())
    }

    pub fn remove_header_meta(&self, hash: &[u8; 32]) -> Result<()> {
        self.0.write()?.pop(hash);
        Ok(())
    }
}

impl Default for HeaderMetaCache {
    fn default() -> Self {
        HeaderMetaCache(RwLock::new(LruCache::new(LRU_SIZE)))
    }
}

#[derive(Clone, Debug)]
pub struct HeaderMeta {
    pub hash: [u8; 32],
    pub number: U256,
    pub parent: [u8; 32],
    pub state_root: [u8; 32],
    ancestor: [u8; 32],
}

impl From<&Header> for HeaderMeta {
    fn from(header: &Header) -> Self {
        HeaderMeta {
            hash: header.hash.clone(),
            number: header.number.clone(),
            parent: header.parent_hash.clone(),
            state_root: header.state_root.clone(),
            ancestor: header.parent_hash.clone(),
        }
    }
}

pub fn lowest_common_ancestor<B>(
    backend: &B,
    hash_a: &[u8; 32],
    hash_b: &[u8; 32],
) -> Result<([u8; 32], U256)>
where
    B: BackendHeader,
{
    let header_a = backend.get_header_meta(hash_a)?;
    let header_b = backend.get_header_meta(hash_b)?;

    let linear_track = |left: &HeaderMeta, right: &HeaderMeta, backend: &B| {
        let mut result = left.clone();
        while left.number > right.number {
            let ancestor = backend.get_header_meta(&left.ancestor)?;
            if ancestor.number >= right.number {
                result = ancestor
            } else {
                break;
            }
        }
        Ok(result)
    };

    let mut header_a = linear_track(&header_a, &header_b, backend)?;
    let mut header_b = linear_track(&header_b, &header_a, backend)?;

    while header_a.hash != header_b.hash {
        if header_a.number > header_b.number {
            header_a = backend.get_header_meta(&header_a.parent)?;
        } else {
            header_b = backend.get_header_meta(&header_b.parent)?;
        }
    }
    Ok((header_a.hash, header_a.number))
}
