use std::fmt;
use std::ops::{Add, Mul, Sub};

/// Monetary value. Wraps f64; all values assumed finite and non-negative.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Money(pub f64);

impl Money {
    pub fn zero() -> Self {
        Money(0.0)
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${:.2}", self.0)
    }
}

impl Add for Money {
    type Output = Money;
    fn add(self, rhs: Money) -> Money {
        Money(self.0 + rhs.0)
    }
}

impl Sub for Money {
    type Output = Money;
    fn sub(self, rhs: Money) -> Money {
        Money(self.0 - rhs.0)
    }
}

impl Mul<f64> for Money {
    type Output = Money;
    fn mul(self, rhs: f64) -> Money {
        Money(self.0 * rhs)
    }
}

/// Opaque participant identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BidderId(pub u32);

impl fmt::Display for BidderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bidder#{}", self.0)
    }
}

/// Opaque item identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(pub u32);

/// Simulated time in seconds.
pub type SimTime = f64;

/// Which auction mechanism is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuctionType {
    English,
    Dutch,
    FirstPriceSealedBid,
    Vickrey,
    AllPay,
    Double,
    Combinatorial,
    Vcg,
}

/// Lifecycle phase of an auction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuctionPhase {
    NotStarted,
    Bidding,
    Resolving,
    Complete,
}
