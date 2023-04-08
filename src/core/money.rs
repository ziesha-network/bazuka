use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, Sub, SubAssign};
use std::str::FromStr;
use thiserror::Error;

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    PartialOrd,
    Ord,
    Eq,
    Default,
)]
pub struct Amount {
    pub value: u64,
    pub num_decimals: u8,
}

impl<'a> Sum<&'a Self> for Amount {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a Self>,
    {
        iter.fold(Self::new(0), |a, b| a + *b)
    }
}

#[derive(Error, Debug)]
pub enum ParseAmountError {
    #[error("amount invalid")]
    Invalid,
}

impl Amount {
    pub fn new(value: u64) -> Self {
        Amount {
            value,
            num_decimals: 0,
        }
    }
    pub fn normalize(&self, decimals: u8) -> u64 {
        let mut value = self.value;
        if self.num_decimals < decimals {
            for _ in self.num_decimals..decimals {
                value = value.saturating_mul(10);
            }
        } else {
            for _ in decimals..self.num_decimals {
                value = value.saturating_div(10);
            }
        }
        value
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = self.value.to_string();
        if self.num_decimals == 0 {
            return write!(f, "{}", s);
        };
        while s.len() <= self.num_decimals as usize {
            s.insert(0, '0');
        }
        s.insert(s.len() - self.num_decimals as usize, '.');
        while let Some(last) = s.chars().last() {
            if last == '0' {
                s.pop();
            } else {
                break;
            }
        }
        if let Some(last) = s.chars().last() {
            if last == '.' {
                s.push('0');
            }
        }
        write!(f, "{}", s)
    }
}

impl FromStr for Amount {
    type Err = ParseAmountError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = s.trim().to_string();
        if let Some(dot_pos) = s.find('.') {
            if s == "." {
                return Err(Self::Err::Invalid);
            }
            while let Some(last) = s.chars().last() {
                if last == '0' {
                    s.pop();
                } else {
                    break;
                }
            }
            let num_decimals = (s.len() - dot_pos - 1)
                .try_into()
                .map_err(|_| Self::Err::Invalid)?;
            s.remove(dot_pos);
            let value: u64 = s.parse().map_err(|_| Self::Err::Invalid)?;
            Ok(Self {
                value,
                num_decimals,
            })
        } else {
            let value: u64 = s.parse().map_err(|_| Self::Err::Invalid)?;
            Ok(Self {
                value,
                num_decimals: 0,
            })
        }
    }
}

impl From<u64> for Amount {
    fn from(val: u64) -> Self {
        Self::new(val)
    }
}

impl AddAssign for Amount {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl SubAssign for Amount {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl Add for Amount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let num_decimals = std::cmp::max(self.num_decimals, other.num_decimals);
        let value = self.normalize(num_decimals) + other.normalize(num_decimals);
        Self {
            value,
            num_decimals,
        }
    }
}

impl Sub for Amount {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let num_decimals = std::cmp::max(self.num_decimals, other.num_decimals);
        let value = self.normalize(num_decimals) - other.normalize(num_decimals);
        Self {
            value,
            num_decimals,
        }
    }
}

impl Div for Amount {
    type Output = f64;

    fn div(self, other: Self) -> f64 {
        self.value as f64 / other.value as f64
    }
}

impl Div<u64> for Amount {
    type Output = Self;

