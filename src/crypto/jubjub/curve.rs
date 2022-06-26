use std::ops::*;
use std::str::FromStr;

use ff::{Field, PrimeField, PrimeFieldBits};
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};

use crate::zk::ZkScalar;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct PointCompressed(pub ZkScalar, pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct PointAffine(pub ZkScalar, pub ZkScalar);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointProjective(pub ZkScalar, pub ZkScalar, pub ZkScalar);

impl AddAssign<&PointAffine> for PointAffine {
    fn add_assign(&mut self, other: &PointAffine) {
        if *self == *other {
            *self = self.double();
            return;
        }
        let xx = (ZkScalar::one() + *D * self.0 * other.0 * self.1 * other.1)
            .invert()
            .unwrap();
        let yy = (ZkScalar::one() - *D * self.0 * other.0 * self.1 * other.1)
            .invert()
            .unwrap();
        *self = Self(
            (self.0 * other.1 + self.1 * other.0) * xx,
            (self.1 * other.1 - *A * self.0 * other.0) * yy,
        );
    }
}

impl PointAffine {
    pub fn zero() -> Self {
        Self(ZkScalar::zero(), ZkScalar::one())
    }
    pub fn double(&self) -> Self {
        let xx = (*A * self.0 * self.0 + self.1 * self.1).invert().unwrap();
        let yy = (ZkScalar::one() + ZkScalar::one() - *A * self.0 * self.0 - self.1 * self.1)
            .invert()
            .unwrap();
        return Self(
            ((self.0 * self.1) * xx).double(),
            (self.1 * self.1 - *A * self.0 * self.0) * yy,
        );
    }
    pub fn multiply(&self, scalar: &ZkScalar) -> Self {
        let mut result = PointProjective::zero();
        let self_proj = self.to_projective();
        for bit in scalar.to_le_bits().iter().rev() {
            result = result.double();
            if *bit {
                result.add_assign(&self_proj);
            }
        }
        result.to_affine()
    }
    pub fn to_projective(&self) -> PointProjective {
        PointProjective(self.0, self.1, ZkScalar::one())
    }
    pub fn compress(&self) -> PointCompressed {
        PointCompressed(self.0, self.1.is_odd().into())
    }
}

impl PointCompressed {
    pub fn decompress(&self) -> PointAffine {
        let mut y = ((ZkScalar::one() - *D * self.0.square()).invert().unwrap()
            * (ZkScalar::one() - *A * self.0.square()))
        .sqrt()
        .unwrap();
        let is_odd: bool = y.is_odd().into();
        if self.1 != is_odd {
            y = y.neg();
        }
        PointAffine(self.0.clone(), y)
    }
}

impl AddAssign<&PointProjective> for PointProjective {
    fn add_assign(&mut self, other: &PointProjective) {
        if self.is_zero() {
            *self = *other;
            return;
        }
        if other.is_zero() {
            return;
        }
        if self.to_affine() == other.to_affine() {
            *self = self.double();
            return;
        }
        let a = self.2 * other.2; // A = Z1 * Z2
        let b = a.square(); // B = A^2
        let c = self.0 * other.0; // C = X1 * X2
        let d = self.1 * other.1; // D = Y1 * Y2
        let e = *D * c * d; // E = dC · D
        let f = b - e; // F = B − E
        let g = b + e; // G = B + E
        self.0 = a * f * ((self.0 + self.1) * (other.0 + other.1) - c - d);
        self.1 = a * g * (d - *A * c);
        self.2 = f * g;
    }
}

impl PointProjective {
    pub fn zero() -> Self {
        PointProjective(ZkScalar::zero(), ZkScalar::one(), ZkScalar::zero())
    }
    pub fn is_zero(&self) -> bool {
        self.2.is_zero().into()
    }
    pub fn double(&self) -> PointProjective {
        if self.is_zero() {
            return PointProjective::zero();
        }
        let b = (self.0 + self.1).square();
        let c = self.0.square();
        let d = self.1.square();
        let e = *A * c;
        let f = e + d;
        let h = self.2.square();
        let j = f - h.double();
        PointProjective((b - c - d) * j, f * (e - d), f * j)
    }
    pub fn to_affine(&self) -> PointAffine {
        if self.is_zero() {
            return PointAffine::zero();
        }
        let zinv = self.2.invert().unwrap();
        PointAffine(self.0 * zinv, self.1 * zinv)
    }
}

lazy_static! {
    pub static ref A: ZkScalar = ZkScalar::one().neg();
    pub static ref D: ZkScalar = ZkScalar::from_str_vartime(
        "19257038036680949359750312669786877991949435402254120286184196891950884077233"
    )
    .unwrap();
    pub static ref BASE: PointAffine = PointAffine(
        ZkScalar::from_str_vartime(
            "28867639725710769449342053336011988556061781325688749245863888315629457631946"
        )
        .unwrap(),
        ZkScalar::from_str_vartime("18").unwrap()
    );
    pub static ref ORDER: BigUint = BigUint::from_str(
        "6554484396890773809930967563523245729705921265872317281365359162392183254199"
    )
    .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_twisted_edwards_curve_ops() {
        // ((2G) + G) + G
        let mut a = BASE.double();
        a.add_assign(&BASE);
        a.add_assign(&BASE);

        // 2(2G)
        let b = BASE.double().double();

        assert_eq!(a, b);

        // G + G + G + G
        let mut c = BASE.clone();
        c.add_assign(&BASE);
        c.add_assign(&BASE);
        c.add_assign(&BASE);

        assert_eq!(b, c);

        // Check if projective points are working
        let mut pnt1 = BASE.to_projective().double().double();
        pnt1.add_assign(&BASE.to_projective());
        let mut pnt2 = BASE.double().double();
        pnt2.add_assign(&BASE);

        assert_eq!(pnt1.to_affine(), pnt2);
    }
}
