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
    let mut byte_offset = 0; // Current position in the byte array

    // Parse batch_index
    let mut batch_index: u64 = 0;
    let mut byte_idx = 0;
    while byte_idx < 8 {
        let current_byte: u64 = (*journal_bytes.at(byte_offset + byte_idx)).into();
        let shifted_byte: u64 = BitShift::shl(current_byte, 8 * byte_idx.into());
        batch_index += shifted_byte;
        byte_idx += 1;
    };

    // Parse latest_mmr_block
    byte_offset += 8;
    let mut latest_mmr_block: u64 = 0;
    let mut byte_idx = 0;
    while byte_idx < 8 {
        let current_byte: u64 = (*journal_bytes.at(byte_offset + byte_idx)).into();
        let shifted_byte: u64 = BitShift::shl(current_byte, 8 * byte_idx.into());
        latest_mmr_block += shifted_byte;
        byte_idx += 1;
    };

    // Parse latest_mmr_block_hash
    byte_offset += 8; // Skip to start of hash length
    byte_offset += 4; // Skip length indicator (66, 0, 0, 0)
    byte_offset += 2; // Skip "0x" prefix
    let mut latest_mmr_block_hash: u256 = 0;
    let mut hex_idx = byte_offset;
    let hex_end = byte_offset + 64; // 64 hex characters for 32 bytes

    loop {
        if hex_idx >= hex_end {
            break;
        }

        let shifted_hash: u256 = BitShift::shl(latest_mmr_block_hash, 4);
        let hex_byte: u256 = (*journal_bytes.at(hex_idx)).into();
        let hex_base: u256 = if hex_byte < 58 { // '0'-'9' vs 'a'-'f'
            48 // ASCII '0'
        } else {
            87 // ASCII 'a' - 10
        };
        latest_mmr_block_hash = shifted_hash + hex_byte - hex_base;
        hex_idx += 1;
    };

    // Parse root_hash
    byte_offset += 66; // Skip past latest_mmr_block_hash (64 hex chars + "0x")
    byte_offset += 4; // Skip length indicator (66, 0, 0, 0)
    byte_offset += 2; // Skip "0x" prefix
    let mut root_hash: u256 = 0;
    let mut hex_idx = byte_offset;
    let hex_end = byte_offset + 64; // 64 hex characters for 32 bytes

    loop {
        if hex_idx >= hex_end {
            break;
        }

        let shifted_hash: u256 = BitShift::shl(root_hash, 4);
        let hex_byte: u256 = (*journal_bytes.at(hex_idx)).into();
        let hex_base: u256 = if hex_byte < 58 { // '0'-'9' vs 'a'-'f'
            48 // ASCII '0'
        } else {
            87 // ASCII 'a' - 10
        };
        root_hash = shifted_hash + hex_byte - hex_base;
        hex_idx += 1;
    };

    // Parse leaves_count
    byte_offset += 66;
    let mut leaves_count: u64 = 0;
    let mut byte_idx = 0;
    while byte_idx < 8 {
        let current_byte: u64 = (*journal_bytes.at(byte_offset + byte_idx)).into();
        let shifted_byte: u64 = BitShift::shl(current_byte, 8 * byte_idx.into());
        leaves_count += shifted_byte;
        byte_idx += 1;
    };

    // Parse first_block_parent_hash
    byte_offset += 8;
    byte_offset += 4; // Skip length indicator (66, 0, 0, 0)
    byte_offset += 2; // Skip "0x" prefix
    let mut first_block_parent_hash: u256 = 0;
    let mut hex_idx = byte_offset;
    let hex_end = byte_offset + 64;

    loop {
        if hex_idx >= hex_end {
            break;
        }

        let shifted_hash: u256 = BitShift::shl(first_block_parent_hash, 4);
        let hex_byte: u256 = (*journal_bytes.at(hex_idx)).into();
        let hex_base: u256 = if hex_byte < 58 { // '0'-'9' vs 'a'-'f'
            48 // ASCII '0'
        } else {
            87 // ASCII 'a' - 10
        };
        first_block_parent_hash = shifted_hash + hex_byte - hex_base;
        hex_idx += 1;
    };

    // Parse avg_fees
    byte_offset += 66;

    // Read the number of fee entries
    let mut avg_fees_len: usize = 0;
    let mut byte_idx = 0;
    while byte_idx < 4 {
        let current_byte: u32 = (*journal_bytes.at(byte_offset + byte_idx)).into();
        let shifted_byte: u32 = BitShift::shl(current_byte, 8 * byte_idx.into());
        avg_fees_len += shifted_byte;
        byte_idx += 1;
    };
    byte_offset += 4;

    // Create array to hold fee data
    let mut avg_fees: Array<AvgFees> = array![];

    // Process each fee entry
    let mut entry_idx = 0;
    while entry_idx < avg_fees_len {
        // Read timestamp (8 bytes)
        let mut timestamp: u64 = 0;
        let mut byte_idx = 0;
        while byte_idx < 8 {
            let current_byte: u64 = (*journal_bytes.at(byte_offset + byte_idx)).into();
            let shift_amount: u64 = byte_idx.into() * 8;
            let shifted_byte = BitShift::shl(current_byte, shift_amount);
            timestamp = timestamp | shifted_byte;
            byte_idx += 1;
        };
        byte_offset += 8;

        // Read data_points (8 bytes)
        let mut data_points: u64 = 0;
        let mut byte_idx = 0;
        while byte_idx < 8 {
            let current_byte: u64 = (*journal_bytes.at(byte_offset + byte_idx)).into();
            let shift_amount: u64 = byte_idx.into() * 8;
            let shifted_byte = BitShift::shl(current_byte, shift_amount);
            data_points = data_points | shifted_byte;
            byte_idx += 1;
        };
        byte_offset += 8;

        // Read the length of the decimal string (4 bytes)
        let mut string_len: usize = 0;
        let mut byte_idx = 0;
        while byte_idx < 4 {
            let current_byte: u32 = (*journal_bytes.at(byte_offset + byte_idx)).into();
            let shifted_byte: u32 = BitShift::shl(current_byte, 8 * byte_idx.into());
            string_len += shifted_byte;
            byte_idx += 1;
        };
        byte_offset += 4;

        // Read the decimal string and convert to felt252
        let mut avg_fee: felt252 = 0;
        let mut char_idx = 0;
        while char_idx < string_len {
            // Read each digit as ASCII
            let digit_ascii: felt252 = (*journal_bytes.at(byte_offset + char_idx)).into();
            // Convert ASCII to digit (ASCII '0' is 48)
            let digit: felt252 = digit_ascii - 48;
            // Multiply current value by 10 and add digit
            avg_fee = avg_fee * 10 + digit;
            char_idx += 1;
        };
        byte_offset += string_len;

        avg_fees.append(AvgFees { timestamp, data_points, avg_fee });
        entry_idx += 1;
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

        assert_eq!(journal.batch_index, 6836);
        assert_eq!(journal.latest_mmr_block, 7001024);
        assert_eq!(
            journal.latest_mmr_block_hash,
            0x687921aa26d5b9cb402c973399239b064eeb9a5244ad2d3a9502e30f943dd651,
        );
        assert_eq!(
            journal.root_hash, 0x5f8ffc504770b31d1f3d35a94adc963618dbf11f0dac7d38752d39bc6e056a22,
        );
        assert_eq!(journal.leaves_count, 961);
        assert_eq!(
            journal.first_block_parent_hash,
            0xfa95fcfe69836342d5c2055625febc42f195894ce09269444ad006ae1ecfc67b,
        );
        assert_eq!(avg_fees.len(), 5);

        assert_eq!(*avg_fees[0].timestamp, 1730588400);
        assert_eq!(*avg_fees[0].data_points, 40);
        assert_eq!(*avg_fees[0].avg_fee, 598075904616818972094179355334047654844540387328);

        assert_eq!(*avg_fees[1].timestamp, 1730592000);
        assert_eq!(*avg_fees[1].data_points, 278);
        assert_eq!(*avg_fees[1].avg_fee, 502289900871589525872488034409481539267487334400);

        assert_eq!(*avg_fees[2].timestamp, 1730595600);
        assert_eq!(*avg_fees[2].data_points, 275);
        assert_eq!(*avg_fees[2].avg_fee, 371817754130332701524257909486397809448615673856);

        assert_eq!(*avg_fees[3].timestamp, 1730599200);
        assert_eq!(*avg_fees[3].data_points, 281);
        assert_eq!(*avg_fees[3].avg_fee, 374242570500456464922378181161976959228671164416);

        assert_eq!(*avg_fees[4].timestamp, 1730602800);
        assert_eq!(*avg_fees[4].data_points, 87);
        assert_eq!(*avg_fees[4].avg_fee, 2278501099249657563423332593325301706948060643328);
    }

    fn get_journal_bytes() -> Span<u8> {
        array![
            180,
            26,
            0,
            0,
            0,
            0,
            0,
            0,
            192,
            211,
            106,
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
            54,
            56,
            55,
            57,
            50,
            49,
            97,
            97,
            50,
            54,
            100,
            53,
            98,
            57,
            99,
            98,
            52,
            48,
            50,
            99,
            57,
            55,
            51,
            51,
            57,
            57,
            50,
            51,
            57,
            98,
            48,
            54,
            52,
            101,
            101,
            98,
            57,
            97,
            53,
            50,
            52,
            52,
            97,
            100,
            50,
            100,
            51,
            97,
            57,
            53,
            48,
            50,
            101,
            51,
            48,
            102,
            57,
            52,
            51,
            100,
            100,
            54,
            53,
            49,
            0,
            0,
            66,
            0,
            0,
            0,
            48,
            120,
            53,
            102,
            56,
            102,
            102,
            99,
            53,
            48,
            52,
            55,
            55,
            48,
            98,
            51,
            49,
            100,
            49,
            102,
            51,
            100,
            51,
            53,
            97,
            57,
            52,
            97,
            100,
            99,
            57,
            54,
            51,
            54,
            49,
            56,
            100,
            98,
            102,
            49,
            49,
            102,
            48,
            100,
            97,
            99,
            55,
            100,
            51,
            56,
            55,
            53,
            50,
            100,
            51,
            57,
            98,
            99,
            54,
            101,
            48,
            53,
            54,
            97,
            50,
            50,
            0,
            0,
            193,
            3,
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
            102,
            97,
            57,
            53,
            102,
            99,
            102,
            101,
            54,
            57,
            56,
            51,
            54,
            51,
            52,
            50,
            100,
            53,
            99,
            50,
            48,
            53,
            53,
            54,
            50,
            53,
            102,
            101,
            98,
            99,
            52,
            50,
            102,
            49,
            57,
            53,
            56,
            57,
            52,
            99,
            101,
            48,
            57,
            50,
            54,
            57,
            52,
            52,
            52,
            97,
            100,
            48,
            48,
            54,
            97,
            101,
            49,
            101,
            99,
            102,
            99,
            54,
            55,
            98,
            0,
            0,
            5,
            0,
            0,
            0,
            240,
            174,
            38,
            103,
            0,
            0,
            0,
            0,
            40,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            48,
            0,
            0,
            0,
            53,
            57,
            56,
            48,
            55,
            53,
            57,
            48,
            52,
            54,
            49,
            54,
            56,
            49,
            56,
            57,
            55,
            50,
            48,
            57,
            52,
            49,
            55,
            57,
            51,
            53,
            53,
            51,
            51,
            52,
            48,
            52,
            55,
            54,
            53,
            52,
            56,
            52,
            52,
            53,
            52,
            48,
            51,
            56,
            55,
            51,
            50,
            56,
            0,
            189,
            38,
            103,
            0,
            0,
            0,
            0,
            22,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            48,
            0,
            0,
            0,
            53,
            48,
            50,
            50,
            56,
            57,
            57,
            48,
            48,
            56,
            55,
            49,
            53,
            56,
            57,
            53,
            50,
            53,
            56,
            55,
            50,
            52,
            56,
            56,
            48,
            51,
            52,
            52,
            48,
            57,
            52,
            56,
            49,
            53,
            51,
            57,
            50,
            54,
            55,
            52,
            56,
            55,
            51,
            51,
            52,
            52,
            48,
            48,
            16,
            203,
            38,
            103,
            0,
            0,
            0,
            0,
            19,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            48,
            0,
            0,
            0,
            51,
            55,
            49,
            56,
            49,
            55,
            55,
            53,
            52,
            49,
            51,
            48,
            51,
            51,
            50,
            55,
            48,
            49,
            53,
            50,
            52,
            50,
            53,
            55,
            57,
            48,
            57,
            52,
            56,
            54,
            51,
            57,
            55,
            56,
            48,
            57,
            52,
            52,
            56,
            54,
            49,
            53,
            54,
            55,
            51,
            56,
            53,
            54,
            32,
            217,
            38,
            103,
            0,
            0,
            0,
            0,
            25,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            48,
            0,
            0,
            0,
            51,
            55,
            52,
            50,
            52,
            50,
            53,
            55,
            48,
            53,
            48,
            48,
            52,
            53,
            54,
            52,
            54,
            52,
            57,
            50,
            50,
            51,
            55,
            56,
            49,
            56,
            49,
            49,
            54,
            49,
            57,
            55,
            54,
            57,
            53,
            57,
            50,
            50,
            56,
            54,
            55,
            49,
            49,
            54,
            52,
            52,
            49,
            54,
            48,
            231,
            38,
            103,
            0,
            0,
            0,
            0,
            87,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            49,
            0,
            0,
            0,
            50,
            50,
            55,
            56,
            53,
            48,
            49,
            48,
            57,
            57,
            50,
            52,
            57,
            54,
            53,
            55,
            53,
            54,
            51,
            52,
            50,
            51,
            51,
            51,
            50,
            53,
            57,
            51,
            51,
            50,
            53,
            51,
            48,
            49,
            55,
            48,
            54,
            57,
            52,
            56,
            48,
            54,
            48,
            54,
            52,
            51,
            51,
            50,
            56,
            0,
            0,
            0,
        ]
            .span()
    }
}
