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
pub struct Amount(pub u64);

#[derive(Error, Debug)]
pub enum ParseDecimalError {
    #[error("amount invalid")]
    Invalid,
}

#[derive(Clone, Copy)]
pub struct Decimal {
    pub value: u64,
    pub num_decimals: u8,
}

impl Decimal {
    pub fn to_amount(&self, decimals: u8) -> Amount {
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
        Amount(value)
    }
}

impl FromStr for Decimal {
    type Err = ParseDecimalError;
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

impl Amount {
    pub fn display_by_decimals(&self, decimals: u8) -> String {
        let mut s = self.0.to_string();
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

impl From<Amount> for u64 {
    fn from(a: Amount) -> u64 {
        a.0
    }
}

impl From<u64> for Amount {
    fn from(val: u64) -> Self {
        Self(val)
    }
}

impl AddAssign for Amount {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Amount {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl Add for Amount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl Sub for Amount {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl Div<u64> for Amount {
    type Output = Self;

    fn div(self, other: u64) -> Self {
        Self(self.0 / other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UNIT_ZEROS;

    #[test]
    fn test_decimals() {
        let a: Decimal = "123".parse().unwrap();
        assert_eq!((a.value, a.num_decimals), (123, 0));

        let b: Decimal = "123.45".parse().unwrap();
        assert_eq!((b.value, b.num_decimals), (12345, 2));

        let c: Decimal = "123.450".parse().unwrap();
        assert_eq!((c.value, c.num_decimals), (12345, 2));

        let d: Decimal = "123.450000".parse().unwrap();
        assert_eq!((d.value, d.num_decimals), (12345, 2));

        let e: Decimal = "123.".parse().unwrap();
        assert_eq!((e.value, e.num_decimals), (123, 0));

        let f: Decimal = "123.00".parse().unwrap();
        assert_eq!((f.value, f.num_decimals), (123, 0));

        let g: Decimal = "1.".parse().unwrap();
        assert_eq!((g.value, g.num_decimals), (1, 0));

        let h: Decimal = "1.00".parse().unwrap();
        assert_eq!((h.value, h.num_decimals), (1, 0));

        let i: Decimal = ".1".parse().unwrap();
        assert_eq!((i.value, i.num_decimals), (1, 1));

        let j: Decimal = ".12300".parse().unwrap();
        assert_eq!((j.value, j.num_decimals), (123, 3));
    }

    #[test]
    fn test_display_by_decimals_func() {
        assert_eq!(Amount(0).display_by_decimals(0), "0");
        assert_eq!(Amount(1223).display_by_decimals(0), "1223");
        assert_eq!(Amount(0).display_by_decimals(UNIT_ZEROS), "0.0");
        assert_eq!(Amount(1).display_by_decimals(UNIT_ZEROS), "0.000000001");
        assert_eq!(Amount(12).display_by_decimals(UNIT_ZEROS), "0.000000012");
        assert_eq!(Amount(1234).display_by_decimals(UNIT_ZEROS), "0.000001234");
        assert_eq!(
            Amount(123000000000).display_by_decimals(UNIT_ZEROS),
            "123.0"
        );
        assert_eq!(
            Amount(123456789).display_by_decimals(UNIT_ZEROS),
            "0.123456789"
        );
        assert_eq!(
            Amount(1234567898).display_by_decimals(UNIT_ZEROS),
            "1.234567898"
        );
        assert_eq!(
            Amount(123456789987654321).display_by_decimals(UNIT_ZEROS),
            "123456789.987654321"
        );
        assert_eq!(Amount(123000000000).display_by_decimals(4), "12300000.0");
        assert_eq!(Amount(123456789).display_by_decimals(6), "123.456789");
        assert_eq!(Amount(123456789).display_by_decimals(9), "0.123456789");
    }

    #[test]
    fn test_str_to_amount() {
        assert_eq!(
            "0".parse::<Decimal>().unwrap().to_amount(UNIT_ZEROS),
            Amount(0)
        );
        assert_eq!(
            "0.".parse::<Decimal>().unwrap().to_amount(UNIT_ZEROS),
            Amount(0)
        );
        assert_eq!(
            "0.0".parse::<Decimal>().unwrap().to_amount(UNIT_ZEROS),
            Amount(0)
        );
        assert_eq!(
            "1".parse::<Decimal>().unwrap().to_amount(UNIT_ZEROS),
            Amount(1000000000)
        );
        assert_eq!(
            "1.".parse::<Decimal>().unwrap().to_amount(UNIT_ZEROS),
            Amount(1000000000)
        );
        assert_eq!(
            "1.0".parse::<Decimal>().unwrap().to_amount(UNIT_ZEROS),
            Amount(1000000000)
        );
        assert_eq!(
            "123".parse::<Decimal>().unwrap().to_amount(UNIT_ZEROS),
            Amount(123000000000)
        );
        assert_eq!(
            "123.".parse::<Decimal>().unwrap().to_amount(UNIT_ZEROS),
            Amount(123000000000)
        );
        assert_eq!(
            "123.0".parse::<Decimal>().unwrap().to_amount(UNIT_ZEROS),
            Amount(123000000000)
        );
        assert_eq!(
            "123.1".parse::<Decimal>().unwrap().to_amount(UNIT_ZEROS),
            Amount(123100000000)
        );
        assert_eq!(
            "123.100".parse::<Decimal>().unwrap().to_amount(UNIT_ZEROS),
            Amount(123100000000)
        );
        assert_eq!(
            "123.100000000"
                .parse::<Decimal>()
                .unwrap()
                .to_amount(UNIT_ZEROS),
            Amount(123100000000)
        );
        assert_eq!(
            "123.123456"
                .parse::<Decimal>()
                .unwrap()
                .to_amount(UNIT_ZEROS),
            Amount(123123456000)
        );
        assert_eq!(
            "123.123456000"
                .parse::<Decimal>()
                .unwrap()
                .to_amount(UNIT_ZEROS),
            Amount(123123456000)
        );
        assert_eq!(
            "123.123456789"
                .parse::<Decimal>()
                .unwrap()
                .to_amount(UNIT_ZEROS),
            Amount(123123456789)
        );
        assert_eq!(
            "123.0001".parse::<Decimal>().unwrap().to_amount(UNIT_ZEROS),
            Amount(123000100000)
        );
        assert_eq!(
            "123.000000001"
                .parse::<Decimal>()
                .unwrap()
                .to_amount(UNIT_ZEROS),
            Amount(123000000001)
        );
        assert_eq!(
            "0.0001".parse::<Decimal>().unwrap().to_amount(UNIT_ZEROS),
            Amount(100000)
        );
        assert_eq!(
            "0.000000001"
                .parse::<Decimal>()
                .unwrap()
                .to_amount(UNIT_ZEROS),
            Amount(1)
        );
        assert_eq!(
            ".0001".parse::<Decimal>().unwrap().to_amount(UNIT_ZEROS),
            Amount(100000)
        );
        assert_eq!(
            ".000000001"
                .parse::<Decimal>()
                .unwrap()
                .to_amount(UNIT_ZEROS),
            Amount(1)
        );
        assert_eq!(
            ".123456789"
                .parse::<Decimal>()
                .unwrap()
                .to_amount(UNIT_ZEROS),
            Amount(123456789)
        );
        assert_eq!(
            " 123 ".parse::<Decimal>().unwrap().to_amount(UNIT_ZEROS),
            Amount(123000000000)
        );
        assert_eq!(
            " 123.456 "
                .parse::<Decimal>()
                .unwrap()
                .to_amount(UNIT_ZEROS),
            Amount(123456000000)
        );
        assert!("123.234.123".parse::<Decimal>().is_err());
        assert!("k123".parse::<Decimal>().is_err());
        assert!("12 34".parse::<Decimal>().is_err());
        assert!(".".parse::<Decimal>().is_err());
        assert!(" . ".parse::<Decimal>().is_err());
        assert!("12 .".parse::<Decimal>().is_err());
        assert!(". 12".parse::<Decimal>().is_err());
    }
}
