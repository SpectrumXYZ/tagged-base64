// Copyright © 2021 Translucence Research, Inc. All rights reserved.
//

//! User-oriented format for binary data. Tagged Base64 is intended to be
//! used in user interfaces including URLs and text to be copied and
//! pasted without the need for additional encoding, such as quoting or
//! escape sequences.
//!
//! Large binary values don't fit nicely into JavaScript numbers due to
//! range and representation. JavaScript numbers are represented as 64-bit
//! floating point numbers. This means that the largest unsigned integer
//! that can be represented is 2^53 - 1. Moreover, it is very easy to
//! accidentally coerce a string that looks like a number into a
//! JavaScript number, thus running the risk of loss of precision, which
//! is corruption.  Therefore, values are encoded in base64 to allow safe
//! transit to- and from JavaScript, including in URLs, as well as display
//! and input in a user interface.
//!
//! To further reduce confusion, the values are prefixed with a tag
//! intended to disambiguate usage. Although not necessary for
//! correctness, developers and users may find it convenient to have a
//! usage hint enabling them to see at a glance whether something is a
//! transaction id or a ledger address, etc.
//!
//! For example,
//!    TX~Zm9vYmFy
//!    LA~MzE0MTU
//!
//! Like the base64 value, the tag is also restricted to the URL-safe
//! base64 character set.
//!
//! Note: It is allowed for the tag or value to be the empty string. A
//! lone delimiter can be parsed as a tagged base64 value.
//!
//! Note: Integrating this with the Serde crate would be nice.

use base64;
use core::fmt;
use core::fmt::Display;
use crc_any::CRC;
use wasm_bindgen::prelude::*;

/// The tag string and the binary data.
#[wasm_bindgen]
#[derive(Debug, Eq, PartialEq)]
pub struct TaggedBase64 {
    tag: String,
    value: Vec<u8>,
    checksum: u8,
}

#[derive(Debug)]
pub enum TB64Error {
    /// An invalid character was found in the tag.
    InvalidTag,
    /// An invalid byte was found while decoding the base64-encoded value.
    /// The offset and offending byte are provided.
    InvalidByte(usize, u8),
    /// The length of the base64-encoded value is invalid.
    InvalidLength,
    /// The checksum did not match.
    InvalidChecksum,
}

/// Separator that does not appear in URL-safe base64 encoding and can
/// appear in URLs without percent-encoding.
pub const TB64_DELIM: char = '~';

/// Uses '-' and '_' as the 63rd and 64th characters. Does not use padding.
pub const TB64_CONFIG: base64::Config = base64::URL_SAFE_NO_PAD;

/// Converts a TaggedBase64 value to a String.
#[wasm_bindgen]
pub fn to_string(tb64: &TaggedBase64) -> String {
    let value = &mut tb64.value.clone();
    value.push(TaggedBase64::calc_checksum(&tb64.tag, &tb64.value));
    format!(
        "{}{}{}",
        tb64.tag,
        TB64_DELIM,
        TaggedBase64::encode_raw(value)
    )
}

impl From<&TaggedBase64> for String {
    fn from(tb64: &TaggedBase64) -> Self {
        let value = &mut tb64.value.clone();
        value.push(TaggedBase64::calc_checksum(&tb64.tag, &tb64.value));
        format!(
            "{}{}{}",
            tb64.tag,
            TB64_DELIM,
            TaggedBase64::encode_raw(value)
        )
    }
}

/// Produces a string by concatenating the tag and the base64 encoding
/// of the value, separated by a tilde (~).
impl fmt::Display for TaggedBase64 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let value = &mut self.value.clone();
        value.push(TaggedBase64::calc_checksum(&self.tag, &self.value));
        write!(
            f,
            "{}{}{}",
            self.tag,
            TB64_DELIM,
            TaggedBase64::encode_raw(value)
        )
    }
}

impl TaggedBase64 {
    /// Constructs a TaggedBase64 from a tag and array of bytes. The tag
    /// must be URL-safe (alphanumeric with hyphen and underscore). The
    /// byte values are unconstrained.
    pub fn new(tag: &str, value: &[u8]) -> Result<TaggedBase64, TB64Error> {
        if TaggedBase64::is_safe_base64_tag(tag) {
            let cs = TaggedBase64::calc_checksum(&tag, &value);
            Ok(TaggedBase64 {
                tag: tag.to_string(),
                value: value.to_vec(),
                checksum: cs,
            })
        } else {
            Err(TB64Error::InvalidTag)
        }
    }

    pub fn calc_checksum(tag: &str, value: &[u8]) -> u8 {
        let mut crc8 = CRC::crc8();
        crc8.digest(&tag.to_string());
        crc8.digest(&value);
        crc8.get_crc() as u8
    }

    /// Returns true for characters permitted in URL-safe base64 encoding,
    /// and false otherwise.
    pub fn is_safe_base64_ascii(c: char) -> bool {
        ('a'..='z').contains(&c)
            || ('A'..='Z').contains(&c)
            || ('0'..='9').contains(&c)
            || (c == '-')
            || (c == '_')
    }

    /// Checks that an ASCII byte is safe for use in the tag of a
    /// TaggedBase64. Because the tags are merely intended to be mnemonic,
    /// there's no need to support a large and visually ambiguous
    /// character set.
    pub fn is_safe_base64_tag(tag: &str) -> bool {
        tag.bytes()
            .skip_while(|b| TaggedBase64::is_safe_base64_ascii(*b as char))
            .next()
            .is_none()
    }

