pub mod fossil_verifier;
pub mod groth16_verifier;
mod groth16_verifier_constants;
pub mod universal_ecip;
use core::num::traits::{WideMul, Bounded};

pub(crate) fn decode_journal(journal_bytes: Span<u8>) -> (u256, u64) {
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
    assert!(leaves_count_offset + 8 <= journal_bytes.len(), "Invalid journal length");

    let mut leaves_count: u64 = 0;
    let mut j = 0;
    while j < 8 {
        let f0: u64 = (*journal_bytes.at(leaves_count_offset + j)).into();
        let f1: u64 = BitShift::shl(f0, 8 * j.into());
        leaves_count += f1;
        j += 1;
    };

    (root_hash, leaves_count)
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

        let (root_hash, leaves_count) = decode_journal(journal_bytes);
        assert_eq!(
            root_hash,
            107280012852884767793665731955398724025869444191778930550273500320771511566933
        );
        assert_eq!(leaves_count, 305);
    }

    fn get_journal_bytes() -> Span<u8> {
        array![
            66,
            0,
            0,
            0,
            48,
            120,
            101,
            100,
            50,
            101,
            53,
            53,
            101,
            51,
            51,
            50,
            56,
            99,
            57,
            98,
            98,
            48,
            101,
            99,
            101,
            57,
            51,
            97,
            48,
            97,
            49,
            48,
            57,
            100,
            49,
            56,
            101,
            53,
            99,
            101,
            100,
            102,
            51,
            48,
            55,
            53,
            57,
            100,
            102,
            48,
            99,
            50,
            102,
            102,
            49,
            52,
            51,
            97,
            51,
            55,
            49,
            50,
            99,
            51,
            49,
            54,
            52,
            50,
            53,
            53,
            0,
            0,
            49,
            1,
            0,
            0,
            0,
            0,
            0,
            0
        ]
            .span()
    }
}
