use std::ops::*;
use std::str::FromStr;

use ff::{Field, PrimeField};
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};

use super::field::Fr;
use crate::core::number::U256;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointCompressed(pub Fr, pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointAffine(pub Fr, pub Fr);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointProjective(pub Fr, pub Fr, pub Fr);

impl AddAssign<&PointAffine> for PointAffine {
    fn add_assign(&mut self, other: &PointAffine) {
        if *self == *other {
            *self = self.double();
            return;
        }
        let xx = (Fr::one() + *D * self.0 * other.0 * self.1 * other.1)
            .invert()
            .unwrap();
        let yy = (Fr::one() - *D * self.0 * other.0 * self.1 * other.1)
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
        Self(Fr::zero(), Fr::one())
    }
    pub fn double(&self) -> Self {
        let xx = (*A * self.0 * self.0 + self.1 * self.1).invert().unwrap();
        let yy = (Fr::one() + Fr::one() - *A * self.0 * self.0 - self.1 * self.1)
            .invert()
            .unwrap();
        return Self(
            ((self.0 * self.1) * xx).double(),
            (self.1 * self.1 - *A * self.0 * self.0) * yy,
        );
    }
    pub fn multiply(&self, scalar: &U256) -> Self {
        let mut result = PointProjective::zero();
        let self_proj = self.to_projective();
        for bit in scalar.to_bits().iter().rev() {
            result = result.double();
            if *bit {
                result.add_assign(&self_proj);
            }
        }
        result.to_affine()
    }
    pub fn to_projective(&self) -> PointProjective {
        PointProjective(self.0, self.1, Fr::one())
    }
    pub fn compress(&self) -> PointCompressed {
        PointCompressed(self.0, self.1.is_odd().into())
    }
}

impl PointCompressed {
    pub fn decompress(&self) -> PointAffine {
        let mut y = ((Fr::one() - *D * self.0.square()).invert().unwrap()
            * (Fr::one() - *A * self.0.square()))
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
        PointProjective(Fr::zero(), Fr::one(), Fr::zero())
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
    static ref A: Fr = Fr::one().neg();
    static ref D: Fr = Fr::from_str_vartime(
        "12181644023421730124874158521699555681764249180949974110617291017600649128846"
    )
    .unwrap();
    pub static ref BASE: PointAffine = PointAffine(
        Fr::from_str_vartime(
            "9671717474070082183213120605117400219616337014328744928644933853176787189663"
        )
        .unwrap(),
        Fr::from_str_vartime(
            "16950150798460657717958625567821834550301663161624707787222815936182638968203"
        )
        .unwrap()
    );
    pub static ref ORDER: BigUint = BigUint::from_str(
        "2736030358979909402780800718157159386076813972158567259200215660948447373041"
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
