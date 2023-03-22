use crate::{B160, B176, B256, U256};
use hex_literal::hex;
use sha3::{Digest, Keccak256};
use std::str::FromStr;

const MAINNET: &str = "cb";
const TESTNET: &str = "ab";
const PRIVATE: &str = "ce";

pub enum NetworkType {
    Mainnet,
    Testnet,
    Private,
}

pub const KECCAK_EMPTY: B256 = B256(hex!(
    "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
));

#[inline(always)]
pub fn keccak256(input: &[u8]) -> B256 {
    B256::from_slice(Keccak256::digest(input).as_slice())
}

/// Returns the address for the legacy `CREATE` scheme: [`CreateScheme::Create`]
pub fn create_address(caller: B176, nonce: u64) -> B176 {
    let mut stream = rlp::RlpStream::new_list(2);
    stream.append(&caller.0.as_ref());
    stream.append(&nonce);
    let out = keccak256(&stream.out());

    // Get the last 20 bytes of the hash
    let addr = B160(out[12..].try_into().unwrap());

    // Calculate the checksum and add the network prefix
    to_ican(&addr, &NetworkType::Mainnet)
}

/// Returns the address for the `CREATE2` scheme: [`CreateScheme::Create2`]
pub fn create2_address(caller: B176, code_hash: B256, salt: U256) -> B176 {
    let mut hasher = Keccak256::new();
    hasher.update([0xff]);
    hasher.update(&caller[..]);
    hasher.update(salt.to_be_bytes::<{ U256::BYTES }>());
    hasher.update(&code_hash[..]);

    // Get the last 20 bytes of the hash
    let addr = B160(hasher.finalize().as_slice()[12..].try_into().unwrap());

    // Calculate the checksum and add the network prefix
    to_ican(&addr, &NetworkType::Mainnet)
}

fn to_ican(addr: &B160, network: &NetworkType) -> B176 {
    // Get the prefix str
    let prefix = match network {
        NetworkType::Mainnet => MAINNET,
        NetworkType::Testnet => TESTNET,
        NetworkType::Private => PRIVATE,
    };

    // Get the number string from the hex address
    let number_str = get_number_string(addr, network);

    // Calculate the checksum
    let checksum = calculate_checksum(&number_str);

    // Format it all together
    construct_ican_address(prefix, &checksum, addr)
}

fn get_number_string(addr: &B160, network: &NetworkType) -> String {
    let prefix = match network {
        NetworkType::Mainnet => MAINNET,
        NetworkType::Testnet => TESTNET,
        NetworkType::Private => PRIVATE,
    };

    // We have to use the Debug trait for addr https://github.com/paritytech/parity-common/issues/656
    let mut addr_str = format!("{:?}{}{}", addr, prefix, "00");
    // Remove the 0x prefix
    addr_str = addr_str.replace("0x", "");

    // Convert every hex digit to decimal and then to String
    addr_str
        .chars()
        .map(|x| x.to_digit(16).expect("Invalid Address").to_string())
        .collect::<String>()
}

fn calculate_checksum(number_str: &str) -> u64 {
    // number_str % 97
    let result = number_str.chars().fold(0, |acc, ch| {
        let digit = ch.to_digit(10).expect("Invalid Digit") as u64;
        (acc * 10 + digit) % 97
    });

    98 - result
}

fn construct_ican_address(prefix: &str, checksum: &u64, addr: &B160) -> B176 {
    // We need to use debug for the address https://github.com/paritytech/parity-common/issues/656
    let addr = format!("{:?}", addr);
    // Remove 0x prefix
    let addr = addr.replace("0x", "");

    // If the checksum is less than 10 we need to add a zero to the address
    if *checksum < 10 {
        B176::from_str(&format!("{prefix}{zero}{checksum}{addr}", zero = "0")).unwrap()
    } else {
        let formated = format!("{prefix}{checksum}{addr}");
        println!("{}", formated);
        B176::from_str(&format!("{prefix}{checksum}{addr}")).unwrap()
    }
}

