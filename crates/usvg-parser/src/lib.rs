// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
`usvg-parser` is an [SVG] parser used by [usvg].

[SVG]: https://en.wikipedia.org/wiki/Scalable_Vector_Graphics
[usvg]: https://github.com/RazrFalcon/resvg/tree/master/crates/usvg
*/

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(missing_copy_implementations)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::identity_op)]
#![allow(clippy::question_mark)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::upper_case_acronyms)]

mod clippath;
mod converter;
mod filter;
mod font;
mod image;
mod marker;
mod mask;
mod number;
mod options;
mod paint_server;
mod shapes;
mod stream;
mod style;
mod svgtree;
mod switch;
mod text;
mod units;
mod use_node;

use crate::stream::{ByteExt, Stream};

pub use crate::font::*;
pub use crate::number::*;
pub use crate::options::*;
pub use image::ImageHrefResolver;
pub use roxmltree;
pub use svgtree::{AId, EId};

/// List of all errors.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// Only UTF-8 content are supported.
    NotAnUtf8Str,

    /// Compressed SVG must use the GZip algorithm.
    MalformedGZip,

    /// We do not allow SVG with more than 1_000_000 elements for security reasons.
    ElementsLimitReached,

    /// SVG doesn't have a valid size.
    ///
    /// Occurs when width and/or height are <= 0.
    ///
    /// Also occurs if width, height and viewBox are not set.
    InvalidSize,
    /// An input data ended earlier than expected.
    ///
    /// Should only appear on invalid input data.
    /// Errors in a valid XML should be handled by errors below.
    UnexpectedEndOfStream,

    /// An input text contains unknown data.
    UnexpectedData(usize),

    /// A provided string doesn't have a valid data.
    ///
    /// For example, if we try to parse a color form `zzz`
    /// string - we will get this error.
    /// But if we try to parse a number list like `1.2 zzz`,
    /// then we will get `InvalidNumber`, because at least some data is valid.
    InvalidValue,

    /// An invalid ident.
    ///
    /// CSS idents have certain rules with regard to the characters they may contain.
    /// For example, they may not start with a number. If an invalid ident is encountered,
    /// this error will be returned.
    InvalidIdent,

    /// An invalid/unexpected character.
    ///
    /// The first byte is an actual one, others - expected.
    ///
    /// We are using a single value to reduce the struct size.
    InvalidChar(Vec<u8>, usize),

    /// An unexpected character instead of an XML space.
    ///
    /// The first string is an actual one, others - expected.
    ///
    /// We are using a single value to reduce the struct size.
    InvalidString(Vec<String>, usize),

    /// An invalid number.
    InvalidNumber(usize),

    /// Failed to parse an SVG data.
    ParsingFailed(roxmltree::Error),
}

impl From<roxmltree::Error> for Error {
    fn from(e: roxmltree::Error) -> Self {
        Error::ParsingFailed(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::NotAnUtf8Str => {
                write!(f, "provided data has not an UTF-8 encoding")
            }
            Error::MalformedGZip => {
                write!(f, "provided data has a malformed GZip content")
            }
            Error::ElementsLimitReached => {
                write!(f, "the maximum number of SVG elements has been reached")
            }
            Error::InvalidSize => {
                write!(f, "SVG has an invalid size")
            }
            Error::ParsingFailed(ref e) => {
                write!(f, "SVG data parsing failed cause {}", e)
            }
            Error::UnexpectedEndOfStream => {
                write!(f, "unexpected end of stream")
            }
            Error::UnexpectedData(pos) => {
                write!(f, "unexpected data at position {}", pos)
            }
            Error::InvalidValue => {
                write!(f, "invalid value")
            }
            Error::InvalidIdent => {
                write!(f, "invalid ident")
            }
            Error::InvalidChar(ref chars, pos) => {
                // Vec<u8> -> Vec<String>
                let list: Vec<String> = chars
                    .iter()
                    .skip(1)
                    .map(|c| String::from_utf8(vec![*c]).unwrap())
                    .collect();

                write!(
                    f,
                    "expected '{}' not '{}' at position {}",
                    list.join("', '"),
                    chars[0] as char,
                    pos
                )
            }
            Error::InvalidString(ref strings, pos) => {
                write!(
                    f,
                    "expected '{}' not '{}' at position {}",
                    strings[1..].join("', '"),
                    strings[0],
                    pos
                )
            }
            Error::InvalidNumber(pos) => {
                write!(f, "invalid number at position {}", pos)
            }
        }
    }
}

impl std::error::Error for Error {}

trait OptionLog {
    fn log_none<F: FnOnce()>(self, f: F) -> Self;
}

impl<T> OptionLog for Option<T> {
    #[inline]
    fn log_none<F: FnOnce()>(self, f: F) -> Self {
        self.or_else(|| {
            f();
            None
        })
    }
}

/// A trait to parse `usvg_tree::Tree` from various sources.
pub trait TreeParsing: Sized {
    /// Parses `Tree` from an SVG data.
    ///
    /// Can contain an SVG string or a gzip compressed data.
    fn from_data(data: &[u8], opt: &Options) -> Result<Self, Error>;

    /// Parses `Tree` from an SVG string.
    fn from_str(text: &str, opt: &Options) -> Result<Self, Error>;

    /// Parses `Tree` from `roxmltree::Document`.
    fn from_xmltree(doc: &roxmltree::Document, opt: &Options) -> Result<Self, Error>;
}

impl TreeParsing for usvg_tree::Tree {
    /// Parses `Tree` from an SVG data.
    ///
    /// Can contain an SVG string or a gzip compressed data.
    fn from_data(data: &[u8], opt: &Options) -> Result<Self, Error> {
        if data.starts_with(&[0x1f, 0x8b]) {
            let data = decompress_svgz(data)?;
            let text = std::str::from_utf8(&data).map_err(|_| Error::NotAnUtf8Str)?;
            Self::from_str(text, opt)
        } else {
            let text = std::str::from_utf8(data).map_err(|_| Error::NotAnUtf8Str)?;
            Self::from_str(text, opt)
        }
    }

    /// Parses `Tree` from an SVG string.
    fn from_str(text: &str, opt: &Options) -> Result<Self, Error> {
        let xml_opt = roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        };

        let doc =
            roxmltree::Document::parse_with_options(text, xml_opt).map_err(Error::ParsingFailed)?;

        Self::from_xmltree(&doc, opt)
    }

    /// Parses `Tree` from `roxmltree::Document`.
    fn from_xmltree(doc: &roxmltree::Document, opt: &Options) -> Result<Self, Error> {
        let doc = svgtree::Document::parse_tree(doc)?;
        crate::converter::convert_doc(&doc, opt)
    }
}

/// Decompresses an SVGZ file.
pub fn decompress_svgz(data: &[u8]) -> Result<Vec<u8>, Error> {
    use std::io::Read;

    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut decoded = Vec::with_capacity(data.len() * 2);
    decoder
        .read_to_end(&mut decoded)
        .map_err(|_| Error::MalformedGZip)?;
    Ok(decoded)
}

#[inline]
pub(crate) fn f32_bound(min: f32, val: f32, max: f32) -> f32 {
    debug_assert!(min.is_finite());
    debug_assert!(val.is_finite());
    debug_assert!(max.is_finite());

    if val > max {
        max
    } else if val < min {
        min
    } else {
        val
    }
}
