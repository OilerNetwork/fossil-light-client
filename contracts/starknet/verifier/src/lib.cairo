pub mod fossil_verifier;
pub mod groth16_verifier;
mod groth16_verifier_constants;
pub mod universal_ecip;
use core::num::traits::{Bounded, WideMul};

#[derive(Drop, Debug, Copy, PartialEq, Serde)]
pub struct Journal {
    pub batch_index: u64,
    pub latest_mmr_block: u64,
    pub latest_mmr_block_hash: u256,
    pub root_hash: u256,
    pub leaves_count: u64,
    pub first_block_parent_hash: u256,
}

#[derive(Drop, Debug, PartialEq, Serde)]
pub struct AvgFees {
    pub timestamp: u64,
    pub data_points: u64,
    pub avg_fee: felt252,
}

pub fn decode_journal(journal_bytes: Span<u8>) -> (Journal, Array<AvgFees>) {
    // Constants for byte sizes and offsets
    const U64_SIZE: usize = 8;
    const U32_SIZE: usize = 4;
    const HEX_PREFIX_SIZE: usize = 2; // "0x"
    const HEX_HASH_SIZE: usize = 64; // 32 bytes as hex
    const HEX_HASH_WITH_PREFIX_SIZE: usize = 66; // "0x" + 64 hex chars
    const ASCII_0: u256 = 48;
    const ASCII_A_OFFSET: u256 = 87; // 'a' - 10 = 97 - 10 = 87

    let mut byte_offset = 0; // Current position in the byte array

    // Parse batch_index
    let mut batch_index: u64 = 0;
    let mut byte_idx = 0;

    while byte_idx < U64_SIZE {
        let current_byte: u64 = (*journal_bytes.at(byte_offset + byte_idx)).into();
        let shifted_byte: u64 = BitShift::shl(current_byte, 8 * byte_idx.into());
        batch_index += shifted_byte;
        byte_idx += 1;
    }
    byte_offset += U64_SIZE;

    // Parse latest_mmr_block
    let mut latest_mmr_block: u64 = 0;
    let mut byte_idx = 0;
    while byte_idx < U64_SIZE {
        let current_byte: u64 = (*journal_bytes.at(byte_offset + byte_idx)).into();
        let shifted_byte: u64 = BitShift::shl(current_byte, 8 * byte_idx.into());
        latest_mmr_block += shifted_byte;
        byte_idx += 1;
    }
    // Parse latest_mmr_block_hash
    byte_offset += U64_SIZE; // Skip to start of hash length
    byte_offset += U32_SIZE; // Skip length indicator (66, 0, 0, 0)
    byte_offset += HEX_PREFIX_SIZE; // Skip "0x" prefix
    let mut latest_mmr_block_hash: u256 = 0;
    let mut hex_idx = byte_offset;
    let hex_end = byte_offset + HEX_HASH_SIZE; // 64 hex characters for 32 bytes

    loop {
        if hex_idx >= hex_end {
            break;
        }

        let shifted_hash: u256 = BitShift::shl(latest_mmr_block_hash, 4);
        let hex_byte: u256 = (*journal_bytes.at(hex_idx)).into();
        let hex_base: u256 = if hex_byte < 58 { // '0'-'9' vs 'a'-'f'
            ASCII_0 // ASCII '0'
        } else {
            ASCII_A_OFFSET // ASCII 'a' - 10
        };
        latest_mmr_block_hash = shifted_hash + hex_byte - hex_base;
        hex_idx += 1;
    }
    // Parse root_hash
    byte_offset +=
        HEX_HASH_WITH_PREFIX_SIZE; // Skip past latest_mmr_block_hash (64 hex chars + "0x")
    byte_offset += U32_SIZE; // Skip length indicator (66, 0, 0, 0)
    byte_offset += HEX_PREFIX_SIZE; // Skip "0x" prefix
    let mut root_hash: u256 = 0;
    let mut hex_idx = byte_offset;
    let hex_end = byte_offset + HEX_HASH_SIZE; // 64 hex characters for 32 bytes

    loop {
        if hex_idx >= hex_end {
            break;
        }

        let shifted_hash: u256 = BitShift::shl(root_hash, 4);
        let hex_byte: u256 = (*journal_bytes.at(hex_idx)).into();
        let hex_base: u256 = if hex_byte < 58 { // '0'-'9' vs 'a'-'f'
            ASCII_0 // ASCII '0'
        } else {
            ASCII_A_OFFSET // ASCII 'a' - 10
        };
        root_hash = shifted_hash + hex_byte - hex_base;
        hex_idx += 1;
    }
    // Parse leaves_count
    byte_offset += HEX_HASH_WITH_PREFIX_SIZE;
    let mut leaves_count: u64 = 0;
    let mut byte_idx = 0;
    while byte_idx < U64_SIZE {
        let current_byte: u64 = (*journal_bytes.at(byte_offset + byte_idx)).into();
        let shifted_byte: u64 = BitShift::shl(current_byte, 8 * byte_idx.into());
        leaves_count += shifted_byte;
        byte_idx += 1;
    }
    // Parse first_block_parent_hash
    byte_offset += U64_SIZE;
    byte_offset += U32_SIZE; // Skip length indicator (66, 0, 0, 0)
    byte_offset += HEX_PREFIX_SIZE; // Skip "0x" prefix
    let mut first_block_parent_hash: u256 = 0;
    let mut hex_idx = byte_offset;
    let hex_end = byte_offset + HEX_HASH_SIZE;

    loop {
        if hex_idx >= hex_end {
            break;
        }

        let shifted_hash: u256 = BitShift::shl(first_block_parent_hash, 4);
        let hex_byte: u256 = (*journal_bytes.at(hex_idx)).into();
        let hex_base: u256 = if hex_byte < 58 { // '0'-'9' vs 'a'-'f'
            ASCII_0 // ASCII '0'
        } else {
            ASCII_A_OFFSET // ASCII 'a' - 10
        };
        first_block_parent_hash = shifted_hash + hex_byte - hex_base;
        hex_idx += 1;
    }
    // Parse avg_fees
    byte_offset += HEX_HASH_WITH_PREFIX_SIZE;

    // Read the number of fee entries
    let mut avg_fees_len: usize = 0;
    let mut byte_idx = 0;
    while byte_idx < U32_SIZE {
        let current_byte: u32 = (*journal_bytes.at(byte_offset + byte_idx)).into();
        let shifted_byte: u32 = BitShift::shl(current_byte, 8 * byte_idx.into());
        avg_fees_len += shifted_byte;
        byte_idx += 1;
    }

    byte_offset += U32_SIZE;
    // Create array to hold fee data
    let mut avg_fees: Array<AvgFees> = array![];

    // Process each fee entry
    let mut entry_idx = 0;
    while entry_idx < avg_fees_len {
        // Read timestamp (8 bytes)
        let mut timestamp: u64 = 0;
        let mut byte_idx = 0;
        while byte_idx < U64_SIZE {
            let current_byte: u64 = (*journal_bytes.at(byte_offset + byte_idx)).into();
            let shift_amount: u64 = byte_idx.into() * 8;
            let shifted_byte = BitShift::shl(current_byte, shift_amount);
            timestamp = timestamp | shifted_byte;
            byte_idx += 1;
        }
        byte_offset += U64_SIZE;
        // Read data_points (8 bytes)
        let mut data_points: u64 = 0;
        let mut byte_idx = 0;
        while byte_idx < U64_SIZE {
            let current_byte: u64 = (*journal_bytes.at(byte_offset + byte_idx)).into();
            let shift_amount: u64 = byte_idx.into() * 8;
            let shifted_byte = BitShift::shl(current_byte, shift_amount);
            data_points = data_points | shifted_byte;
            byte_idx += 1;
        }
        byte_offset += U64_SIZE;

        // Read the decimal string and convert to felt252
        byte_offset += U32_SIZE; // Skip length indicator (66, 0, 0, 0)
        byte_offset += HEX_PREFIX_SIZE; // Skip "0x" prefix
        let mut avg_fee: u256 = 0;
        let mut hex_idx = byte_offset;
        let hex_end = byte_offset + HEX_HASH_SIZE;
        loop {
            if hex_idx >= hex_end {
                break;
            }

            let shifted_hash: u256 = BitShift::shl(avg_fee, 4);
            let hex_byte: u256 = (*journal_bytes.at(hex_idx)).into();
            let hex_base: u256 = if hex_byte < 58 { // '0'-'9' vs 'a'-'f'
                ASCII_0 // ASCII '0'
            } else {
                ASCII_A_OFFSET // ASCII 'a' - 10
            };
            avg_fee = shifted_hash + hex_byte - hex_base;
            hex_idx += 1;
        }
        byte_offset += HEX_HASH_WITH_PREFIX_SIZE;
        avg_fees.append(AvgFees { timestamp, data_points, avg_fee: avg_fee.try_into().unwrap() });
        entry_idx += 1;
    }

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

        assert_eq!(journal.batch_index, 7639);
        assert_eq!(journal.latest_mmr_block, 7823359);
        assert_eq!(
            journal.latest_mmr_block_hash,
            0xec8093c5392b50867760c9387b03a6b58a480d6214f76101d1b00c6cc1c4c2ac,
        );
        assert_eq!(
            journal.root_hash, 0xdc10a8612fe834aa6b86385433e91e2c29d93fdb86d15c9998a4d53a9c0d51c6,
        );
        assert_eq!(journal.leaves_count, 1024);
        assert_eq!(
            journal.first_block_parent_hash,
            0x8e878300c656618a956ad4f35aa7348f314f9b509e47ef6d3a914cdb8c89ef94,
        );
        assert_eq!(avg_fees.len(), 5);

        assert_eq!(*avg_fees[0].timestamp, 1740981600);
        assert_eq!(*avg_fees[0].data_points, 47);
        assert_eq!(
            *avg_fees[0].avg_fee,
            0x000000000000000000000004c16b35e6ea368000000000000000000000000000,
        );

        assert_eq!(*avg_fees[1].timestamp, 1740985200);
        assert_eq!(*avg_fees[1].data_points, 284);
        assert_eq!(
            *avg_fees[1].avg_fee,
            0x000000000000000000000004c13c17e151208000000000000000000000000000,
        );

        assert_eq!(*avg_fees[2].timestamp, 1740988800);
        assert_eq!(*avg_fees[2].data_points, 292);
        assert_eq!(
            *avg_fees[2].avg_fee,
            0x00000000000000000000000425cf87e2a6934000000000000000000000000000,
        );

        assert_eq!(*avg_fees[3].timestamp, 1740992400);
        assert_eq!(*avg_fees[3].data_points, 290);
        assert_eq!(
            *avg_fees[3].avg_fee,
            0x000000000000000000000003c00e04e1bce90000000000000000000000000000,
        );

        assert_eq!(*avg_fees[4].timestamp, 1740996000);
        assert_eq!(*avg_fees[4].data_points, 111);
        assert_eq!(
            *avg_fees[4].avg_fee,
            0x0000000000000000000000043c0b1212dd67c000000000000000000000000000,
        );
    }


    fn get_journal_bytes() -> Span<u8> {
        array![
            166,
            147,
            30,
            0,
            0,
            0,
            0,
            0,
            153,
            78,
            122,
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
            56,
            50,
            57,
            48,
            51,
            55,
            99,
            99,
            54,
            100,
            52,
            56,
            102,
            54,
            48,
            57,
            53,
            98,
            50,
            101,
            102,
            54,
            56,
            53,
            98,
            54,
            57,
            53,
            50,
            100,
            54,
            54,
            100,
            52,
            51,
            52,
            99,
            57,
            102,
            102,
            50,
            52,
            99,
            53,
            102,
            97,
            55,
            49,
            56,
            97,
            56,
            49,
            102,
            101,
            99,
            51,
            101,
            50,
            97,
            98,
            100,
            99,
            102,
            57,
            0,
            0,
            66,
            0,
            0,
            0,
            48,
            120,
            54,
            98,
            50,
            100,
            98,
            97,
            57,
            99,
            97,
            51,
            99,
            53,
            52,
            57,
            98,
            99,
            99,
            54,
            100,
            48,
            98,
            51,
            55,
            98,
            97,
            49,
            100,
            51,
            55,
            56,
            57,
            56,
            101,
            100,
            97,
            97,
            48,
            57,
            97,
            101,
            97,
            52,
            99,
            97,
            100,
            49,
            50,
            57,
            49,
            100,
            55,
            101,
            49,
            54,
            51,
            52,
            102,
            97,
            55,
            97,
            48,
            51,
            52,
            57,
            0,
            0,
            2,
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
            56,
            101,
            100,
            49,
            50,
            48,
            54,
            50,
            48,
            48,
            102,
            49,
            50,
            99,
            50,
            101,
            50,
            101,
            52,
            54,
            54,
            55,
            50,
            51,
            99,
            97,
            98,
            101,
            51,
            57,
            50,
            55,
            101,
            53,
            54,
            52,
            56,
            98,
            102,
            56,
            100,
            101,
            101,
            100,
            50,
            97,
            57,
            51,
            99,
            55,
            100,
            97,
            50,
            54,
            48,
            56,
            52,
            49,
            99,
            49,
            99,
            100,
            49,
            55,
            0,
            0,
            1,
            0,
            0,
            0,
            160,
            134,
            233,
            103,
            0,
            0,
            0,
            0,
            2,
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
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            49,
            57,
            53,
            49,
            55,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            48,
            0,
            0,
        ]
            .span()
    }
}
