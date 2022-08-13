use crate::config::{SYMBOL, UNIT, UNIT_ZEROS};
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
        write!(f, "{}{}", s, SYMBOL)
    }
}

impl FromStr for Money {
    type Err = ParseMoneyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = s.trim().to_string();
        if let Some(dot_pos) = s.find('.') {
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
        assert_eq!(format!("{}", Money(0)), format!("0.{}", SYMBOL));
        assert_eq!(format!("{}", Money(1)), format!("0.000000001{}", SYMBOL));
        assert_eq!(format!("{}", Money(12)), format!("0.000000012{}", SYMBOL));
        assert_eq!(format!("{}", Money(1234)), format!("0.000001234{}", SYMBOL));
        assert_eq!(
            format!("{}", Money(123456789)),
            format!("0.123456789{}", SYMBOL)
        );
        assert_eq!(
            format!("{}", Money(1234567898)),
            format!("1.234567898{}", SYMBOL)
        );
        assert_eq!(
            format!("{}", Money(123456789987654321)),
            format!("123456789.987654321{}", SYMBOL)
        );
    }
}
