use std::fmt;
use std::marker::PhantomData;

use serde;
use serde::de::{Deserializer, Error, SeqAccess, Visitor};
use serde::ser::{SerializeTuple, Serializer};

pub trait Array<'de>: Sized {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;
}

macro_rules! serde_num_array_external {
    ($([$typ:ty; $len:expr],)+) => {
        $(
            impl<'de> Array<'de> for [$typ; $len]
            {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where S: Serializer
                {
                    let mut seq = serializer.serialize_tuple(self.len())?;
                    for elem in &self[..] {
                        seq.serialize_element(elem)?;
                    }
                    seq.end()
                }

                fn deserialize<D>(deserializer: D) -> Result<[$typ; $len], D::Error>
                    where D: Deserializer<'de>
                {
                    struct ArrayVisitor {
                        element: PhantomData<$typ>,
                    }

                    impl<'de> Visitor<'de> for ArrayVisitor
                    {
                        type Value = [$typ; $len];

                        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                            formatter.write_str(concat!("an array of length ", $len))
                        }

                        fn visit_seq<A>(self, mut seq: A) -> Result<[$typ; $len], A::Error>
                            where A: SeqAccess<'de>
                        {
                            let mut arr = [0; $len];
                            for i in 0..$len {
                                arr[i] = seq.next_element()?
                                    .ok_or_else(|| Error::invalid_length(i, &self))?;
                            }
                            Ok(arr)
                        }
                    }

                    let visitor = ArrayVisitor { element: PhantomData };
                    deserializer.deserialize_tuple($len, visitor)
                }
            }
        )+
    }
}

serde_num_array_external!([u8; 64], [u8; 256], [u8; 512],);
