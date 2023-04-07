use crate::config::{UNIT, UNIT_ZEROS};
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
    pub fn display_by_decimals(&self, decimals: u8) -> String {
        let mut s = self.value.to_string();
        if decimals == 0 {
            return s;
        };
        while s.len() <= decimals as usize {
            s.insert(0, '0');
        }
        s.insert(s.len() - decimals as usize, '.');
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
        return s;
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
        *self = Self::new(u64::from(self.value) + u64::from(other.value))
    }
}

impl SubAssign for Amount {
    fn sub_assign(&mut self, other: Self) {
        *self = Self::new(u64::from(self.value) - u64::from(other.value))
    }
}

impl Add for Amount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(u64::from(self.value) + u64::from(other.value))
    }
}

impl Sub for Amount {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(u64::from(self.value) - u64::from(other.value))
    }
}

impl Div<u64> for Amount {
    type Output = Self;

    fn div(self, other: u64) -> Self {
        Self::new(u64::from(self.value) / other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_by_decimals_func() {
        assert_eq!(Amount::new(0).display_by_decimals(0), "0");
        assert_eq!(Amount::new(1223).display_by_decimals(0), "1223");
        assert_eq!(Amount::new(0).display_by_decimals(UNIT_ZEROS), "0.0");
        assert_eq!(
            Amount::new(1).display_by_decimals(UNIT_ZEROS),
            "0.000000001"
        );
        assert_eq!(
            Amount::new(12).display_by_decimals(UNIT_ZEROS),
            "0.000000012"
        );
        assert_eq!(
            Amount::new(1234).display_by_decimals(UNIT_ZEROS),
            "0.000001234"
        );
        assert_eq!(
            Amount::new(123000000000).display_by_decimals(UNIT_ZEROS),
            "123.0"
        );
        assert_eq!(
            Amount::new(123456789).display_by_decimals(UNIT_ZEROS),
            "0.123456789"
        );
        assert_eq!(
            Amount::new(1234567898).display_by_decimals(UNIT_ZEROS),
            "1.234567898"
        );
        assert_eq!(
            Amount::new(123456789987654321).display_by_decimals(UNIT_ZEROS),
            "123456789.987654321"
        );
        assert_eq!(
            Amount::new(123000000000).display_by_decimals(4),
            "12300000.0"
        );
        assert_eq!(Amount::new(123456789).display_by_decimals(6), "123.456789");
        assert_eq!(Amount::new(123456789).display_by_decimals(9), "0.123456789");
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
