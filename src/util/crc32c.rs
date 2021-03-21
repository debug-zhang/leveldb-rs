// Copyright (c) 2021, storagezhang <storagezhang@outlook.com>. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file. See the AUTHORS file for names of contributors.

extern crate crc;

const MASK_DELTA: u32 = 0xa282ead8;

/// Return the crc32c of data[0,n-1]
#[inline]
pub fn value(data: &[u8]) -> u32 {
    crc::crc32::checksum_castagnoli(data)
}

/// Return a masked representation of crc.
///
/// Motivation: it is problematic to compute the CRC of a string that
/// contains embedded CRCs.  Therefore we recommend that CRCs stored
/// somewhere (e.g., in files) should be masked before being stored.
#[inline]
pub fn mask(crc: u32) -> u32 {
    // Rotate right by 15 bits and add a constant.
    ((crc >> 15) | (crc << 17)) + MASK_DELTA
}

/// Return the crc whose masked representation is masked_crc.
#[inline]
pub fn unmask(masked_crc: u32) -> u32 {
    let rot = masked_crc - MASK_DELTA;
    (rot >> 17) | (rot << 15)
}

#[cfg(test)]
mod tests {
    use super::value;
    use super::mask;
    use super::unmask;

    #[test]
    pub fn standard_results() {
        // From rfc3720 section B.4.
        let mut buf: Vec<u8> = vec![0; 32];
        assert_eq!(value(&buf), 0x8a9136aa);

        buf = vec![0xff; 32];
        assert_eq!(value(&buf), 0x62a8ab43);

        for i in 0..32 {
            buf[i] = i as u8;
        }
        assert_eq!(value(&buf), 0x46dd794e);

        for i in 0..32 {
            buf[i] = (31 - i) as u8;
        }
        assert_eq!(value(&buf), 0x113fdb5c);

        let data = [
            0x01, 0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
            0x00, 0x14, 0x00, 0x00, 0x00, 0x18, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert_eq!(value(&data), 0xd9963a56);
    }

    #[test]
    pub fn values() {
        assert_ne!(value("a".as_bytes()), value("foo".as_bytes()));
    }

    #[test]
    pub fn mask_and_umask() {
        let crc = value("foo".as_bytes());
        assert_ne!(crc, mask(crc));
        assert_ne!(crc, mask(mask(crc)));
        assert_eq!(crc, unmask(mask(crc)));
        assert_eq!(crc, unmask(unmask(mask(mask(crc)))));
    }
}