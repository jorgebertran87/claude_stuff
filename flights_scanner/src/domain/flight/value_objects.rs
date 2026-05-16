use crate::domain::error::DomainError;

// ── Price ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct Price {
    amount: f64,
    currency: String,
}

impl Price {
    pub fn new(amount: f64, currency: &str) -> Result<Self, DomainError> {
        if amount <= 0.0 {
            return Err(DomainError::InvalidPrice);
        }
        Ok(Self {
            amount,
            currency: currency.to_uppercase(),
        })
    }

    pub fn amount(&self) -> f64 {
        self.amount
    }

    pub fn currency(&self) -> &str {
        &self.currency
    }
}

impl PartialOrd for Price {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.amount.partial_cmp(&other.amount)
    }
}

// ── Duration ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    minutes: u32,
}

impl Duration {
    pub fn from_minutes(minutes: u32) -> Result<Self, DomainError> {
        if minutes == 0 {
            return Err(DomainError::InvalidDuration);
        }
        Ok(Self { minutes })
    }

    pub fn minutes(&self) -> u32 {
        self.minutes
    }
}

impl std::ops::Add for Duration {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self { minutes: self.minutes + rhs.minutes }
    }
}

// ── CabinClass ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CabinClass {
    Economy,
    Business,
    First,
}

// ── PassengerCount ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassengerCount {
    pub adults: u8,
    pub children: u8,
    pub infants: u8,
}

impl PassengerCount {
    pub fn new(adults: u8, children: u8, infants: u8) -> Result<Self, DomainError> {
        if adults == 0 {
            return Err(DomainError::InvalidPassengerCount(
                "at least one adult required".into(),
            ));
        }
        if infants > adults {
            return Err(DomainError::InvalidPassengerCount(
                "infants cannot exceed adults".into(),
            ));
        }
        Ok(Self { adults, children, infants })
    }

    pub fn total(&self) -> u8 {
        self.adults + self.children + self.infants
    }
}

// ── FlightNumber ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlightNumber(String);

impl FlightNumber {
    /// Format: 2-letter IATA airline code + 1-4 digit number, e.g. "IB3456"
    pub fn new(value: &str) -> Result<Self, DomainError> {
        let value = value.trim();
        let (prefix, digits) = value.split_at(value.len().min(2));
        let valid = prefix.len() == 2
            && prefix.chars().all(|c| c.is_ascii_uppercase())
            && !digits.is_empty()
            && digits.len() <= 4
            && digits.chars().all(|c| c.is_ascii_digit());
        if valid {
            Ok(Self(value.to_string()))
        } else {
            Err(DomainError::InvalidFlightNumber(value.to_string()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Price
    #[test]
    fn valid_price_is_accepted() {
        let p = Price::new(199.99, "EUR").unwrap();
        assert_eq!(p.amount(), 199.99);
        assert_eq!(p.currency(), "EUR");
    }

    #[test]
    fn zero_price_is_rejected() {
        assert_eq!(Price::new(0.0, "EUR"), Err(DomainError::InvalidPrice));
    }

    #[test]
    fn negative_price_is_rejected() {
        assert_eq!(Price::new(-1.0, "EUR"), Err(DomainError::InvalidPrice));
    }

    #[test]
    fn prices_are_comparable() {
        let cheap = Price::new(100.0, "EUR").unwrap();
        let expensive = Price::new(300.0, "EUR").unwrap();
        assert!(cheap < expensive);
    }

    // Duration
    #[test]
    fn valid_duration_is_accepted() {
        let d = Duration::from_minutes(90).unwrap();
        assert_eq!(d.minutes(), 90);
    }

    #[test]
    fn zero_duration_is_rejected() {
        assert_eq!(Duration::from_minutes(0), Err(DomainError::InvalidDuration));
    }

    #[test]
    fn durations_add_correctly() {
        let a = Duration::from_minutes(60).unwrap();
        let b = Duration::from_minutes(90).unwrap();
        assert_eq!((a + b).minutes(), 150);
    }

    // PassengerCount
    #[test]
    fn valid_passenger_count_accepted() {
        let pc = PassengerCount::new(2, 1, 1).unwrap();
        assert_eq!(pc.total(), 4);
    }

    #[test]
    fn zero_adults_is_rejected() {
        assert!(PassengerCount::new(0, 0, 0).is_err());
    }

    #[test]
    fn infants_exceeding_adults_is_rejected() {
        assert!(PassengerCount::new(1, 0, 2).is_err());
    }

    // FlightNumber
    #[test]
    fn valid_flight_number_accepted() {
        let fn_ = FlightNumber::new("IB3456").unwrap();
        assert_eq!(fn_.as_str(), "IB3456");
    }

    #[test]
    fn flight_number_with_lowercase_prefix_rejected() {
        assert!(FlightNumber::new("ib34").is_err());
    }

    #[test]
    fn flight_number_with_too_many_digits_rejected() {
        assert!(FlightNumber::new("IB12345").is_err());
    }
}
