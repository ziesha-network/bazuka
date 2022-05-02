use std::mem::MaybeUninit;
use std::{fmt, result};

use serde;
use serde::de::{Error, SeqAccess, Visitor};
use serde::Deserialize;

struct LazyInitialized<T, const N: usize>(Option<MaybeUninit<[T; N]>>, usize);

/// drop from tailing
impl<T, const N: usize> Drop for LazyInitialized<T, N> {
    fn drop(&mut self) {
        unsafe {
            if core::mem::needs_drop::<T>() {
                if let Some(arr) = &mut self.0 {
                    while self.1 > 0 {
                        let offset = self.1;
                        self.1 -= 1;
                        let p = (arr.as_mut_ptr() as *mut T).wrapping_add(offset);
                        core::ptr::drop_in_place::<T>(p);
                    }
                }
            }
        }
    }
}

pub struct ArrayVisitor<T> {
    element: std::marker::PhantomData<T>,
}

impl<T> ArrayVisitor<T> {
    #[inline]
    pub fn new() -> Self {
        ArrayVisitor {
            element: Default::default(),
        }
    }
}

impl<'de, T, const N: usize> Visitor<'de> for ArrayVisitor<[T; N]>
where
    T: Deserialize<'de>,
{
    type Value = [T; N];

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "an array of length {}", N)
    }

    fn visit_seq<A>(self, mut seq: A) -> result::Result<[T; N], A::Error>
    where
        A: SeqAccess<'de>,
    {
        unsafe {
            let mut arr: LazyInitialized<T, N> = LazyInitialized(Some(MaybeUninit::uninit()), 0);
            {
                let p = arr.0.as_mut().unwrap();
                for i in 0..N {
                    let p = (p.as_mut_ptr() as *mut T).wrapping_add(i);
                    core::ptr::write(
                        p,
                        seq.next_element()?
                            .ok_or_else(|| Error::invalid_length(i, &self))?,
                    );
                    arr.1 += 1;
                }
            }
            let initialized = arr.0.take().unwrap().assume_init();
            Ok(initialized)
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_with_serde_derive() {}
}
