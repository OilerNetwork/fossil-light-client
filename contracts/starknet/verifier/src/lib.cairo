pub mod fossil_verifier;
pub mod groth16_verifier;
mod groth16_verifier_constants;
pub mod universal_ecip;
use core::num::traits::{WideMul, Bounded};

pub(crate) fn decode_journal(journal_bytes: Span<u8>) -> (u256, u64, u64, u64) {
    let mut root_hash_start = 4;

    let mut i = root_hash_start + 2;
    let loop_end = root_hash_start + 66;
    let mut root_hash: u256 = 0;

    loop {
        if i >= loop_end {
            break;
        }

        let f0: u256 = BitShift::shl(root_hash, 4);
        let f1: u256 = (*journal_bytes.at(i)).into();
        let f2: u256 = if f1 < 58 {
            48
        } else {
            87
        };
        root_hash = f0 + f1 - f2;
        i += 1;
    };

    let leaves_count_offset = root_hash_start + 68;
    let mut leaves_count: u64 = 0;
    let mut j = 0;
    while j < 8 {
        let f0: u64 = (*journal_bytes.at(leaves_count_offset + j)).into();
        let f1: u64 = BitShift::shl(f0, 8 * j.into());
        leaves_count += f1;
        j += 1;
    };

    let batch_index_offset = leaves_count_offset + 8;
    let mut batch_index: u64 = 0;
    let mut k = 0;
    while k < 8 {
        let f0: u128 = (*journal_bytes.at(batch_index_offset + k)).into();
        let f1: u128 = BitShift::shl(f0.into(), 8 * k.into());
        batch_index += f1.try_into().unwrap();
        k += 1;
    };

    let latest_mmr_block_offset = batch_index_offset + 8;
    let mut latest_mmr_block: u64 = 0;
    let mut l = 0;
    while l < 8 {
        let f0: u128 = (*journal_bytes.at(latest_mmr_block_offset + l)).into();
        let f1: u128 = BitShift::shl(f0.into(), 8 * l.into());
        latest_mmr_block += f1.try_into().unwrap();
        l += 1;
    };

    (root_hash, leaves_count, batch_index, latest_mmr_block)
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
    base: T, exp: T
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

        let (root_hash, leaves_count, batch_index, latest_mmr_block) = decode_journal(
            journal_bytes
        );
        assert_eq!(
            root_hash, 29702900245875837452673332994207853201723234670058705475440356238131929011662
        );
        assert_eq!(leaves_count, 437);
        assert_eq!(batch_index, 20819);
        assert_eq!(latest_mmr_block, 21319092);
    }

    fn get_journal_bytes() -> Span<u8> {
        array![
            66,
            0,
            0,
            0,
            48,
            120,
            52,
            49,
            97,
            98,
            51,
            101,
            101,
            97,
            100,
            97,
            52,
            50,
            54,
            55,
            101,
            52,
            100,
            48,
            51,
            50,
            53,
            49,
            101,
            98,
            54,
            48,
            99,
            100,
            102,
            51,
            99,
            50,
            49,
            100,
            51,
            54,
            50,
            53,
            56,
            55,
            101,
            53,
            97,
            100,
            99,
            98,
            49,
            102,
            54,
            100,
            56,
            100,
            57,
            50,
            98,
            57,
            99,
            52,
            100,
            51,
            100,
            53,
            99,
            101,
            0,
            0,
            181,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            83,
            81,
            0,
            0,
            0,
            0,
            0,
            0,
            180,
            77,
            69,
            1,
            0,
            0,
            0,
            0
        ]
            .span()
    }
}