    /// Gets the tag of a TaggedBase64 instance.
    pub fn tag(&self) -> String {
        self.tag.clone()
    }

    /// Sets the tag of a TaggedBase64 instance.
    pub fn set_tag(&mut self, tag: &str) {
        assert!(TaggedBase64::is_safe_base64_tag(tag));
        self.tag = tag.to_string();
    }

    /// Gets the value of a TaggedBase64 instance.
    pub fn value(&self) -> Vec<u8> {
        self.value.clone()
    }

    /// Sets the value of a TaggedBase64 instance.
    pub fn set_value(&mut self, value: &[u8]) {
        self.value = value.to_vec();
    }

    /// Wraps the underlying base64 encoder.
    // WASM doesn't support the most general type.
    //
    // pub fn encode_raw<T: ?Sized + AsRef<[u8]>>(input: &T) -> String {
    //     base64::encode_config(input, TB64_CONFIG)
    // }
    pub fn encode_raw(input: &[u8]) -> String {
        base64::encode_config(input, TB64_CONFIG)
    }

    /// Wraps the underlying base64 decoder.
    // WASM doesn't support returning Result<Vec<u8>, base64::DecodeError>
    pub fn decode_raw(value: &str) -> Result<Vec<u8>, JsValue> {
        base64::decode_config(value, TB64_CONFIG).map_err(|err| to_jsvalue(err))
    }
    //}
}

/// Converts any object that supports the Display trait to a JsValue for
/// passing to Javascript.
///
/// Note: Type parameters aren't supported by `wasm-pack` yet so this
/// can't be included in the TaggedBase64 type implementation.
pub fn to_jsvalue<D: Display>(d: D) -> JsValue {
    JsValue::from_str(&format!("{}", d))
}

#[wasm_bindgen]
#[derive(Debug, Eq, PartialEq)]
pub struct JsTaggedBase64 {
    tb64: TaggedBase64,
}

#[wasm_bindgen]
impl JsTaggedBase64 {
    #[wasm_bindgen(constructor)]
    pub fn new(tag: &str, value: &[u8]) -> Result<TaggedBase64, JsValue> {
        if TaggedBase64::is_safe_base64_tag(tag) {
            let cs = TaggedBase64::calc_checksum(&tag, &value);
            Ok(TaggedBase64 {
                tag: tag.to_string(),
                value: value.to_vec(),
                checksum: cs,
            })
        } else {
            Err(to_jsvalue(format!(
            "Only alphanumeric ASCII, underscore (_), and hyphen (-) are allowed in the tag ({})",
            tag
        )))
        }
    }

    /// Parses a string of the form tag~value into a TaggedBase64 value.
    ///
    /// The tag is restricted to URL-safe base64 ASCII characters. The tag
    /// may be empty. The delimiter is required.
    ///
    /// The value is a base64-encoded string, using the URL-safe character
    /// set, and no padding is used.
    pub fn tagged_base64_from(tb64: &str) -> Result<TaggedBase64, JsValue> {
        // Would be convenient to use split_first() here. Alas, not stable yet.
        let delim_pos = tb64
            .find(TB64_DELIM)
            .ok_or(to_jsvalue("Missing delimiter parsing TaggedBase64"))?;
        let (tag, delim_b64) = tb64.split_at(delim_pos);

        if !TaggedBase64::is_safe_base64_tag(tag) {
            return Err(to_jsvalue(format!(
            "Only alphanumeric ASCII, underscore (_), and hyphen (-) are allowed in the tag ({})",
            tag
        )));
        }

        // Remove the delimiter.
        let mut iter = delim_b64.chars();
        iter.next();
        let value = iter.as_str();

        // Base64 decode the value.
        let bytes = TaggedBase64::decode_raw(value)?;
        let cs = bytes[0];

        if cs == TaggedBase64::calc_checksum(&tag, &bytes[1..]) {
            Ok(TaggedBase64 {
                tag: tag.to_string(),
                value: bytes[1..].to_vec(),
                checksum: cs,
            })
        } else {
            Err(to_jsvalue("Invalid JsTaggedBase64 checksum"))
        }
    }

    /// Constructs a TaggedBase64 from a tag string and a base64-encoded
    /// value.
    ///
    /// The tag is restricted to URL-safe base64 ASCII characters. The tag
    /// may be empty. The delimiter is required.  The value is a a
    /// base64-encoded string, using the URL-safe character set, and no
    /// padding is used.
    pub fn make_tagged_base64(tag: &str, value: &str) -> Result<TaggedBase64, JsValue> {
        if !TaggedBase64::is_safe_base64_tag(tag) {
            return Err(to_jsvalue(format!(
            "Only alphanumeric ASCII, underscore (_), and hyphen (-) are allowed in the tag ({})",
            tag
        )));
        }
        let bytes = TaggedBase64::decode_raw(value)?;
        let cs = bytes[0];

        if cs == TaggedBase64::calc_checksum(&tag, &bytes[1..]) {
            Ok(TaggedBase64 {
                tag: tag.to_string(),
                value: bytes[1..].to_vec(),
                checksum: cs,
            })
        } else {
            Err(to_jsvalue("Invalid JsTaggedBase64 checksum"))
        }
    }
}
