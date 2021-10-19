use crate::state::ValidatorMetrics;
use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::{Decimal, StdError, StdResult, Uint128};


pub fn decimal_summation_in_256(a: Decimal, b: Decimal) -> Decimal {
    let a_u256: Decimal256 = a.into();
    let b_u256: Decimal256 = b.into();
    let c_u256: Decimal = (b_u256 + a_u256).into();
    c_u256
}

pub fn decimal_subtraction_in_256(a: Decimal, b: Decimal) -> Decimal {
    let a_u256: Decimal256 = a.into();
    let b_u256: Decimal256 = b.into();
    let c_u256: Decimal = (a_u256 - b_u256).into();
    c_u256
}

pub fn decimal_multiplication_in_256(a: Decimal, b: Decimal) -> Decimal {
    let a_u256: Decimal256 = a.into();
    let b_u256: Decimal256 = b.into();
    let c_u256: Decimal = (b_u256 * a_u256).into();
    c_u256
}

pub fn decimal_division_in_256(a: Decimal, b: Decimal) -> Decimal {
    let a_u256: Decimal256 = a.into();
    let b_u256: Decimal256 = b.into();
    let c_u256: Decimal = (a_u256 / b_u256).into();
    c_u256
}

pub fn uint128_to_decimal(num: Uint128) -> Decimal {
    let numerator: u128 = num.into();
    Decimal::from_ratio(numerator, 1_u128)
}

pub fn u64_to_decimal(num: u64) -> Decimal {
    let numerator: u128 = num.into();
    Decimal::from_ratio(numerator, 1_u128)
}

pub fn compute_apr(
    h1: &ValidatorMetrics,
    h2: &ValidatorMetrics,
    time_diff_in_seconds: u64,
) -> StdResult<Decimal> {
    if h1.delegated_amount.is_zero() {
        return Err(StdError::GenericErr {
            msg: "ZeroDivisionError: Cannot compute apr as the delegation amount is zero"
                .to_string(),
        });
    }

    if time_diff_in_seconds == 0 {
        return Err(StdError::GenericErr {
            msg: "ZeroDivisionError: Cannot compute apr as the time difference is zero".to_string(),
        });
    }

    let numerator = decimal_multiplication_in_256(
        decimal_subtraction_in_256(h2.rewards, h1.rewards),
        u64_to_decimal(3153600000), // (365 * 86400) * 100 => (365 * 86400) = Seconds in an year, 100 = percentage
    );

    let denominator = decimal_multiplication_in_256(
        uint128_to_decimal(h1.delegated_amount),
        u64_to_decimal(time_diff_in_seconds),
    );

    Ok(decimal_division_in_256(numerator, denominator))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Addr;

    use super::*;

    #[test]
    fn test_compute_apr() {
        let h1 = ValidatorMetrics {
            addr: Addr::unchecked("a1"),
            rewards: Decimal::one(),
            delegated_amount: Uint128::new(10),
            self_delegated_amount: Uint128::new(5),
            commission: Decimal::one(),
            max_commission: Decimal::one(),
            timestamp: 1,
            rewards_in_coins: vec![],
        };
        let h2 = ValidatorMetrics {
            addr: Addr::unchecked("a1"),
            rewards: u64_to_decimal(2),
            delegated_amount: Uint128::new(100),
            self_delegated_amount: Uint128::new(10),
            commission: Decimal::one(),
            max_commission: Decimal::one(),
            timestamp: 2,
            rewards_in_coins: vec![],
        };
        assert_eq!(compute_apr(&h1, &h2, 1), Ok(u64_to_decimal(315360000)))
    }
}