    fn div(self, other: u64) -> Self {
        let normalized = self.normalize(0);
        Self::new(normalized / other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DECIMALS;

    #[test]
    fn test_amount_normalize() {
        assert_eq!(
            Amount {
                value: 0,
                num_decimals: 0
            }
            .normalize(3),
            0
        );
        assert_eq!(
            Amount {
                value: 123456,
                num_decimals: 3
            }
            .normalize(3),
            123456
        );
        assert_eq!(
            Amount {
                value: 123456,
                num_decimals: 3
            }
            .normalize(2),
            12345
        );
        assert_eq!(
            Amount {
                value: 123456,
                num_decimals: 3
            }
            .normalize(4),
            1234560
        );
        assert_eq!(
            Amount {
                value: 123456,
                num_decimals: 3
            }
            .normalize(10),
            1234560000000
        );
        assert_eq!(
            Amount {
                value: 123456,
                num_decimals: 3
            }
            .normalize(100),
            u64::MAX
        );
        assert_eq!(
            Amount {
                value: 123456,
                num_decimals: 100
            }
            .normalize(9),
            0
        );
    }

    #[test]
    fn test_display_by_decimals_func() {
        assert_eq!(
            &Amount {
                value: 1234,
                num_decimals: DECIMALS
            }
            .to_string(),
            "0.000001234"
        );
        assert_eq!(
            &Amount {
                value: 123000000000,
                num_decimals: DECIMALS
            }
            .to_string(),
            "123.0"
        );
        assert_eq!(
            &Amount {
                value: 123456789,
                num_decimals: DECIMALS
            }
            .to_string(),
            "0.123456789"
        );
        assert_eq!(
            &Amount {
                value: 1234567898,
                num_decimals: DECIMALS
            }
            .to_string(),
            "1.234567898"
        );
        assert_eq!(
            &Amount {
                value: 123456789987654321,
                num_decimals: DECIMALS
            }
            .to_string(),
            "123456789.987654321"
        );
        assert_eq!(
            &Amount {
                value: 123000000000,
                num_decimals: DECIMALS
            }
            .to_string(),
            "12300000.0"
        );
        assert_eq!(
            &Amount {
                value: 123456789,
                num_decimals: DECIMALS
            }
            .to_string(),
            "123.456789"
        );
        assert_eq!(
            &Amount {
                value: 123456789,
                num_decimals: DECIMALS
            }
            .to_string(),
            "0.123456789"
        );
    }

    #[test]
    fn test_str_to_amount() {
        assert_eq!(
            "0".parse::<Amount>().unwrap(),
            Amount {
                value: 0,
                num_decimals: 0
            }
        );
        assert_eq!(
            "0.".parse::<Amount>().unwrap(),
            Amount {
                value: 0,
                num_decimals: 0
            }
        );
        assert_eq!(
            "0.0".parse::<Amount>().unwrap(),
            Amount {
                value: 0,
                num_decimals: 0
            }
        );
        assert_eq!(
            "1".parse::<Amount>().unwrap(),
            Amount {
                value: 1,
                num_decimals: 0
            }
        );
        assert_eq!(
            "1.".parse::<Amount>().unwrap(),
            Amount {
                value: 1,
                num_decimals: 0
            }
        );
        assert_eq!(
            "1.0".parse::<Amount>().unwrap(),
            Amount {
                value: 1,
                num_decimals: 0
            }
        );
        assert_eq!(
            "123".parse::<Amount>().unwrap(),
            Amount {
                value: 123,
                num_decimals: 0
            }
        );
        assert_eq!(
            "123.".parse::<Amount>().unwrap(),
            Amount {
                value: 123,
                num_decimals: 0
            }
        );
        assert_eq!(
            "123.0".parse::<Amount>().unwrap(),
            Amount {
                value: 123,
                num_decimals: 0
            }
        );
        assert_eq!(
            "123.1".parse::<Amount>().unwrap(),
            Amount {
                value: 1231,
                num_decimals: 1
            }
        );
        assert_eq!(
            "123.100".parse::<Amount>().unwrap(),
            Amount {
                value: 1231,
                num_decimals: 1
            }
        );
        assert_eq!(
            "123.100000000".parse::<Amount>().unwrap(),
            Amount {
                value: 1231,
                num_decimals: 1
            }
        );
        assert_eq!(
            "123.123456".parse::<Amount>().unwrap(),
            Amount {
                value: 123123456,
                num_decimals: 6
            }
        );
        assert_eq!(
            "123.123456000".parse::<Amount>().unwrap(),
            Amount {
                value: 123123456,
                num_decimals: 6
            }
        );
        assert_eq!(
            "123.123456789".parse::<Amount>().unwrap(),
            Amount {
                value: 123123456789,
                num_decimals: 9
            }
        );
        assert_eq!(
            "123.0001".parse::<Amount>().unwrap(),
            Amount {
                value: 1230001,
                num_decimals: 4
            }
        );
        assert_eq!(
            "123.000000001".parse::<Amount>().unwrap(),
            Amount {
                value: 123000000001,
                num_decimals: 9
            }
        );
        assert_eq!(
            "0.0001".parse::<Amount>().unwrap(),
            Amount {
                value: 1,
                num_decimals: 4
            }
        );
        assert_eq!(
            "0.000000001".parse::<Amount>().unwrap(),
            Amount {
                value: 1,
                num_decimals: 9
            }
        );
        assert_eq!(
            ".0001".parse::<Amount>().unwrap(),
            Amount {
                value: 1,
                num_decimals: 4
            }
        );
        assert_eq!(
            ".000000001".parse::<Amount>().unwrap(),
            Amount {
                value: 1,
                num_decimals: 9
            }
        );
        assert_eq!(
            ".123456789".parse::<Amount>().unwrap(),
            Amount {
                value: 123456789,
                num_decimals: 9
            }
        );
        assert_eq!(
            " 123 ".parse::<Amount>().unwrap(),
            Amount {
                value: 123,
                num_decimals: 0
            }
        );
        assert_eq!(
            " 123.456 ".parse::<Amount>().unwrap(),
            Amount {
                value: 123456,
                num_decimals: 3
            }
        );
        assert!("123.234.123".parse::<Amount>().is_err());
        assert!("k123".parse::<Amount>().is_err());
        assert!("12 34".parse::<Amount>().is_err());
        assert!(".".parse::<Amount>().is_err());
        assert!(" . ".parse::<Amount>().is_err());
        assert!("12 .".parse::<Amount>().is_err());
        assert!(". 12".parse::<Amount>().is_err());
    }
}
