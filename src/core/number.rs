use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

macro_rules! impl_map_from_u256 {
    ($primitive:ty, $as_func:ident) => {
        impl From<$primitive> for U256 {
            fn from(value: $primitive) -> U256 {
                use std::{mem, slice};
                let from_p: *const $primitive = &value;
                let binary_p: *const u8 = from_p as *const _;
                let binary_s: &[u8] =
                    unsafe { slice::from_raw_parts(binary_p, mem::size_of::<$primitive>()) };
                U256(copy_into_array::<[u8; 32], u8>(binary_s))
            }
        }

        impl From<U256> for $primitive {
            fn from(u: U256) -> Self {
                u.$as_func()
            }
        }
    };
}

/// Little-endian large integer type
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
pub struct U256(pub [u8; 32]);

impl Default for U256 {
    fn default() -> Self {
        U256::zero()
    }
}

impl_map_from_u256!(u64, as_u64);
impl_map_from_u256!(u32, as_u32);

impl U256 {
    pub fn zero() -> Self {
        Self([0u8; 32])
    }
    pub fn empty() -> Self {
        Self([0u8; 32])
    }

    pub fn from_le_bytes(bytes: &[u8]) -> Self {
        let mut data = [0u8; 32];
        data[..bytes.len()].copy_from_slice(bytes);
        Self(data)
    }

    pub fn from_be_bytes(bytes: &[u8]) -> Self {
        let mut data = [0u8; 32];
        data[..bytes.len()].copy_from_slice(bytes);
        let mut ret = [0; 32];
        for i in 0..32 {
            ret[32 - i - 1] = data[i];
        }
        Self(ret)
    }

    pub fn random<R: RngCore>(rng: &mut R) -> Self {
        let mut data = [0u8; 32];
        rng.fill_bytes(&mut data);
        Self(data)
    }

    pub fn to_bits(&self) -> [bool; 256] {
        let mut ret = [false; 256];
        for i in 0..256 {
            ret[i] = ((self.0[i / 8] >> (i % 8)) & 1) == 1;
        }
        ret
    }

    pub fn to_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn generate(data: &[u8]) -> Self {
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        Self(hasher.finalize().try_into().unwrap())
    }

    // Dummy implementation of a 512-bit hash function, used for generating
    // scalar and randomness of EdDSA signatures.
    pub fn generate_two(data: &[u8]) -> (Self, Self) {
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        let first: [u8; 32] = hasher.finalize().try_into().unwrap();

        let mut hasher = Sha3_256::new();
        hasher.update(data);
        hasher.update(data);
        let second: [u8; 32] = hasher.finalize().try_into().unwrap();

        (Self(first), Self(second))
    }

    /// panic, a word == an u8
    #[inline]
    fn fits_n_word(&self, n: usize) -> bool {
        assert!(n < 32, "only could check bit before 32, but {}", n);
        let &U256(ref arr) = self;
        for i in n..32 {
            if arr[i] != 0 {
                return false;
            }
        }
        true
    }

    /// panic
    #[inline]
    pub fn as_u64(&self) -> u64 {
        let &U256(ref arr) = self;
        if !self.fits_n_word(8) {
            panic!("Integer overflow when casting to u64")
        }
        let mut ret = 0u64;
        for i in 0..8 {
            ret += (arr[i] as u64) << (i * 8)
        }
        ret
    }

    /// panic
    #[inline]
    pub fn as_u32(&self) -> u32 {
        let &U256(ref arr) = self;
        if !self.fits_n_word(4) {
            panic!("Integer overflow when casting to u32")
        }
        let mut ret = 0u32;
        for i in 0..4 {
            ret += (arr[i] as u32) << (i * 8)
        }
        ret
    }
}

fn copy_into_array<A, T>(slice: &[T]) -> A
where
    A: Default + std::convert::AsMut<[T]>,
    T: Copy,
{
    let mut a = A::default();
    let s = <A as AsMut<[T]>>::as_mut(&mut a);
    assert!(
        s.len() >= slice.len(),
        "the length of dst array must not be smaller than input slice, input length: {}",
        slice.len()
    );
    s[..slice.len()].copy_from_slice(slice);
    a
}

#[cfg(test)]
mod tests {
    use crate::core::number::*;

    #[test]
    fn test_little_endian_format_as_std() {
        macro_rules! test_uint_primitive_ok {
            ($primitive: ty, $num:expr) => {
                assert_eq!(
                    U256::from($num).0,
                    copy_into_array::<[u8; 32], u8>(&$num.to_le_bytes())
                );
            };
        }

        test_uint_primitive_ok!(u64, 1u64);
        test_uint_primitive_ok!(u64, 254u64);
        test_uint_primitive_ok!(u64, 256u64);
        test_uint_primitive_ok!(u32, 1u32);
        test_uint_primitive_ok!(u32, 254u32);
        test_uint_primitive_ok!(u32, 256u32);
    }

    #[test]
    fn test_from_primitive() {
        macro_rules! test_uint_eq_ok {
            ($num:expr, $as_func:ident, $typ:ty) => {
                assert_eq!(U256::from($num as $typ).$as_func(), $num as $typ);
            };
        }

        test_uint_eq_ok!(1, as_u64, u64);
        test_uint_eq_ok!(65523, as_u64, u64);

        test_uint_eq_ok!(2324, as_u32, u32);
        test_uint_eq_ok!(1, as_u32, u32);
    }
}
