//! High-performance binary reader for TrueType/OpenType files.
//! Zero third-party dependencies, purely standard library Rust.

use std::fmt;

/// Errors that can occur while parsing TrueType binary font tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TtfError {
    /// Unexpected end of file while reading data.
    UnexpectedEof,
    /// Invalid SFNT version or table magic identifier.
    InvalidMagic([u8; 4]),
    /// Required font table was not found in directory.
    TableNotFound(&'static str),
    /// Malformed table or unsupported format version.
    InvalidFormat(&'static str),
    /// Glyph index was out of bounds for the font.
    InvalidGlyphIndex(u16),
    /// IO error reading font file from disk.
    IoError(String),
}

impl fmt::Display for TtfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TtfError::UnexpectedEof => write!(f, "Unexpected end of file while reading TTF stream"),
            TtfError::InvalidMagic(m) => write!(
                f,
                "Invalid TTF magic tag: 0x{:02X}{:02X}{:02X}{:02X}",
                m[0], m[1], m[2], m[3]
            ),
            TtfError::TableNotFound(t) => write!(f, "Required TTF table not found: '{}'", t),
            TtfError::InvalidFormat(msg) => write!(f, "Invalid TTF format: {}", msg),
            TtfError::InvalidGlyphIndex(id) => write!(f, "Glyph index {} out of bounds", id),
            TtfError::IoError(msg) => write!(f, "Failed to read font file: {}", msg),
        }
    }
}

impl std::error::Error for TtfError {}

/// Big-endian stream reader with strict bounds checking.
#[derive(Debug, Clone, Copy)]
pub struct TtfReader<'a> {
    data: &'a [u8],
    cursor: usize,
}

impl<'a> TtfReader<'a> {
    /// Creates a new reader from a byte slice.
    #[inline]
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data, cursor: 0 }
    }

    /// Creates a reader for a specific sub-range of bytes.
    pub fn sub_reader(&self, offset: usize, len: usize) -> Result<Self, TtfError> {
        let end = offset.checked_add(len).ok_or(TtfError::UnexpectedEof)?;
        if end > self.data.len() {
            return Err(TtfError::UnexpectedEof);
        }
        Ok(Self {
            data: &self.data[offset..end],
            cursor: 0,
        })
    }

    /// Sets cursor position within the slice.
    #[inline]
    pub fn seek(&mut self, pos: usize) -> Result<(), TtfError> {
        if pos > self.data.len() {
            Err(TtfError::UnexpectedEof)
        } else {
            self.cursor = pos;
            Ok(())
        }
    }

    /// Advances the cursor by `n` bytes.
    #[inline]
    pub fn skip(&mut self, n: usize) -> Result<(), TtfError> {
        let new_pos = self.cursor.checked_add(n).ok_or(TtfError::UnexpectedEof)?;
        self.seek(new_pos)
    }

    /// Current cursor position.
    #[inline]
    pub const fn tell(&self) -> usize {
        self.cursor
    }

    /// Total length in bytes.
    #[inline]
    pub const fn len(&self) -> usize {
        self.data.len()
    }

    /// Number of remaining bytes from cursor to end.
    #[inline]
    pub const fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.cursor)
    }

    /// Reads `n` raw bytes without copying.
    pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], TtfError> {
        let end = self.cursor.checked_add(n).ok_or(TtfError::UnexpectedEof)?;
        if end > self.data.len() {
            return Err(TtfError::UnexpectedEof);
        }
        let slice = &self.data[self.cursor..end];
        self.cursor = end;
        Ok(slice)
    }

    /// Reads an unsigned 8-bit integer.
    #[inline]
    pub fn read_u8(&mut self) -> Result<u8, TtfError> {
        if self.cursor >= self.data.len() {
            Err(TtfError::UnexpectedEof)
        } else {
            let val = self.data[self.cursor];
            self.cursor += 1;
            Ok(val)
        }
    }

    /// Reads a signed 8-bit integer.
    #[inline]
    pub fn read_i8(&mut self) -> Result<i8, TtfError> {
        self.read_u8().map(|v| v as i8)
    }

    /// Reads an unsigned 16-bit big-endian integer.
    #[inline]
    pub fn read_u16(&mut self) -> Result<u16, TtfError> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    /// Reads a signed 16-bit big-endian integer.
    #[inline]
    pub fn read_i16(&mut self) -> Result<i16, TtfError> {
        let bytes = self.read_bytes(2)?;
        Ok(i16::from_be_bytes([bytes[0], bytes[1]]))
    }

    /// Reads an unsigned 32-bit big-endian integer.
    #[inline]
    pub fn read_u32(&mut self) -> Result<u32, TtfError> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Reads a signed 32-bit big-endian integer.
    #[inline]
    pub fn read_i32(&mut self) -> Result<i32, TtfError> {
        let bytes = self.read_bytes(4)?;
        Ok(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Reads a 2.14 signed fixed-point value as `f32`.
    #[inline]
    pub fn read_f2dot14(&mut self) -> Result<f32, TtfError> {
        let raw = self.read_i16()?;
        Ok((raw as f32) / 16384.0)
    }

    /// Reads a 16.16 signed fixed-point value as `f32`.
    #[inline]
    pub fn read_fixed(&mut self) -> Result<f32, TtfError> {
        let raw = self.read_i32()?;
        Ok((raw as f32) / 65536.0)
    }

    /// Peeks a 16-bit big-endian integer at given absolute offset without advancing cursor.
    pub fn peek_u16_at(&self, offset: usize) -> Result<u16, TtfError> {
        let end = offset.checked_add(2).ok_or(TtfError::UnexpectedEof)?;
        if end > self.data.len() {
            Err(TtfError::UnexpectedEof)
        } else {
            Ok(u16::from_be_bytes([self.data[offset], self.data[offset + 1]]))
        }
    }

    /// Peeks a 32-bit big-endian integer at given absolute offset without advancing cursor.
    pub fn peek_u32_at(&self, offset: usize) -> Result<u32, TtfError> {
        let end = offset.checked_add(4).ok_or(TtfError::UnexpectedEof)?;
        if end > self.data.len() {
            Err(TtfError::UnexpectedEof)
        } else {
            Ok(u32::from_be_bytes([
                self.data[offset],
                self.data[offset + 1],
                self.data[offset + 2],
                self.data[offset + 3],
            ]))
        }
    }
}
