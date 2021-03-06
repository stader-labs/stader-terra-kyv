use cosmwasm_std::Addr;

pub(crate) fn u64_from_vec_u8(vector: Vec<u8>) -> u64 {
    let byte_0 = *vector.get(0).unwrap_or(&0);
    let byte_1 = *vector.get(1).unwrap_or(&0);
    let byte_2 = *vector.get(2).unwrap_or(&0);
    let byte_3 = *vector.get(3).unwrap_or(&0);
    let byte_4 = *vector.get(4).unwrap_or(&0);
    let byte_5 = *vector.get(5).unwrap_or(&0);
    let byte_6 = *vector.get(6).unwrap_or(&0);
    let byte_7 = *vector.get(7).unwrap_or(&0);

    (byte_7 as u64)
        + ((byte_6 as u64) * (256_u64))
        + ((byte_5 as u64) * (65536_u64))
        + ((byte_4 as u64) * (16777216_u64))
        + ((byte_3 as u64) * (4294967296_u64))
        + ((byte_2 as u64) * (1099511627776_u64))
        + ((byte_1 as u64) * (281474976710656_u64))
        + ((byte_0 as u64) * (72057594037927940_u64))
}
pub(crate) fn addr_from_vec_u8(vector: Vec<u8>) -> Addr {
    let addr_string = vector
        .into_iter()
        .map(|value| value as char)
        .collect::<String>();

    Addr::unchecked(addr_string)
}

#[cfg(test)]
mod tests {
    use crate::conversion_utils::{addr_from_vec_u8, u64_from_vec_u8};
    use cosmwasm_std::Addr;

    #[test]
    pub fn simple_test_0_0() {
        let actual_result = u64_from_vec_u8(vec![0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(0, actual_result);
    }
    #[test]
    pub fn simple_test_10_10() {
        let actual_result = u64_from_vec_u8(vec![0, 0, 0, 0, 0, 0, 0, 10]);
        assert_eq!(10, actual_result);
    }
    #[test]
    pub fn simple_test_1234_67305985() {
        let actual_result = u64_from_vec_u8(vec![0, 0, 0, 0, 4, 3, 2, 1]);
        assert_eq!(67305985, actual_result);
    }

    #[test]
    pub fn simple_test_0_0_0_0_0_0_1_134_160_to_100000() {
        let actual_result = u64_from_vec_u8(vec![0, 0, 0, 0, 0, 1, 134, 160]);
        let expected: u64 = 100000;

        assert_eq!(actual_result, expected);
    }

    #[test]
    pub fn address_test() {
        let input = vec![b't', b'e', b's', b't'];
        let expected = Addr::unchecked("test");
        let actual = addr_from_vec_u8(input);

        assert_eq!(actual, expected);
    }
}