/// Serde functions to serde as [bytes::Bytes] hex string
#[cfg(feature = "serde")]
pub mod serde_hex_bytes {
    use alloc::string::String;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S, T>(x: T, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: AsRef<[u8]>,
    {
        s.serialize_str(&alloc::format!("0x{}", hex::encode(x.as_ref())))
    }

    pub fn deserialize<'de, D>(d: D) -> Result<bytes::Bytes, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(d)?;
        if let Some(value) = value.strip_prefix("0x") {
            hex::decode(value)
        } else {
            hex::decode(&value)
        }
        .map(Into::into)
        .map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_create_one() {
        let caller = B176::from_str("cb72e8cF4629ACB360350399B6CFF367A97CF36E62B9").unwrap();
        let ican_address = create_address(caller, 1);

        assert_eq!(
            ican_address,
            B176::from_str("cb41485a42277ed7f4fea81cc12efd12d57dcb549150").unwrap()
        );
    }
    #[test]
    fn test_create_two() {
        let caller = B176::from_str("cb72e8cF4629ACB360350399B6CFF367A97CF36E62Ba").unwrap();
        let ican_address = create_address(caller, 1);

        assert_eq!(
            ican_address,
            B176::from_str("cb68b6dd5cdf0c69c82081746ec9856dad75075e72e4").unwrap()
        );
    }
    #[test]
    fn test_create_three() {
        let caller = B176::from_str("cb72e8cF4629ACB360350399B6CFF367A97CF36E62Bc").unwrap();
        let ican_address = create_address(caller, 1);

        assert_eq!(
            ican_address,
            B176::from_str("cb6959e403eb217b58e3b892dd0d07b560ec36a7f7f4").unwrap()
        );
    }

    #[test]
    fn test_create2_one() {
        let caller = B176::from_str("cb72e8cF4629ACB360350399B6CFF367A97CF36E62B9").unwrap();
        let ican_address = create2_address(caller, B256::repeat_byte(10), U256::from(239048));

        assert_eq!(
            ican_address,
            B176::from_str("cb1530dffdf96017ce586326f231beb4fbbdfb117447").unwrap()
        );
    }
    #[test]
    fn test_create2_two() {
        let caller = B176::from_str("cb72e8cF4629ACB360350399B6CFF367A97CF36E62Ba").unwrap();
        let ican_address = create2_address(caller, B256::repeat_byte(11), U256::from(239048));

        assert_eq!(
            ican_address,
            B176::from_str("cb43708ccdbe03ea3773582f4af6aab1982a8e9482d6").unwrap()
        );
    }
    #[test]
    fn test_create2_three() {
        let caller = B176::from_str("cb72e8cF4629ACB360350399B6CFF367A97CF36E62Bb").unwrap();
        let ican_address = create2_address(caller, B256::repeat_byte(12), U256::from(239048));

        assert_eq!(
            ican_address,
            B176::from_str("cb33779fc3d7bb2c9e6ded3c42e865b28c5d8a70c8b7").unwrap()
        );
    }

    // Done
    #[test]
    fn test_get_number_string_address() {
        let address = B160::from_str("e8cF4629ACB360350399B6CFF367A97CF36E62B9").unwrap();
        let number_str = get_number_string(&address, &NetworkType::Mainnet);
        assert_eq!(
            number_str,
            String::from("1481215462910121136035039911612151536710971215361462119121100")
        );
    }

    #[test]
    fn test_calculate_checksum_address() {
        let address = B160::from_str("e8cF4629ACB360350399B6CFF367A97CF36E62B9").unwrap();
        let number_str = get_number_string(&address, &NetworkType::Mainnet);
        let checksum = calculate_checksum(&number_str);
        assert_eq!(checksum, 72u64);
    }
}
