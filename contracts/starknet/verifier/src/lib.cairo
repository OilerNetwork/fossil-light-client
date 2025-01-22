pub mod fossil_verifier;
pub mod groth16_verifier;
mod groth16_verifier_constants;
pub mod universal_ecip;
use core::num::traits::{Bounded, WideMul};

#[derive(Drop, Copy, Serde)]
pub struct Journal {
    pub batch_index: u64,
    pub latest_mmr_block: u64,
    pub latest_mmr_block_hash: u256,
    pub root_hash: u256,
    pub leaves_count: u64,
    pub first_block_parent_hash: u256,
}

pub(crate) fn decode_journal(journal_bytes: Span<u8>) -> Journal {
    let mut offset = 0; // Skip initial bytes

    // Parse batch_index
    let mut batch_index: u64 = 0;
    let mut i = 0;
    while i < 8 {
        let f0: u64 = (*journal_bytes.at(offset + i)).into();
        let f1: u64 = BitShift::shl(f0, 8 * i.into());
        batch_index += f1;
        i += 1;
    };

    // Parse latest_mmr_block
    offset += 8;
    let mut latest_mmr_block: u64 = 0;
    let mut j = 0;
    while j < 8 {
        let f0: u64 = (*journal_bytes.at(offset + j)).into();
        let f1: u64 = BitShift::shl(f0, 8 * j.into());
        latest_mmr_block += f1;
        j += 1;
    };
    // Parse latest_mmr_block_hash
    offset += 8; // Skip to start of hash length
    offset += 4; // Skip length indicator (66, 0, 0, 0)
    offset += 2; // Skip "0x" prefix
    let mut latest_mmr_block_hash: u256 = 0;
    let mut i = offset;
    let loop_end = offset + 64; // 64 hex characters for 32 bytes

    loop {
        if i >= loop_end {
            break;
        }

        let f0: u256 = BitShift::shl(latest_mmr_block_hash, 4);
        let f1: u256 = (*journal_bytes.at(i)).into();
        let f2: u256 = if f1 < 58 { // '0'-'9' vs 'a'-'f'
            48 // ASCII '0'
        } else {
            87 // ASCII 'a' - 10
        };
        latest_mmr_block_hash = f0 + f1 - f2;
        i += 1;
    };
    // Parse root_hash
    offset += 66; // Skip past latest_mmr_block_hash (64 hex chars + "0x")
    offset += 4; // Skip length indicator (66, 0, 0, 0)
    offset += 2; // Skip "0x" prefix
    let mut root_hash: u256 = 0;
    let mut i = offset;
    let loop_end = offset + 64; // 64 hex characters for 32 bytes

    loop {
        if i >= loop_end {
            break;
        }

        let f0: u256 = BitShift::shl(root_hash, 4);
        let f1: u256 = (*journal_bytes.at(i)).into();
        let f2: u256 = if f1 < 58 { // '0'-'9' vs 'a'-'f'
            48 // ASCII '0'
        } else {
            87 // ASCII 'a' - 10
        };
        root_hash = f0 + f1 - f2;
        i += 1;
    };
    // Parse leaves_count
    offset += 66;
    let mut leaves_count: u64 = 0;
    let mut m = 0;
    while m < 8 {
        let f0: u64 = (*journal_bytes.at(offset + m)).into();
        let f1: u64 = BitShift::shl(f0, 8 * m.into());
        leaves_count += f1;
        m += 1;
    };

    // Parse first_block_parent_hash
    offset += 8;
    offset += 4; // Skip length indicator (66, 0, 0, 0)
    offset += 2; // Skip "0x" prefix
    let mut first_block_parent_hash: u256 = 0;
    let mut i = offset;
    let loop_end = offset + 64;

    loop {
        if i >= loop_end {
            break;
        }

        let f0: u256 = BitShift::shl(first_block_parent_hash, 4);
        let f1: u256 = (*journal_bytes.at(i)).into();
        let f2: u256 = if f1 < 58 { // '0'-'9' vs 'a'-'f'
            48 // ASCII '0'
        } else {
            87 // ASCII 'a' - 10
        };
        first_block_parent_hash = f0 + f1 - f2;
        i += 1;
    };
   
    Journal {
        batch_index,
        latest_mmr_block,
        latest_mmr_block_hash,
        root_hash,
        leaves_count,
        first_block_parent_hash,
    }
}

trait BitShift<T> {
    fn shl(x: T, n: T) -> T;
    fn shr(x: T, n: T) -> T;
}

