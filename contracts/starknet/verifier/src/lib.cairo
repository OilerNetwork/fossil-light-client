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
}

pub(crate) fn decode_journal(journal_bytes: Span<u8>) -> Journal {
    let mut offset = 4; // Skip initial bytes

    // Parse batch_index
    let mut batch_index: u64 = 0;
    let mut i = 0;
    while i < 8 {
        let f0: u128 = (*journal_bytes.at(offset + i)).into();
        let f1: u128 = BitShift::shl(f0.into(), 8 * i.into());
        batch_index += f1.try_into().unwrap();
        i += 1;
    };

    // Parse latest_mmr_block (last_block_number)
    offset += 8;
    let mut latest_mmr_block: u64 = 0;
    let mut j = 0;
    while j < 8 {
        let f0: u128 = (*journal_bytes.at(offset + j)).into();
        let f1: u128 = BitShift::shl(f0.into(), 8 * j.into());
        latest_mmr_block += f1.try_into().unwrap();
        j += 1;
    };

    // Parse latest_mmr_block_hash
    offset += 8;
    let mut latest_mmr_block_hash: u256 = 0;
    let mut k = 0;
    while k < 32 {
        let f0: u256 = (*journal_bytes.at(offset + k)).into();
        let f1: u256 = BitShift::shl(f0, 8 * k.into());
        latest_mmr_block_hash += f1;
        k += 1;
    };

    // Parse root_hash
    offset += 32;
    let mut root_hash: u256 = 0;
    let mut l = 0;
    while l < 32 {
        let f0: u256 = (*journal_bytes.at(offset + l)).into();
        let f1: u256 = BitShift::shl(f0, 8 * l.into());
        root_hash += f1;
        l += 1;
    };

    // Parse leaves_count
    offset += 32;
    let mut leaves_count: u64 = 0;
    let mut m = 0;
    while m < 8 {
        let f0: u64 = (*journal_bytes.at(offset + m)).into();
        let f1: u64 = BitShift::shl(f0, 8 * m.into());
        leaves_count += f1;
        m += 1;
    };

    Journal { batch_index, latest_mmr_block, latest_mmr_block_hash, root_hash, leaves_count, }
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
        assert_eq!(
            journal.root_hash,
            63221064195583864302708890759072807439606022947695579972969583236139473588457,
        );
        assert_eq!(journal.leaves_count, 170);
        assert_eq!(journal.batch_index, 20820);
        assert_eq!(journal.latest_mmr_block, 21319849);
    }

    fn get_journal_bytes() -> Span<u8> {
        array![
            66,
            0,
            0,
            0,
            48,
            120,
            56,
            98,
            99,
            53,
            100,
            97,
            98,
            49,
            97,
            99,
            51,
            50,
            49,
            102,
            100,
            100,
            50,
            97,
            56,
            55,
            55,
            54,
            53,
            97,
            50,
            100,
            102,
            55,
            55,
            98,
            98,
            49,
            101,
            52,
            57,
            98,
            51,
            51,
            51,
            101,
            56,
            54,
            101,
            54,
            50,
            100,
            49,
            54,
            55,
            49,
            48,
            54,
            57,
            52,
            48,
            102,
            53,
            100,
            56,
            50,
            53,
            48,
            101,
            57,
            0,
            0,
            170,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            84,
            81,
            0,
            0,
            0,
            0,
            0,
            0,
            169,
            80,
            69,
            1,
            0,
            0,
            0,
            0,
        ]
            .span()
    }
}
