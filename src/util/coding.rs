// Copyright (c) 2021, storagezhang <storagezhang@outlook.com>. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file. See the AUTHORS file for names of contributors.

use std::mem::transmute;
use std::ptr::copy_nonoverlapping;
use crate::util::slice::Slice;

/// Encode `value` in little-endian and put it in the first 4-bytes of `dst`.
///
/// Panic if `dst.len()` is less than 4.
#[inline]
pub fn encode_fixed_32(dst: &mut [u8], value: u32) {
    assert!(dst.len() >= 4);
    unsafe {
        let bytes = transmute::<_, [u8; 4]>(value.to_le());
        copy_nonoverlapping(bytes.as_ptr(), dst.as_mut_ptr(), 4);
    }
}

/// Encode `value` in little-endian and append it to `dst`.
/// This will increase the capacity of `dst` if there's not enough space.
pub fn put_fixed_32(dst: &mut Vec<u8>, value: u32) {
    let mut buf: [u8; 4] = [0; 4];
    encode_fixed_32(&mut buf, value);
    dst.extend_from_slice(&mut buf);
}

/// Decode the first 4-bytes of `src` in little-endian.
///
/// Return the decoded value.
///
/// Panic if `src.len()` is less than 4.
#[inline]
pub fn decode_fixed_32(src: &[u8]) -> u32 {
    assert!(src.len() >= 4);
    let mut data: u32 = 0;
    unsafe {
        copy_nonoverlapping(src.as_ptr(), (&mut data as *mut u32) as *mut u8, 4);
    }
    data.to_le()
}

/// Encode `value` in little-endian and put it in the first 8-bytes of `dst`.
///
/// Panic if `dst.len()` is less than 8.
#[inline]
pub fn encode_fixed_64(dst: &mut [u8], value: u64) {
    assert!(dst.len() >= 8);
    unsafe {
        let bytes = transmute::<_, [u8; 8]>(value.to_le());
        copy_nonoverlapping(bytes.as_ptr(), dst.as_mut_ptr(), 8);
    }
}

/// Encode `value` in little-endian and append it to `dst`.
/// This will increase the capacity of `dst` if there's not enough space.
pub fn put_fixed_64(dst: &mut Vec<u8>, value: u64) {
    let mut buf: [u8; 8] = [0; 8];
    encode_fixed_64(&mut buf, value);
    dst.extend_from_slice(&mut buf);
}

/// Decode the first 8-bytes of `src` in little-endian.
///
/// Return the decoded value.
///
/// Panic if `src.len()` is less than 8.
#[inline]
pub fn decode_fixed_64(src: &[u8]) -> u64 {
    assert!(src.len() >= 8);
    let mut data: u64 = 0;
    unsafe {
        copy_nonoverlapping(src.as_ptr(), (&mut data as *mut u64) as *mut u8, 8);
    }
    data.to_le()
}

/// Encode `value` in varint32 and put it in the first `N`-bytes of `dst`.
///
/// Panic if `dst` doesn't have enough space to encode the value.
pub fn encode_varint_32(dst: &mut [u8], value: u32) {
    const B: u32 = 0b10000000;

    if value < (1 << 7) {
        dst[0] = value as u8;
    } else if value < (1 << 14) {
        dst[0] = (value | B) as u8;
        dst[1] = (value >> 7) as u8;
    } else if value < (1 << 21) {
        dst[0] = (value | B) as u8;
        dst[1] = (value >> 7) as u8;
        dst[2] = (value >> 14) as u8;
    } else if value < (1 << 28) {
        dst[0] = (value | B) as u8;
        dst[1] = (value >> 7) as u8;
        dst[2] = (value >> 14) as u8;
        dst[3] = (value >> 21) as u8;
    } else {
        dst[0] = (value | B) as u8;
        dst[1] = (value >> 7) as u8;
        dst[2] = (value >> 14) as u8;
        dst[3] = (value >> 21) as u8;
        dst[4] = (value >> 28) as u8;
    }
}

/// Encode `value` in varint32 and append it to the last `N`-bytes of `dst`.
/// This will increase the capacity of `dst` if there's not enough space.
pub fn put_varint_32(dst: &mut Vec<u8>, value: u32) {
    let mut buf: Vec<u8> = Vec::with_capacity(5);
    encode_varint_32(&mut buf, value);
    dst.append(&mut buf);
}

/// Internal routine for use by fallback path of GetVarint32Ptr
/// TODO
pub fn get_varint_32_ptr_fall_back() {}

/// Decode varint32 from `src`, and returns a tuple of which the first element is the
/// decoded value, and the second element is the number of bytes used to encode the result
/// value.
///
/// Returns error if `src` doesn't contain a valid varint32.
/// TODO
// pub fn get_varint_32(src: &[u8]) -> Result<(u64, usize)> {}

/// Decode the varint32 encoded u32 value from the `input`,
/// and advance the slice past the decoded value.
///
/// Returns a u32 value if the decoding is successful, otherwise returns error.
/// TODO
/*
pub fn get_varint_32_slice(input: &mut Slice) -> Result<u32> {
    let (result, len) = get_varint_32(input.slice_data())?;
    input.remove_prefix(len);
    Ok(result)
}
*/

/// Encode `value` in varint64 and put it in the first `N`-bytes of `dst`.
///
/// Panic if `dst` doesn't have enough space to encode the value.
pub fn encode_varint_64(dst: &mut [u8], mut value: u64) {
    const B: u64 = 0b10000000;

    let mut idx = 0;
    while value >= B {
        dst[idx] = (value | B) as u8;
        idx += 1;
        value >>= 7;
    }
    dst[idx] = value as u8;
}

/// Encode `value` in varint64 and append it to the last `N`-bytes of `dst`.
/// This will increase the capacity of `dst` if there's not enough space.
pub fn put_varint_64(dst: &mut Vec<u8>, value: u64) {
    let mut buf: Vec<u8> = Vec::with_capacity(10);
    encode_varint_64(&mut buf, value);
    dst.append(&mut buf);
}

/// TODO
pub fn get_varint_64_ptr() {}

pub fn get_varint_64_slice() {}

pub fn get_varint_64() {}

/// Encode the slice `value` using length prefixed encoding,
/// and append the encoded value to `dst` .
pub fn put_length_prefixed_slice(dst: &mut Vec<u8>, value: &Slice) {
    put_varint_32(dst, value.size() as u32);
    dst.extend_from_slice(value.slice_data());
}

/// Decode the value from the slice using length-prefixed encoding,
/// and advance the slice past the value.
///
/// Return a slice which contains the decoded value, or error if the input is malformed.
/// TODO
/*
pub fn get_length_prefixed_slice(input: &mut Slice) -> Result<Slice> {
    let len = get_varint_32() as usize;
}
*/

/// Return the length of the varint32 or varint64 encoding of `value`.
pub fn varint_length(mut value: u64) -> usize {
    let mut len = 1;
    while value >= 128 {
        value >>= 7;
        len += 1;
    }
    len
}


#[cfg(test)]
mod tests {
    // use create::util::random::Random;

    #[test]
    fn fixed_32() {}

    #[test]
    fn fixed_64() {}

    #[test]
    fn varint_32() {}

    #[test]
    fn varint_64() {}

    #[test]
    fn put_varint_32() {}

    #[test]
    fn put_varint_64() {}

    #[test]
    fn varint_length() {
        /*
        let rand=Random::new(0xFFFFFFFF);
        let mut data = vec![0;5];
        for _ in 0..1000{
            let value = rand.next();
        }
        */
    }

    #[test]
    fn length_prefixed_slice() {}
}