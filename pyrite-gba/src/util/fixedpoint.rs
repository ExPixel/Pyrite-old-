#[derive(Copy, Clone)]
pub struct FixedPoint16 {
    inner: i16,
}

impl FixedPoint16 {
    #[inline]
    pub const fn wrap(value: i16) -> Self {
        FixedPoint16 { inner: value }
    }

    #[inline]
    pub const fn to_inner(self) -> i16 {
        self.inner
    }

    #[inline]
    pub const fn integer(self) -> i16 {
        self.inner >> 8
    }

    #[inline]
    pub const fn fractional(self) -> u16 {
        (self.inner & 0xFF) as u16
    }
}

impl Default for FixedPoint16 {
    fn default() -> Self {
        Self::wrap(0)
    }
}

impl std::ops::Add for FixedPoint16 {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        FixedPoint16::wrap(self.inner.wrapping_add(other.inner))
    }
}

impl std::ops::AddAssign for FixedPoint16 {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        self.inner = self.inner.wrapping_add(other.inner);
    }
}

impl std::ops::Sub for FixedPoint16 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        FixedPoint16::wrap(self.inner.wrapping_sub(other.inner))
    }
}

impl std::ops::SubAssign for FixedPoint16 {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        self.inner = self.inner.wrapping_sub(other.inner);
    }
}

impl std::ops::Mul for FixedPoint16 {
    type Output = Self;
    #[inline]
    fn mul(self, other: Self) -> Self {
        FixedPoint16::wrap(self.inner.wrapping_mul(other.inner) >> 8)
    }
}

impl std::ops::MulAssign for FixedPoint16 {
    #[inline]
    fn mul_assign(&mut self, other: Self) {
        self.inner = self.inner.wrapping_mul(other.inner) >> 8;
    }
}

impl std::ops::Neg for FixedPoint16 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        FixedPoint16::wrap(-self.inner)
    }
}

#[derive(Copy, Clone)]
pub struct FixedPoint32 {
    inner: i32,
}

impl FixedPoint32 {
    #[inline]
    pub const fn wrap(value: i32) -> Self {
        FixedPoint32 { inner: value }
    }

    #[inline]
    pub const fn to_inner(self) -> i32 {
        self.inner
    }

    #[inline]
    pub const fn integer(self) -> i32 {
        self.inner >> 8
    }

    #[inline]
    pub const fn fractional(self) -> u32 {
        (self.inner & 0xFF) as u32
    }
}

impl Default for FixedPoint32 {
    fn default() -> Self {
        Self::wrap(0)
    }
}

impl std::convert::From<u32> for FixedPoint32 {
    #[inline]
    fn from(original: u32) -> Self {
        FixedPoint32::wrap((original as i32) << 8)
    }
}

impl std::convert::From<u16> for FixedPoint32 {
    #[inline]
    fn from(original: u16) -> Self {
        FixedPoint32::wrap((original as u32 as i32) << 8)
    }
}

impl std::convert::From<i32> for FixedPoint32 {
    fn from(original: i32) -> Self {
        FixedPoint32::wrap(original << 8)
    }
}

impl std::convert::From<FixedPoint16> for FixedPoint32 {
    #[inline]
    fn from(original: FixedPoint16) -> Self {
        FixedPoint32::wrap(original.inner as i32)
    }
}

impl std::ops::Add for FixedPoint32 {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        FixedPoint32::wrap(self.inner.wrapping_add(other.inner))
    }
}

impl std::ops::AddAssign for FixedPoint32 {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        self.inner = self.inner.wrapping_add(other.inner);
    }
}

impl std::ops::Sub for FixedPoint32 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        FixedPoint32::wrap(self.inner.wrapping_sub(other.inner))
    }
}

impl std::ops::SubAssign for FixedPoint32 {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        self.inner = self.inner.wrapping_sub(other.inner);
    }
}

impl std::ops::Mul for FixedPoint32 {
    type Output = Self;
    #[inline]
    fn mul(self, other: Self) -> Self {
        FixedPoint32::wrap(self.inner.wrapping_mul(other.inner) >> 8)
    }
}

impl std::ops::MulAssign for FixedPoint32 {
    #[inline]
    fn mul_assign(&mut self, other: Self) {
        self.inner = self.inner.wrapping_mul(other.inner) >> 8;
    }
}

impl std::ops::Div for FixedPoint32 {
    type Output = Self;
    #[inline]
    fn div(self, other: Self) -> Self {
        FixedPoint32::wrap((self.inner << 16) / (other.inner << 8))
    }
}

impl std::ops::DivAssign for FixedPoint32 {
    #[inline]
    fn div_assign(&mut self, other: Self) {
        self.inner = (self.inner << 16) / (other.inner << 8);
    }
}

impl std::ops::Neg for FixedPoint32 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        FixedPoint32::wrap(-self.inner)
    }
}

impl std::fmt::Debug for FixedPoint32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}fp32", self.integer(), self.fractional())
    }
}
