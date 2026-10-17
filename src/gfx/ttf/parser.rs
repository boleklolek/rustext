//! Pure Rust TrueType (.ttf / .otf SFNT) parser.
//! Parses table directories, header, metrics, cmap character mappings, loca offsets,
//! and simple & composite glyph outlines from the glyf table.
//! Zero third-party dependencies, purely standard library Rust.

use std::collections::BTreeMap;
use super::outline::{Contour, GlyphOutline, OutlinePoint};
use super::reader::{TtfError, TtfReader};

/// Horizontal typography metrics for a single glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphMetrics {
    /// Horizontal advance width in font design units.
    pub advance_width: u16,
    /// Left side bearing in font design units.
    pub left_side_bearing: i16,
}

/// Parsed TrueType Font representation.
#[derive(Debug, Clone)]
pub struct TrueTypeFont {
    /// Raw font binary data.
    data: Vec<u8>,
    /// Table offsets and lengths keyed by 4-byte tag.
    tables: BTreeMap<[u8; 4], (usize, usize)>,
    /// Units per EM square (typically 1000 or 2048).
    pub units_per_em: u16,
    /// 0 for 16-bit short loca offsets; 1 for 32-bit long loca offsets.
    index_to_loc_format: i16,
    /// Total number of glyphs in this font.
    pub num_glyphs: u16,
    /// Typographical ascender above baseline in design units.
    pub ascender: i16,
    /// Typographical descender below baseline in design units (usually negative).
    pub descender: i16,
    /// Recommended line gap between consecutive lines in design units.
    pub line_gap: i16,
    /// Number of horizontal metrics entries in `hmtx`.
    number_of_hmetrics: u16,
    /// Global bounding box.
    pub bbox: (i16, i16, i16, i16),
    /// Font family display name from `name` table.
    pub family_name: String,
    /// Subfamily / style name (e.g. Regular, Bold).
    pub style_name: String,
    /// Cached selected cmap subtable offset.
    cmap_subtable_offset: usize,
    /// Format of selected cmap subtable (4, 12, or 0).
    cmap_format: u16,
}

