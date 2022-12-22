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
pub struct Money(pub u64);

#[derive(Error, Debug)]
pub enum ParseMoneyError {
    #[error("money invalid")]
    Invalid,
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut s = self.0.to_string();
        while s.len() <= UNIT_ZEROS as usize {
            s.insert(0, '0');
        }
        s.insert(s.len() - UNIT_ZEROS as usize, '.');
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

impl FromStr for Money {
    type Err = ParseMoneyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = s.trim().to_string();
        if let Some(dot_pos) = s.find('.') {
            if s == "." {
                return Err(ParseMoneyError::Invalid);
            }
            let dot_rpos = s.len() - 1 - dot_pos;
            if dot_rpos > UNIT_ZEROS as usize {
                return Err(ParseMoneyError::Invalid);
            }
            for _ in 0..UNIT_ZEROS as usize - dot_rpos {
                s.push('0');
            }
            s.remove(dot_pos);
            Ok(Self(s.parse().map_err(|_| ParseMoneyError::Invalid)?))
        } else {
            let as_u64: u64 = s.parse().map_err(|_| ParseMoneyError::Invalid)?;
            Ok(Self(as_u64 * UNIT))
        }
    }
}

impl Into<u64> for Money {
    fn into(self) -> u64 {
        self.0
    }
}

impl From<u64> for Money {
    fn from(val: u64) -> Self {
        Self(val)
    }
}

impl AddAssign for Money {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Money {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl Add for Money {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl Sub for Money {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl Div<u64> for Money {
    type Output = Self;

    fn div(self, other: u64) -> Self {
        Self(self.0 / other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_money_to_str() {
        assert_eq!(format!("{}", Money(0)), "0.0");
        assert_eq!(format!("{}", Money(1)), "0.000000001");
        assert_eq!(format!("{}", Money(12)), "0.000000012");
        assert_eq!(format!("{}", Money(1234)), "0.000001234");
        assert_eq!(format!("{}", Money(123000000000)), "123.0");
        assert_eq!(format!("{}", Money(123456789)), "0.123456789");
        assert_eq!(format!("{}", Money(1234567898)), "1.234567898");
        assert_eq!(
            format!("{}", Money(123456789987654321)),
            "123456789.987654321"
        );
    }

    #[test]
    fn test_str_to_money() {
        assert_eq!("0".parse::<Money>().unwrap(), Money(0));
        assert_eq!("0.".parse::<Money>().unwrap(), Money(0));
        assert_eq!("0.0".parse::<Money>().unwrap(), Money(0));
        assert_eq!("1".parse::<Money>().unwrap(), Money(1000000000));
        assert_eq!("1.".parse::<Money>().unwrap(), Money(1000000000));
        assert_eq!("1.0".parse::<Money>().unwrap(), Money(1000000000));
        assert_eq!("123".parse::<Money>().unwrap(), Money(123000000000));
        assert_eq!("123.".parse::<Money>().unwrap(), Money(123000000000));
        assert_eq!("123.0".parse::<Money>().unwrap(), Money(123000000000));
        assert_eq!("123.1".parse::<Money>().unwrap(), Money(123100000000));
        assert_eq!("123.100".parse::<Money>().unwrap(), Money(123100000000));
        assert_eq!(
            "123.100000000".parse::<Money>().unwrap(),
            Money(123100000000)
        );
        assert_eq!("123.123456".parse::<Money>().unwrap(), Money(123123456000));
        assert_eq!(
            "123.123456000".parse::<Money>().unwrap(),
            Money(123123456000)
        );
        assert_eq!(
            "123.123456789".parse::<Money>().unwrap(),
            Money(123123456789)
        );
        assert_eq!("123.0001".parse::<Money>().unwrap(), Money(123000100000));
        assert_eq!(
            "123.000000001".parse::<Money>().unwrap(),
            Money(123000000001)
        );
        assert_eq!("0.0001".parse::<Money>().unwrap(), Money(100000));
        assert_eq!("0.000000001".parse::<Money>().unwrap(), Money(1));
        assert_eq!(".0001".parse::<Money>().unwrap(), Money(100000));
        assert_eq!(".000000001".parse::<Money>().unwrap(), Money(1));
        assert_eq!(".123456789".parse::<Money>().unwrap(), Money(123456789));
        assert_eq!(" 123 ".parse::<Money>().unwrap(), Money(123000000000));
        assert_eq!(" 123.456 ".parse::<Money>().unwrap(), Money(123456000000));
        assert!("123.234.123".parse::<Money>().is_err());
        assert!("k123".parse::<Money>().is_err());
        assert!("12 34".parse::<Money>().is_err());
        assert!(".".parse::<Money>().is_err());
        assert!(" . ".parse::<Money>().is_err());
        assert!("12 .".parse::<Money>().is_err());
        assert!(". 12".parse::<Money>().is_err());
    }
}
