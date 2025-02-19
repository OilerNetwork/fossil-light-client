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

#[derive(Drop, Serde)]
pub struct AvgFees {
    pub timestamp: u64,
    pub data_points: u64,
    pub avg_fee: u64,
}

pub(crate) fn decode_journal(journal_bytes: Span<u8>) -> (Journal, Array<AvgFees>) {
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

    let mut avg_fees_len: usize = 0;
    let mut i = 0;
    while i < 4 {
        let f0: u32 = (*journal_bytes.at(offset + i)).into();
        let f1: u32 = BitShift::shl(f0, 8 * i.into());
        avg_fees_len += f1;
        i += 1;
    };

    offset += 4;
    let mut avg_fees: Array<AvgFees> = array![];

    for _ in 0..avg_fees_len {
        let mut timestamp: u64 = 0;
        let mut data_points: u64 = 0;
        let mut avg_fee: u64 = 0;

        let mut j = 0;
        while j < 8 {
            let f0: u64 = (*journal_bytes.at(offset + j)).into();
            let f1: u64 = BitShift::shl(f0, 8 * j.into());
            timestamp += f1;
            j += 1;
        };

        offset += 8;
        let mut j = 0;
        while j < 8 {
            let f0: u64 = (*journal_bytes.at(offset + j)).into();
            let f1: u64 = BitShift::shl(f0, 8 * j.into());
            data_points += f1;
            j += 1;
        };

        offset += 8;
        let mut j = 0;
        while j < 8 {
            let f0: u64 = (*journal_bytes.at(offset + j)).into();
            let f1: u64 = BitShift::shl(f0, 8 * j.into());
            avg_fee += f1;
            j += 1;
        };

        offset += 8;

        avg_fees.append(AvgFees { timestamp, data_points, avg_fee });
    };

    (
        Journal {
            batch_index,
            latest_mmr_block,
            latest_mmr_block_hash,
            root_hash,
            leaves_count,
            first_block_parent_hash,
        },
        avg_fees,
    )
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

impl U32BitShift of BitShift<u32> {
    fn shl(x: u32, n: u32) -> u32 {
        (WideMul::wide_mul(x, pow(2, n)) & Bounded::<u32>::MAX.into()).try_into().unwrap()
    }

    fn shr(x: u32, n: u32) -> u32 {
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

        let (journal, avg_fees) = decode_journal(journal_bytes);
        assert_eq!(journal.batch_index, 21369);
        assert_eq!(journal.latest_mmr_block, 21882622);
        assert_eq!(
            journal.latest_mmr_block_hash,
            0x930046a42d2e9ae094e48890903f998d6edf12265aad7f5f620be4507e961d48,
        );
        assert_eq!(
            journal.root_hash, 0x930aa189e5be10188debe34e6df8930d0bce5ee7c61ff26222e5a33e2ce421fb,
        );
        assert_eq!(journal.leaves_count, 767);
        assert_eq!(avg_fees.len(), 3);
        assert_eq!(*avg_fees[0].timestamp, 1739984400);
        assert_eq!(*avg_fees[0].data_points, 210);
        assert_eq!(*avg_fees[0].avg_fee, 1356994173);

        assert_eq!(*avg_fees[1].timestamp, 1739988000);
        assert_eq!(*avg_fees[1].data_points, 297);
        assert_eq!(*avg_fees[1].avg_fee, 957746452);

        assert_eq!(*avg_fees[2].timestamp, 1739991600);
        assert_eq!(*avg_fees[2].data_points, 260);
        assert_eq!(*avg_fees[2].avg_fee, 864421784);
    }

    fn get_journal_bytes() -> Span<u8> {
        array![
            121,
            83,
            0,
            0,
            0,
            0,
            0,
            0,
            254,
            230,
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
            57,
            51,
            48,
            48,
            52,
            54,
            97,
            52,
            50,
            100,
            50,
            101,
            57,
            97,
            101,
            48,
            57,
            52,
            101,
            52,
            56,
            56,
            57,
            48,
            57,
            48,
            51,
            102,
            57,
            57,
            56,
            100,
            54,
            101,
            100,
            102,
            49,
            50,
            50,
            54,
            53,
            97,
            97,
            100,
            55,
            102,
            53,
            102,
            54,
            50,
            48,
            98,
            101,
            52,
            53,
            48,
            55,
            101,
            57,
            54,
            49,
            100,
            52,
            56,
            0,
            0,
            66,
            0,
            0,
            0,
            48,
            120,
            57,
            51,
            48,
            97,
            97,
            49,
            56,
            57,
            101,
            53,
            98,
            101,
            49,
            48,
            49,
            56,
            56,
            100,
            101,
            98,
            101,
            51,
            52,
            101,
            54,
            100,
            102,
            56,
            57,
            51,
            48,
            100,
            48,
            98,
            99,
            101,
            53,
            101,
            101,
            55,
            99,
            54,
            49,
            102,
            102,
            50,
            54,
            50,
            50,
            50,
            101,
            53,
            97,
            51,
            51,
            101,
            50,
            99,
            101,
            52,
            50,
            49,
            102,
            98,
            0,
            0,
            255,
            2,
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
            49,
            100,
            51,
            52,
            97,
            52,
            51,
            50,
            51,
            102,
            54,
            55,
            97,
            50,
            55,
            57,
            53,
            98,
            56,
            51,
            52,
            102,
            55,
            100,
            54,
            52,
            53,
            98,
            48,
            57,
            102,
            100,
            100,
            102,
            51,
            55,
            97,
            50,
            99,
            99,
            49,
            49,
            100,
            52,
            100,
            98,
            51,
            50,
            56,
            52,
            102,
            52,
            56,
            55,
            100,
            102,
            52,
            53,
            102,
            102,
            53,
            54,
            101,
            101,
            0,
            0,
            3,
            0,
            0,
            0,
            16,
            14,
            182,
            103,
            0,
            0,
            0,
            0,
            210,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            125,
            22,
            226,
            80,
            0,
            0,
            0,
            0,
            32,
            28,
            182,
            103,
            0,
            0,
            0,
            0,
            41,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            20,
            13,
            22,
            57,
            0,
            0,
            0,
            0,
            48,
            42,
            182,
            103,
            0,
            0,
            0,
            0,
            4,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            152,
            7,
            134,
            51,
            0,
            0,
            0,
            0,
        ]
            .span()
    }
}