impl TrueTypeFont {
    /// Parses a TrueType font from an in-memory byte slice.
    pub fn from_bytes(data: &[u8]) -> Result<Self, TtfError> {
        let mut reader = TtfReader::new(data);

        // 1. Parse SFNT Table Directory
        let sfnt_version = reader.read_u32()?;
        if sfnt_version != 0x00010000 && sfnt_version != 0x74727565 && sfnt_version != 0x4F54544F {
            // 0x00010000: Standard TrueType
            // 0x74727565: 'true' (Apple TrueType)
            // 0x4F54544F: 'OTTO' (OpenType)
            let bytes = sfnt_version.to_be_bytes();
            return Err(TtfError::InvalidMagic(bytes));
        }

        let num_tables = reader.read_u16()?;
        let _search_range = reader.read_u16()?;
        let _entry_selector = reader.read_u16()?;
        let _range_shift = reader.read_u16()?;

        let mut tables = BTreeMap::new();
        for _ in 0..num_tables {
            let tag_bytes = reader.read_bytes(4)?;
            let mut tag = [0u8; 4];
            tag.copy_from_slice(tag_bytes);

            let _checksum = reader.read_u32()?;
            let offset = reader.read_u32()? as usize;
            let length = reader.read_u32()? as usize;

            tables.insert(tag, (offset, length));
        }

        // 2. Parse `head` Table
        let (head_offset, head_len) = tables
            .get(b"head")
            .copied()
            .ok_or(TtfError::TableNotFound("head"))?;
        if head_len < 54 {
            return Err(TtfError::InvalidFormat("head table too short"));
        }
        let mut head_reader = reader.sub_reader(head_offset, head_len)?;
        head_reader.skip(12)?; // version (4) + revision (4) + checksumAdj (4)
        let magic = head_reader.read_u32()?;
        if magic != 0x5F0F3CF5 {
            return Err(TtfError::InvalidFormat("Invalid magic number in head table"));
        }
        head_reader.skip(2)?; // flags
        let units_per_em = head_reader.read_u16()?;
        head_reader.skip(16)?; // created (8) + modified (8)
        let x_min = head_reader.read_i16()?;
        let y_min = head_reader.read_i16()?;
        let x_max = head_reader.read_i16()?;
        let y_max = head_reader.read_i16()?;
        head_reader.skip(6)?; // macStyle (2) + lowestRecPPEM (2) + fontDirectionHint (2)
        let index_to_loc_format = head_reader.read_i16()?;

        // 3. Parse `maxp` Table
        let (maxp_offset, maxp_len) = tables
            .get(b"maxp")
            .copied()
            .ok_or(TtfError::TableNotFound("maxp"))?;
        let mut maxp_reader = reader.sub_reader(maxp_offset, maxp_len)?;
        maxp_reader.skip(4)?; // version
        let num_glyphs = maxp_reader.read_u16()?;

        // 4. Parse `hhea` Table
        let (hhea_offset, hhea_len) = tables
            .get(b"hhea")
            .copied()
            .ok_or(TtfError::TableNotFound("hhea"))?;
        let mut hhea_reader = reader.sub_reader(hhea_offset, hhea_len)?;
        hhea_reader.skip(4)?; // version
        let ascender = hhea_reader.read_i16()?;
        let descender = hhea_reader.read_i16()?;
        let line_gap = hhea_reader.read_i16()?;
        hhea_reader.skip(24)?; // advanceWidthMax, minLSB, minRSB, xMaxExtent, caret, reserved
        let number_of_hmetrics = hhea_reader.read_u16()?;

        // 5. Select Best `cmap` Subtable
        let (cmap_offset, cmap_len) = tables
            .get(b"cmap")
            .copied()
            .ok_or(TtfError::TableNotFound("cmap"))?;
        let mut cmap_reader = reader.sub_reader(cmap_offset, cmap_len)?;
        let _cmap_version = cmap_reader.read_u16()?;
        let num_subtables = cmap_reader.read_u16()?;

        let mut best_subtable_offset = 0;
        let mut best_format = 0;
        let mut best_score = -1;

        for _ in 0..num_subtables {
            let platform_id = cmap_reader.read_u16()?;
            let encoding_id = cmap_reader.read_u16()?;
            let subtable_offset_rel = cmap_reader.read_u32()? as usize;

            let abs_offset = cmap_offset + subtable_offset_rel;
            if abs_offset + 2 <= data.len() {
                let fmt = reader.peek_u16_at(abs_offset).unwrap_or(0);
                // Preference: Format 12 (Unicode Full 32-bit) > Format 4 (Unicode BMP) > Format 0
                let score = match (platform_id, encoding_id, fmt) {
                    (0, _, 12) | (3, 10, 12) => 100,
                    (0, _, 4) | (3, 1, 4) => 80,
                    (3, 0, 4) => 60,
                    (_, _, 4) => 40,
                    (_, _, 12) => 30,
                    (_, _, 0) => 10,
                    _ => 0,
                };

                if score > best_score {
                    best_score = score;
                    best_subtable_offset = abs_offset;
                    best_format = fmt;
                }
            }
        }

        // 6. Parse Font Names from `name` Table (if present)
        let mut family_name = String::from("TrueType Font");
        let mut style_name = String::from("Regular");

        if let Some(&(name_offset, name_len)) = tables.get(b"name") {
            if let Ok(mut name_reader) = reader.sub_reader(name_offset, name_len) {
                let _format = name_reader.read_u16().unwrap_or(0);
                let count = name_reader.read_u16().unwrap_or(0);
                let string_offset = name_reader.read_u16().unwrap_or(0) as usize;

                for _ in 0..count {
                    let platform_id = name_reader.read_u16().unwrap_or(0);
                    let _encoding_id = name_reader.read_u16().unwrap_or(0);
                    let language_id = name_reader.read_u16().unwrap_or(0);
                    let name_id = name_reader.read_u16().unwrap_or(0);
                    let length = name_reader.read_u16().unwrap_or(0) as usize;
                    let offset = name_reader.read_u16().unwrap_or(0) as usize;

                    let abs_str_offset = name_offset + string_offset + offset;
                    if abs_str_offset + length <= data.len() {
                        let bytes = &data[abs_str_offset..abs_str_offset + length];
                        let parsed_str = if platform_id == 0 || platform_id == 3 {
                            // UTF-16BE encoded
                            let u16_chars: Vec<u16> = bytes
                                .chunks_exact(2)
                                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                                .collect();
                            String::from_utf16_lossy(&u16_chars)
                        } else {
                            String::from_utf8_lossy(bytes).to_string()
                        };

                        if name_id == 1 && (family_name == "TrueType Font" || language_id == 0x0409) {
                            family_name = parsed_str.trim().to_string();
                        } else if name_id == 2 && (style_name == "Regular" || language_id == 0x0409) {
                            style_name = parsed_str.trim().to_string();
                        }
                    }
                }
            }
        }

        Ok(Self {
            data: data.to_vec(),
            tables,
            units_per_em,
            index_to_loc_format,
            num_glyphs,
            ascender,
            descender,
            line_gap,
            number_of_hmetrics,
            bbox: (x_min, y_min, x_max, y_max),
            family_name,
            style_name,
            cmap_subtable_offset: best_subtable_offset,
            cmap_format: best_format,
        })
    }