impl U256BitShift of BitShift<u256> {
    fn shl(x: u256, n: u256) -> u256 {
        let res = WideMul::wide_mul(x, pow(2, n));
        u256 { low: res.limb0, high: res.limb1 }
    }

    fn shr(x: u256, n: u256) -> u256 {
        x / pow(2, n)
    }
}

impl U64BitShift of BitShift<u64> {
    fn shl(x: u64, n: u64) -> u64 {
        (WideMul::wide_mul(x, pow(2, n)) & Bounded::<u64>::MAX.into()).try_into().unwrap()
    }

    fn shr(x: u64, n: u64) -> u64 {
        x / pow(2, n)
    }
}

impl U128BitShift of BitShift<u128> {
    fn shl(x: u128, n: u128) -> u128 {
        let res = WideMul::wide_mul(x, pow(2, n));
        res.low
    }

    fn shr(x: u128, n: u128) -> u128 {
        x / pow(2, n)
    }
}

fn pow<T, +Sub<T>, +Mul<T>, +Div<T>, +Rem<T>, +PartialEq<T>, +Into<u8, T>, +Drop<T>, +Copy<T>>(
    base: T, exp: T,
) -> T {
    if exp == 0_u8.into() {
        1_u8.into()
    } else if exp == 1_u8.into() {
        base
    } else if exp % 2_u8.into() == 0_u8.into() {
        pow(base * base, exp / 2_u8.into())
    } else {
        base * pow(base * base, exp / 2_u8.into())
    }
}

#[cfg(test)]
mod tests {
    use super::decode_journal;

    #[test]
    fn decode_journal_test() {
        let journal_bytes = get_journal_bytes();

        let journal = decode_journal(journal_bytes);
        assert_eq!(journal.batch_index, 7083);
        assert_eq!(journal.latest_mmr_block, 7253851);
        assert_eq!(
            journal.latest_mmr_block_hash,
            0x858768dd79b8c6190fb224ff398345ffe4fcb9c4899c55e0fc0994b7d35177af,
        );
        assert_eq!(
            journal.root_hash, 0x72aa9525dc9b7953631c0699d041fd4f23aa9f98c4a73aab27fbf2f0b9b451f8,
        );
        assert_eq!(journal.leaves_count, 4);
    }

    fn get_journal_bytes() -> Span<u8> {
        array![
            123,
            177,
            82,
            0,
            0,
            0,
            0,
            0,
            239,
            197,
            74,
            1,
            0,
            0,
            0,
            0,
            66,
            0,
            0,
            0,
            48,
            120,
            50,
            100,
            98,
            49,
            97,
            55,
            99,
            97,
            51,
            100,
            100,
            51,
            52,
            99,
            57,
            99,
            98,
            48,
            102,
            99,
            50,
            98,
            100,
            54,
            52,
            55,
            51,
            52,
            102,
            49,
            50,
            48,
            101,
            100,
            54,
            100,
            55,
            102,
            49,
            49,
            101,
            52,
            54,
            55,
            48,
            99,
            100,
            97,
            100,
            48,
            98,
            49,
            51,
            52,
            101,
            48,
            57,
            97,
            51,
            50,
            52,
            48,
            52,
            49,
            0,
            0,
            66,
            0,
            0,
            0,
            48,
            120,
            98,
            56,
            51,
            98,
            54,
            97,
            56,
            52,
            100,
            56,
            99,
            102,
            56,
            98,
            55,
            100,
            101,
            54,
            51,
            97,
            102,
            48,
            51,
            101,
            102,
            57,
            51,
            56,
            101,
            97,
            97,
            101,
            100,
            54,
            54,
            52,
            56,
            98,
            101,
            53,
            56,
            55,
            56,
            98,
            54,
            53,
            100,
            49,
            52,
            57,
            98,
            52,
            49,
            98,
            53,
            57,
            49,
            98,
            51,
            53,
            55,
            50,
            50,
            100,
            0,
            0,
            4,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            66,
            0,
            0,
            0,
            48,
            120,
            57,
            48,
            100,
            50,
            49,
            51,
            48,
            53,
            53,
            54,
            55,
            57,
            57,
            97,
            55,
            56,
            51,
            101,
            50,
            97,
            56,
            102,
            97,
            102,
            55,
            98,
            50,
            51,
            52,
            54,
            56,
            53,
            54,
            50,
            52,
            101,
            55,
            49,
            51,
            49,
            52,
            101,
            48,
            100,
            98,
            51,
            56,
            57,
            99,
            101,
            56,
            54,
            49,
            53,
            100,
            98,
            97,
            102,
            53,
            100,
            52,
            50,
            53,
            48,
            0,
            0,
        ]
            .span()
    }
}
