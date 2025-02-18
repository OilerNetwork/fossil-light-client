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
    pub avg_fees: [(u64, u64); 4],
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

    // Parse avg_fees
    offset += 66;
    let mut i_0: u64 = 0;
    let mut j = 0;
    while j < 8 {
        let f0: u64 = (*journal_bytes.at(offset + j)).into();
        let f1: u64 = BitShift::shl(f0, 8 * j.into());
        i_0 += f1;
        j += 1;
    };
    offset += 8;
    let mut fee_0: u64 = 0;
    let mut j = 0;
    while j < 8 {
        let f0: u64 = (*journal_bytes.at(offset + j)).into();
        let f1: u64 = BitShift::shl(f0, 8 * j.into());
        fee_0 += f1;
        j += 1;
    };
    let avg_fees_0 = (i_0, fee_0);

    offset += 8;
    let mut i_1: u64 = 0;
    let mut j = 0;
    while j < 8 {
        let f0: u64 = (*journal_bytes.at(offset + j)).into();
        let f1: u64 = BitShift::shl(f0, 8 * j.into());
        i_1 += f1;
        j += 1;
    };
    offset += 8;
    let mut fee_1: u64 = 0;
    let mut j = 0;
    while j < 8 {
        let f0: u64 = (*journal_bytes.at(offset + j)).into();
        let f1: u64 = BitShift::shl(f0, 8 * j.into());
        fee_1 += f1;
        j += 1;
    };
    let avg_fees_1 = (i_1, fee_1);

    offset += 8;
    let mut i_2: u64 = 0;
    let mut j = 0;
    while j < 8 {
        let f0: u64 = (*journal_bytes.at(offset + j)).into();
        let f1: u64 = BitShift::shl(f0, 8 * j.into());
        i_2 += f1;
        j += 1;
    };
    offset += 8;
    let mut fee_2: u64 = 0;
    let mut j = 0;
    while j < 8 {
        let f0: u64 = (*journal_bytes.at(offset + j)).into();
        let f1: u64 = BitShift::shl(f0, 8 * j.into());
        fee_2 += f1;
        j += 1;
    };
    let avg_fees_2 = (i_2, fee_2);

    offset += 8;
    let mut i_3: u64 = 0;
    let mut j = 0;
    while j < 8 {
        let f0: u64 = (*journal_bytes.at(offset + j)).into();
        let f1: u64 = BitShift::shl(f0, 8 * j.into());
        i_3 += f1;
        j += 1;
    };
    offset += 8;
    let mut fee_3: u64 = 0;
    let mut j = 0;
    while j < 8 {
        let f0: u64 = (*journal_bytes.at(offset + j)).into();
        let f1: u64 = BitShift::shl(f0, 8 * j.into());
        fee_3 += f1;
        j += 1;
    };
    let avg_fees_3 = (i_3, fee_3);

    Journal {
        batch_index,
        latest_mmr_block,
        latest_mmr_block_hash,
        root_hash,
        leaves_count,
        first_block_parent_hash,
        avg_fees: [avg_fees_0, avg_fees_1, avg_fees_2, avg_fees_3],
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
        assert_eq!(journal.batch_index, 21356);
        assert_eq!(journal.latest_mmr_block, 21869567);
        assert_eq!(
            journal.latest_mmr_block_hash,
            0xef3bf25494173c997a6f53b065a90d186fb7b05058b71728c55785a61b2283a3,
        );
        assert_eq!(
            journal.root_hash, 0x12060ac56fb36e69ba68f0914aab0b7cf1c97e7d667c4010411b5d93a1a81336,
        );
        assert_eq!(journal.leaves_count, 1024);
        let [fees_0, fees_1, fees_2, fees_3] = journal.avg_fees;
        assert_eq!(fees_0, (85425, 1311344000));
        assert_eq!(fees_1, (85426, 1240653577));
        assert_eq!(fees_2, (85427, 902624002));
        assert_eq!(fees_3, (85428, 846720077));
    }

    fn get_journal_bytes() -> Span<u8> {
        array![
            108,
            83,
            0,
            0,
            0,
            0,
            0,
            0,
            255,
            179,
            77,
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
            101,
            102,
            51,
            98,
            102,
            50,
            53,
            52,
            57,
            52,
            49,
            55,
            51,
            99,
            57,
            57,
            55,
            97,
            54,
            102,
            53,
            51,
            98,
            48,
            54,
            53,
            97,
            57,
            48,
            100,
            49,
            56,
            54,
            102,
            98,
            55,
            98,
            48,
            53,
            48,
            53,
            56,
            98,
            55,
            49,
            55,
            50,
            56,
            99,
            53,
            53,
            55,
            56,
            53,
            97,
            54,
            49,
            98,
            50,
            50,
            56,
            51,
            97,
            51,
            0,
            0,
            66,
            0,
            0,
            0,
            48,
            120,
            49,
            50,
            48,
            54,
            48,
            97,
            99,
            53,
            54,
            102,
            98,
            51,
            54,
            101,
            54,
            57,
            98,
            97,
            54,
            56,
            102,
            48,
            57,
            49,
            52,
            97,
            97,
            98,
            48,
            98,
            55,
            99,
            102,
            49,
            99,
            57,
            55,
            101,
            55,
            100,
            54,
            54,
            55,
            99,
            52,
            48,
            49,
            48,
            52,
            49,
            49,
            98,
            53,
            100,
            57,
            51,
            97,
            49,
            97,
            56,
            49,
            51,
            51,
            54,
            0,
            0,
            0,
            4,
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
            52,
            53,
            54,
            102,
            97,
            54,
            49,
            49,
            51,
            53,
            100,
            54,
            48,
            53,
            56,
            51,
            98,
            102,
            55,
            51,
            100,
            55,
            53,
            50,
            57,
            101,
            98,
            50,
            101,
            51,
            49,
            99,
            56,
            53,
            101,
            49,
            57,
            99,
            50,
            99,
            55,
            101,
            101,
            101,
            57,
            100,
            56,
            48,
            48,
            52,
            51,
            98,
            48,
            55,
            57,
            97,
            97,
            101,
            51,
            97,
            100,
            48,
            101,
            57,
            0,
            0,
            177,
            77,
            1,
            0,
            0,
            0,
            0,
            0,
            128,
            133,
            41,
            78,
            0,
            0,
            0,
            0,
            178,
            77,
            1,
            0,
            0,
            0,
            0,
            0,
            9,
            223,
            242,
            73,
            0,
            0,
            0,
            0,
            179,
            77,
            1,
            0,
            0,
            0,
            0,
            0,
            2,
            243,
            204,
            53,
            0,
            0,
            0,
            0,
            180,
            77,
            1,
            0,
            0,
            0,
            0,
            0,
            77,
            236,
            119,
            50,
            0,
            0,
            0,
            0,
        ]
            .span()
    }
}
