use std::ops::{Add, AddAssign, BitAnd, BitOr, BitOrAssign, Mul, Not, Sub};
#[derive(Clone, Copy)]
pub struct Mask<const N: usize>([bool; N]);
impl<const N: usize> Mask<N> {
    pub fn any(self) -> bool {
        self.0.into_iter().any(|x| x)
    }
    pub fn to_array(self) -> [bool; N] {
        self.0
    }
    pub fn select<V: Select<N>>(self, yes: V, no: V) -> V {
        V::select(self, yes, no)
    }
}
pub trait Select<const N: usize> {
    fn select(mask: Mask<N>, yes: Self, no: Self) -> Self;
}
impl<const N: usize> Not for Mask<N> {
    type Output = Self;
    fn not(self) -> Self {
        Self(self.0.map(|x| !x))
    }
}
impl<const N: usize> BitOr for Mask<N> {
    type Output = Self;
    fn bitor(self, r: Self) -> Self {
        Self(std::array::from_fn(|i| self.0[i] | r.0[i]))
    }
}
impl<const N: usize> BitAnd for Mask<N> {
    type Output = Self;
    fn bitand(self, r: Self) -> Self {
        Self(std::array::from_fn(|i| self.0[i] & r.0[i]))
    }
}
impl<const N: usize> BitOrAssign for Mask<N> {
    fn bitor_assign(&mut self, r: Self) {
        *self = *self | r;
    }
}
macro_rules! vector {
    ($name:ident, $float:ty, $n:expr) => {
        #[derive(Clone, Copy)]
        pub struct $name([$float; $n]);
        impl $name {
            pub fn splat(x: $float) -> Self {
                Self([x; $n])
            }
            pub fn from_slice(x: &[$float]) -> Self {
                Self(std::array::from_fn(|i| x[i]))
            }
            pub fn copy_to_slice(self, out: &mut [$float]) {
                out.copy_from_slice(&self.0);
            }
            pub fn reduce_sum(self) -> $float {
                self.0.into_iter().sum()
            }
            pub fn reduce_min(self) -> $float {
                self.0.into_iter().fold(<$float>::INFINITY, <$float>::min)
            }
            pub fn reduce_max(self) -> $float {
                self.0
                    .into_iter()
                    .fold(<$float>::NEG_INFINITY, <$float>::max)
            }
            pub fn simd_min(self, rhs: Self) -> Self {
                Self(std::array::from_fn(|i| self.0[i].min(rhs.0[i])))
            }
            pub fn simd_max(self, rhs: Self) -> Self {
                Self(std::array::from_fn(|i| self.0[i].max(rhs.0[i])))
            }
            pub fn is_finite(self) -> Mask<$n> {
                Mask(self.0.map(<$float>::is_finite))
            }
            pub fn is_nan(self) -> Mask<$n> {
                Mask(self.0.map(<$float>::is_nan))
            }
        }
        impl Select<$n> for $name {
            fn select(m: Mask<$n>, y: Self, n: Self) -> Self {
                Self(std::array::from_fn(
                    |i| if m.0[i] { y.0[i] } else { n.0[i] },
                ))
            }
        }
        impl Add for $name {
            type Output = Self;
            fn add(self, r: Self) -> Self {
                Self(std::array::from_fn(|i| self.0[i] + r.0[i]))
            }
        }
        impl Sub for $name {
            type Output = Self;
            fn sub(self, r: Self) -> Self {
                Self(std::array::from_fn(|i| self.0[i] - r.0[i]))
            }
        }
        impl Mul for $name {
            type Output = Self;
            fn mul(self, r: Self) -> Self {
                Self(std::array::from_fn(|i| self.0[i] * r.0[i]))
            }
        }
        impl AddAssign for $name {
            fn add_assign(&mut self, r: Self) {
                *self = *self + r;
            }
        }
    };
}
vector!(F64x4, f64, 4);
vector!(F32x8, f32, 8);

impl F64x4 {
    pub fn to_array(self) -> [f64; 4] {
        self.0
    }
    pub fn abs(self) -> Self {
        Self(self.0.map(f64::abs))
    }
    pub fn is_infinite(self) -> Mask<4> {
        Mask(self.0.map(f64::is_infinite))
    }
}
