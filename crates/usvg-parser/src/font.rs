// Copyright 2024 the SVG Types Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::stream::{ByteExt, Stream};
use crate::Error;

/// Parses a list of font families and generic families from a string.
pub fn parse_font_families(text: &str) -> Result<Vec<FontFamily>, Error> {
    let mut s = Stream::from(text);
    let font_families = s.parse_font_families()?;

    s.skip_spaces();
    if !s.at_end() {
        return Err(Error::UnexpectedData(s.calc_char_pos()));
    }

    Ok(font_families)
}

/// A type of font family.
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum FontFamily {
    /// A serif font.
    Serif,
    /// A sans-serif font.
    SansSerif,
    /// A cursive font.
    Cursive,
    /// A fantasy font.
    Fantasy,
    /// A monospace font.
    Monospace,
    /// A custom named font.
    Named(String),
}

impl ToString for FontFamily {
    fn to_string(&self) -> String {
        match self {
            FontFamily::Serif => "serif".to_string(),
            FontFamily::SansSerif => "sans-serif".to_string(),
            FontFamily::Cursive => "cursive".to_string(),
            FontFamily::Fantasy => "fantasy".to_string(),
            FontFamily::Monospace => "monospace".to_string(),
            FontFamily::Named(name) => name.clone(),
        }
    }
}

impl Stream<'_> {
    pub fn parse_font_families(&mut self) -> Result<Vec<FontFamily>, Error> {
        let mut families = vec![];

        while !self.at_end() {
            self.skip_spaces();

            let family = {
                let ch = self.curr_byte()?;
                if ch == b'\'' || ch == b'\"' {
                    let res = self.parse_quoted_string()?;
                    FontFamily::Named(res.to_string())
                } else {
                    let mut idents = vec![];

                    while let Some(c) = self.chars().next() {
                        if c != ',' {
                            idents.push(self.parse_ident()?.to_string());
                            self.skip_spaces();
                        } else {
                            break;
                        }
                    }

                    let joined = idents.join(" ");

                    // TODO: No CSS keyword must be matched as a family name...
                    match joined.as_str() {
                        "serif" => FontFamily::Serif,
                        "sans-serif" => FontFamily::SansSerif,
                        "cursive" => FontFamily::Cursive,
                        "fantasy" => FontFamily::Fantasy,
                        "monospace" => FontFamily::Monospace,
                        _ => FontFamily::Named(joined),
                    }
                }
            };

            families.push(family);

            if let Ok(b) = self.curr_byte() {
                if b == b',' {
                    self.advance(1);
                } else {
                    break;
                }
            }
        }

        let families = families
            .into_iter()
            .filter(|f| match f {
                FontFamily::Named(s) => !s.is_empty(),
                _ => true,
            })
            .collect();

        Ok(families)
    }
}