// Copyright (c) 2021, storagezhang <storagezhang@outlook.com>. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file. See the AUTHORS file for names of contributors.

use super::coding::decode_fixed_32;

pub fn hash(data: &[u8], seed: u32) -> u32 {
    // Similar to murmur hash
    const M: u32 = 0xc6a4a793;
    const R: u32 = 24;
    let n = data.len();
    let mut h = seed ^ (M.wrapping_mul(n as u32));

    // Pick up four bytes at a time
    let mut i = 0;
    while i + 4 <= n {
        let w = decode_fixed_32(&data[i..]);
        i += 4;
        h += w;
        h = h.wrapping_mul(M);
        h ^= h >> 16;
    }

    // Pick up remaining bytes
    let remainder = n - i;
    if remainder > 2 {
        h += (data[i + 2] as u32) << 16;
    }
    if remainder > 1 {
        h += (data[i + 1] as u32) << 8;
    }
    if remainder > 0 {
        h += data[i] as u32;
        h = h.wrapping_mul(M);
        h ^= h >> R;
    }

    h
}

#[cfg(test)]
mod tests {
    use crate::util::hash::hash;

    #[test]
    fn test() {
        let data1: [u8; 1] = [0x62];
        let data2: [u8; 2] = [0xc3, 0x97];
        let data3: [u8; 3] = [0xe2, 0x99, 0xa5];
        let data4: [u8; 4] = [0xe1, 0x80, 0xb9, 0x32];
        let data5: [u8; 48] = [
            0x01, 0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
            0x00, 0x14, 0x00, 0x00, 0x00, 0x18, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        // 0xbc9f1d34 = 3164544308
        assert_eq!(hash(&[], 0xbc9f1d34), 0xbc9f1d34);
        // 0xbc9f1d34 = 3164544308, 0xef1345c4 = 4011017668
        assert_eq!(hash(&data1, 0xbc9f1d34), 0xef1345c4);
        // 0xbc9f1d34 = 3164544308, 0x5b663814 = 1533425684
        assert_eq!(hash(&data2, 0xbc9f1d34), 0x5b663814);
        // 0xbc9f1d34 = 3164544308, 0x323c078f = 842794895
        assert_eq!(hash(&data3, 0xbc9f1d34), 0x323c078f);
        // 0xbc9f1d34 = 3164544308, 0xed21633a = 3978388282
        assert_eq!(hash(&data4, 0xbc9f1d34), 0xed21633a);
        // 0xbc9f1d34 = 3164544308, 0xf333dabb = 4080261819
        assert_eq!(hash(&data5, 0x12345678), 0xf333dabb);
    }
}