    /// Loads a TrueType font file from disk.
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, TtfError> {
        let bytes = std::fs::read(&path)
            .map_err(|e| TtfError::IoError(format!("{}: {}", path.as_ref().display(), e)))?;
        Self::from_bytes(&bytes)
    }

    /// Maps a Unicode character to its glyph index in this font using the active `cmap` subtable.
    pub fn char_to_glyph_id(&self, ch: char) -> Option<u16> {
        if self.cmap_subtable_offset == 0 {
            return None;
        }

        let code = ch as u32;

        match self.cmap_format {
            4 => self.lookup_cmap_format_4(code as u16),
            12 => self.lookup_cmap_format_12(code),
            0 => {
                if code < 256 {
                    let offset = self.cmap_subtable_offset + 6 + code as usize;
                    if offset < self.data.len() {
                        let gid = self.data[offset] as u16;
                        if gid > 0 { Some(gid) } else { None }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Looks up a character code in a Format 4 `cmap` subtable (standard 16-bit BMP mapping).
    fn lookup_cmap_format_4(&self, code: u16) -> Option<u16> {
        let sub = &self.data[self.cmap_subtable_offset..];
        if sub.len() < 14 {
            return None;
        }

        let seg_count_x2 = u16::from_be_bytes([sub[6], sub[7]]) as usize;
        let seg_count = seg_count_x2 / 2;

        let end_code_offset = 14;
        let start_code_offset = end_code_offset + seg_count * 2 + 2; // +2 for reservedPad
        let id_delta_offset = start_code_offset + seg_count * 2;
        let id_range_offset = id_delta_offset + seg_count * 2;

        if sub.len() < id_range_offset + seg_count * 2 {
            return None;
        }

        for i in 0..seg_count {
            let end_code = u16::from_be_bytes([
                sub[end_code_offset + i * 2],
                sub[end_code_offset + i * 2 + 1],
            ]);

            if end_code >= code {
                let start_code = u16::from_be_bytes([
                    sub[start_code_offset + i * 2],
                    sub[start_code_offset + i * 2 + 1],
                ]);

                if code < start_code {
                    return None;
                }

                let id_delta = i16::from_be_bytes([
                    sub[id_delta_offset + i * 2],
                    sub[id_delta_offset + i * 2 + 1],
                ]);

                let id_range_val = u16::from_be_bytes([
                    sub[id_range_offset + i * 2],
                    sub[id_range_offset + i * 2 + 1],
                ]) as usize;

                if id_range_val == 0 {
                    let gid = (code as i32 + id_delta as i32) as u16;
                    return if gid > 0 && gid < self.num_glyphs { Some(gid) } else { None };
                } else {
                    let glyph_offset = id_range_offset + i * 2 + id_range_val + (code - start_code) as usize * 2;
                    if glyph_offset + 2 <= sub.len() {
                        let glyph_index = u16::from_be_bytes([sub[glyph_offset], sub[glyph_offset + 1]]);
                        if glyph_index != 0 {
                            let gid = (glyph_index as i32 + id_delta as i32) as u16;
                            return if gid > 0 && gid < self.num_glyphs { Some(gid) } else { None };
                        }
                    }
                    return None;
                }
            }
        }

        None
    }

    /// Looks up a character code in a Format 12 `cmap` subtable (full 32-bit Unicode coverage).
    fn lookup_cmap_format_12(&self, code: u32) -> Option<u16> {
        let sub = &self.data[self.cmap_subtable_offset..];
        if sub.len() < 16 {
            return None;
        }

        let num_groups = u32::from_be_bytes([sub[12], sub[13], sub[14], sub[15]]) as usize;
        let groups_offset = 16;

        if sub.len() < groups_offset + num_groups * 12 {
            return None;
        }

        let mut low = 0;
        let mut high = num_groups;

        while low < high {
            let mid = low + (high - low) / 2;
            let offset = groups_offset + mid * 12;

            let start_char = u32::from_be_bytes([sub[offset], sub[offset + 1], sub[offset + 2], sub[offset + 3]]);
            let end_char = u32::from_be_bytes([sub[offset + 4], sub[offset + 5], sub[offset + 6], sub[offset + 7]]);
            let start_glyph = u32::from_be_bytes([sub[offset + 8], sub[offset + 9], sub[offset + 10], sub[offset + 11]]);

            if code < start_char {
                high = mid;
            } else if code > end_char {
                low = mid + 1;
            } else {
                let gid = (start_glyph + (code - start_char)) as u16;
                return if gid < self.num_glyphs { Some(gid) } else { None };
            }
        }

        None
    }

    /// Looks up horizontal metrics `(advance_width, left_side_bearing)` for a glyph ID.
    pub fn get_glyph_metrics(&self, glyph_id: u16) -> GlyphMetrics {
        let (hmtx_offset, hmtx_len) = match self.tables.get(b"hmtx") {
            Some(&t) => t,
            None => return GlyphMetrics { advance_width: self.units_per_em / 2, left_side_bearing: 0 },
        };

        let num_hmetrics = self.number_of_hmetrics as usize;
        let gid = glyph_id as usize;

        if gid < num_hmetrics {
            let offset = hmtx_offset + gid * 4;
            if offset + 4 <= self.data.len() && offset + 4 <= hmtx_offset + hmtx_len {
                let adv = u16::from_be_bytes([self.data[offset], self.data[offset + 1]]);
                let lsb = i16::from_be_bytes([self.data[offset + 2], self.data[offset + 3]]);
                return GlyphMetrics { advance_width: adv, left_side_bearing: lsb };
            }
        } else if num_hmetrics > 0 {
            // Glyphs beyond number_of_hmetrics share the last advance_width
            let last_adv_offset = hmtx_offset + (num_hmetrics - 1) * 4;
            let lsb_offset = hmtx_offset + num_hmetrics * 4 + (gid - num_hmetrics) * 2;

            let adv = if last_adv_offset + 2 <= self.data.len() {
                u16::from_be_bytes([self.data[last_adv_offset], self.data[last_adv_offset + 1]])
            } else {
                self.units_per_em / 2
            };

            let lsb = if lsb_offset + 2 <= self.data.len() && lsb_offset + 2 <= hmtx_offset + hmtx_len {
                i16::from_be_bytes([self.data[lsb_offset], self.data[lsb_offset + 1]])
            } else {
                0
            };

            return GlyphMetrics { advance_width: adv, left_side_bearing: lsb };
        }

        GlyphMetrics {
            advance_width: self.units_per_em / 2,
            left_side_bearing: 0,
        }
    }

    /// Obtains byte range `(offset, length)` of a glyph within the `glyf` table using `loca`.
    pub fn get_glyph_offset(&self, glyph_id: u16) -> Result<(usize, usize), TtfError> {
        if glyph_id >= self.num_glyphs {
            return Err(TtfError::InvalidGlyphIndex(glyph_id));
        }

        let (loca_offset, loca_len) = self
            .tables
            .get(b"loca")
            .copied()
            .ok_or(TtfError::TableNotFound("loca"))?;
        let (glyf_offset, glyf_len) = self
            .tables
            .get(b"glyf")
            .copied()
            .ok_or(TtfError::TableNotFound("glyf"))?;

        let gid = glyph_id as usize;

        let (start, end) = if self.index_to_loc_format == 0 {
            // Short 16-bit format: offset divided by 2
            let entry_offset = loca_offset + gid * 2;
            if entry_offset + 4 > loca_offset + loca_len || entry_offset + 4 > self.data.len() {
                return Err(TtfError::UnexpectedEof);
            }
            let s = (u16::from_be_bytes([self.data[entry_offset], self.data[entry_offset + 1]]) as usize) * 2;
            let e = (u16::from_be_bytes([self.data[entry_offset + 2], self.data[entry_offset + 3]]) as usize) * 2;
            (s, e)
        } else {
            // Long 32-bit format: actual offset
            let entry_offset = loca_offset + gid * 4;
            if entry_offset + 8 > loca_offset + loca_len || entry_offset + 8 > self.data.len() {
                return Err(TtfError::UnexpectedEof);
            }
            let s = u32::from_be_bytes([
                self.data[entry_offset],
                self.data[entry_offset + 1],
                self.data[entry_offset + 2],
                self.data[entry_offset + 3],
            ]) as usize;
            let e = u32::from_be_bytes([
                self.data[entry_offset + 4],
                self.data[entry_offset + 5],
                self.data[entry_offset + 6],
                self.data[entry_offset + 7],
            ]) as usize;
            (s, e)
        };

        if start >= end {
            // Empty glyph (e.g. space character)
            return Ok((glyf_offset + start, 0));
        }

        if start > glyf_len || end > glyf_len {
            return Err(TtfError::InvalidFormat("loca offset exceeds glyf table length"));
        }

        Ok((glyf_offset + start, end - start))
    }

    /// Extracts vector outline contours for a glyph ID.
    pub fn get_glyph_outline(&self, glyph_id: u16) -> Result<GlyphOutline, TtfError> {
        self.get_glyph_outline_depth(glyph_id, 0)
    }

    /// Recursive outline extraction supporting nested composite glyphs up to 16 levels.
    fn get_glyph_outline_depth(&self, glyph_id: u16, depth: usize) -> Result<GlyphOutline, TtfError> {
        if depth >= 16 {
            return Err(TtfError::InvalidFormat("Exceeded maximum composite glyph recursion depth"));
        }

        let (offset, length) = self.get_glyph_offset(glyph_id)?;
        if length == 0 {
            return Ok(GlyphOutline::empty());
        }

        let mut reader = TtfReader::new(&self.data);
        reader.seek(offset)?;

        let number_of_contours = reader.read_i16()?;
        let x_min = reader.read_i16()?;
        let y_min = reader.read_i16()?;
        let x_max = reader.read_i16()?;
        let y_max = reader.read_i16()?;

        if number_of_contours > 0 {
            // Simple Glyph
            self.parse_simple_glyph(&mut reader, number_of_contours as usize, x_min, y_min, x_max, y_max)
        } else if number_of_contours < 0 {
            // Composite Glyph
            self.parse_composite_glyph(&mut reader, depth, x_min, y_min, x_max, y_max)
        } else {
            Ok(GlyphOutline::empty())
        }
    }

    /// Parses a simple TrueType glyph (`number_of_contours > 0`).
    fn parse_simple_glyph(
        &self,
        reader: &mut TtfReader<'_>,
        num_contours: usize,
        x_min: i16,
        y_min: i16,
        x_max: i16,
        y_max: i16,
    ) -> Result<GlyphOutline, TtfError> {
        let mut end_pts = Vec::with_capacity(num_contours);
        for _ in 0..num_contours {
            end_pts.push(reader.read_u16()?);
        }

        let total_points = (*end_pts.last().unwrap_or(&0) as usize) + 1;
        let instruction_length = reader.read_u16()? as usize;
        reader.skip(instruction_length)?;

        // Flags decoding with RLE repeat flag
        let mut flags = Vec::with_capacity(total_points);
        while flags.len() < total_points {
            let flag = reader.read_u8()?;
            flags.push(flag);

            if (flag & 0x08) != 0 {
                // Repeat flag: repeat this flag byte `count` times
                let count = reader.read_u8()? as usize;
                for _ in 0..count {
                    if flags.len() < total_points {
                        flags.push(flag);
                    }
                }
            }
        }

        // X-coordinates decoding
        let mut x_coords = Vec::with_capacity(total_points);
        let mut cur_x = 0i32;
        for &flag in &flags {
            let is_short = (flag & 0x02) != 0;
            let same_or_pos = (flag & 0x10) != 0;

            let dx = if is_short {
                let val = reader.read_u8()? as i32;
                if same_or_pos { val } else { -val }
            } else if same_or_pos {
                0
            } else {
                reader.read_i16()? as i32
            };

            cur_x += dx;
            x_coords.push(cur_x);
        }

        // Y-coordinates decoding
        let mut y_coords = Vec::with_capacity(total_points);
        let mut cur_y = 0i32;
        for &flag in &flags {
            let is_short = (flag & 0x04) != 0;
            let same_or_pos = (flag & 0x20) != 0;

            let dy = if is_short {
                let val = reader.read_u8()? as i32;
                if same_or_pos { val } else { -val }
            } else if same_or_pos {
                0
            } else {
                reader.read_i16()? as i32
            };

            cur_y += dy;
            y_coords.push(cur_y);
        }

        // Build closed contours
        let mut contours = Vec::with_capacity(num_contours);
        for c in 0..num_contours {
            let start = if c == 0 { 0 } else { (end_pts[c - 1] as usize) + 1 };
            let end = end_pts[c] as usize;

            if start > end || end >= total_points {
                continue;
            }

            let mut points = Vec::with_capacity(end - start + 1);
            for p in start..=end {
                let on_curve = (flags[p] & 0x01) != 0;
                points.push(OutlinePoint {
                    x: x_coords[p] as f32,
                    y: y_coords[p] as f32,
                    on_curve,
                });
            }

            contours.push(Contour::with_points(points));
        }

        Ok(GlyphOutline {
            contours,
            x_min,
            y_min,
            x_max,
            y_max,
        })
    }

    /// Parses a composite TrueType glyph (`number_of_contours == -1`).
    fn parse_composite_glyph(
        &self,
        reader: &mut TtfReader<'_>,
        depth: usize,
        x_min: i16,
        y_min: i16,
        x_max: i16,
        y_max: i16,
    ) -> Result<GlyphOutline, TtfError> {
        let mut combined_contours = Vec::new();
        let mut more_components = true;

        while more_components {
            let flags = reader.read_u16()?;
            let component_glyph_id = reader.read_u16()?;

            let arg1_and_2_are_words = (flags & 0x0001) != 0;
            let args_are_xy_values = (flags & 0x0002) != 0;
            let we_have_a_scale = (flags & 0x0008) != 0;
            let we_have_an_x_and_y_scale = (flags & 0x0040) != 0;
            let we_have_a_two_by_two = (flags & 0x0080) != 0;
            more_components = (flags & 0x0020) != 0;

            let (dx, dy) = if args_are_xy_values {
                if arg1_and_2_are_words {
                    (reader.read_i16()? as f32, reader.read_i16()? as f32)
                } else {
                    (reader.read_i8()? as f32, reader.read_i8()? as f32)
                }
            } else {
                if arg1_and_2_are_words {
                    let _pt1 = reader.read_u16()?;
                    let _pt2 = reader.read_u16()?;
                } else {
                    let _pt1 = reader.read_u8()?;
                    let _pt2 = reader.read_u8()?;
                }
                (0.0, 0.0)
            };

            let (mut a, mut b, mut c, mut d) = (1.0f32, 0.0f32, 0.0f32, 1.0f32);
            if we_have_a_scale {
                let s = reader.read_f2dot14()?;
                a = s;
                d = s;
            } else if we_have_an_x_and_y_scale {
                a = reader.read_f2dot14()?;
                d = reader.read_f2dot14()?;
            } else if we_have_a_two_by_two {
                a = reader.read_f2dot14()?;
                b = reader.read_f2dot14()?;
                c = reader.read_f2dot14()?;
                d = reader.read_f2dot14()?;
            }

            // Recursively fetch component outline and apply affine transformation
            let component_outline = self.get_glyph_outline_depth(component_glyph_id, depth + 1)?;

            for contour in component_outline.contours {
                let mut transformed_points = Vec::with_capacity(contour.points.len());
                for pt in contour.points {
                    // Affine transformation:
                    // x' = a * x + c * y + dx
                    // y' = b * x + d * y + dy
                    let tx = a * pt.x + c * pt.y + dx;
                    let ty = b * pt.x + d * pt.y + dy;
                    transformed_points.push(OutlinePoint {
                        x: tx,
                        y: ty,
                        on_curve: pt.on_curve,
                    });
                }
                combined_contours.push(Contour::with_points(transformed_points));
            }
        }

        Ok(GlyphOutline {
            contours: combined_contours,
            x_min,
            y_min,
            x_max,
            y_max,
        })
    }
}
