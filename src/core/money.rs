use crate::config::{SYMBOL, UNIT};
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
        write!(f, "{:.1}{}", (self.0 as f64 / UNIT as f64), SYMBOL)
    }
}

impl FromStr for Money {
    type Err = ParseMoneyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let as_float: f64 = s.parse().map_err(|_| ParseMoneyError::Invalid)?;
        Ok(Self((as_float * UNIT as f64) as u64))
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
