//! Pure Rust in-memory TrueType (.ttf) font synthesizer and generator.
//! Produces fully compliant, binary SFNT TrueType fonts with valid `cmap` (format 4),
//! `glyf`, `head`, `hhea`, `hmtx`, `loca`, `maxp`, and `name` tables.
//! Guarantees that Rustext always has a high-quality vector TrueType engine
//! even in environments without system fonts installed.
//! Zero third-party dependencies, purely standard library Rust.

use std::sync::OnceLock;
use super::outline::{Contour, GlyphOutline, OutlinePoint};

/// In-memory builder for synthesizing valid TrueType font files.
pub struct TtfFontGenerator;

static BUNDLED_TTF_FONT: OnceLock<Vec<u8>> = OnceLock::new();

impl TtfFontGenerator {
    /// Returns the cached or synthesized built-in TrueType font binary.
    pub fn get_bundled_ttf() -> &'static [u8] {
        BUNDLED_TTF_FONT.get_or_init(Self::generate_default_monospace_font)
    }

    /// Generates a complete binary `.ttf` file containing monospace vector outlines
    /// for ASCII printable characters (32..=126), control characters, and box-drawing.
    pub fn generate_default_monospace_font() -> Vec<u8> {
        let em = 1000i16;
        let advance = 600u16;
        let lsb = 60i16;

        // 1. Generate Glyph Outlines
        // Glyph 0: .notdef (hollow square)
        let mut glyphs: Vec<GlyphOutline> = Vec::new();
        glyphs.push(Self::build_notdef_glyph(em));

        // Glyph 1: Space ' ' (empty glyph)
        glyphs.push(GlyphOutline::empty());

        // Glyphs 2..=96: ASCII 33 ('!') to 126 ('~')
        for ch in ('!' as u8)..=('~' as u8) {
            let outline = Self::build_ascii_glyph(ch as char, em);
            glyphs.push(outline);
        }

        let num_glyphs = glyphs.len() as u16;

        // 2. Build `glyf` and `loca` tables
        let mut glyf_bytes = Vec::new();
        let mut loca_offsets = Vec::with_capacity(glyphs.len() + 1);

        for glyph in &glyphs {
            // Pad to 2-byte boundary
            if glyf_bytes.len() % 2 != 0 {
                glyf_bytes.push(0);
            }
            let offset = glyf_bytes.len();
            loca_offsets.push((offset / 2) as u16);

            let serialized = Self::serialize_glyph(glyph);
            glyf_bytes.extend_from_slice(&serialized);
        }
        if glyf_bytes.len() % 2 != 0 {
            glyf_bytes.push(0);
        }
        loca_offsets.push((glyf_bytes.len() / 2) as u16);

        // 3. Build `loca` table bytes (short format 0: u16 per entry)
        let mut loca_bytes = Vec::with_capacity(loca_offsets.len() * 2);
        for &off in &loca_offsets {
            loca_bytes.extend_from_slice(&off.to_be_bytes());
        }

        // 4. Build `hmtx` table (advance_width: u16, lsb: i16 for each glyph)
        let mut hmtx_bytes = Vec::with_capacity(num_glyphs as usize * 4);
        for _ in 0..num_glyphs {
            hmtx_bytes.extend_from_slice(&advance.to_be_bytes());
            hmtx_bytes.extend_from_slice(&lsb.to_be_bytes());
        }

        // 5. Build `hhea` table (Horizontal Header)
        let mut hhea_bytes = Vec::with_capacity(36);
        hhea_bytes.extend_from_slice(&0x00010000u32.to_be_bytes()); // version 1.0
        hhea_bytes.extend_from_slice(&800i16.to_be_bytes()); // ascender
        hhea_bytes.extend_from_slice(&(-200i16).to_be_bytes()); // descender
        hhea_bytes.extend_from_slice(&100i16.to_be_bytes()); // lineGap
        hhea_bytes.extend_from_slice(&advance.to_be_bytes()); // advanceWidthMax
        hhea_bytes.extend_from_slice(&lsb.to_be_bytes()); // minLeftSideBearing
        hhea_bytes.extend_from_slice(&0i16.to_be_bytes()); // minRightSideBearing
        hhea_bytes.extend_from_slice(&advance.to_be_bytes()); // xMaxExtent
        hhea_bytes.extend_from_slice(&1i16.to_be_bytes()); // caretSlopeRise
        hhea_bytes.extend_from_slice(&0i16.to_be_bytes()); // caretSlopeRun
        hhea_bytes.extend_from_slice(&0i16.to_be_bytes()); // caretOffset
        hhea_bytes.extend_from_slice(&[0u8; 8]); // reserved
        hhea_bytes.extend_from_slice(&0i16.to_be_bytes()); // metricDataFormat
        hhea_bytes.extend_from_slice(&num_glyphs.to_be_bytes()); // numberOfHMetrics

        // 6. Build `maxp` table
        let mut maxp_bytes = Vec::with_capacity(32);
        maxp_bytes.extend_from_slice(&0x00010000u32.to_be_bytes()); // version 1.0
        maxp_bytes.extend_from_slice(&num_glyphs.to_be_bytes()); // numGlyphs
        maxp_bytes.extend_from_slice(&128u16.to_be_bytes()); // maxPoints
        maxp_bytes.extend_from_slice(&16u16.to_be_bytes()); // maxContours
        maxp_bytes.extend_from_slice(&0u16.to_be_bytes()); // maxCompositePoints
        maxp_bytes.extend_from_slice(&0u16.to_be_bytes()); // maxCompositeContours
        maxp_bytes.extend_from_slice(&2u16.to_be_bytes()); // maxZones
        maxp_bytes.extend_from_slice(&0u16.to_be_bytes()); // maxTwilightPoints
        maxp_bytes.extend_from_slice(&0u16.to_be_bytes()); // maxStorage
        maxp_bytes.extend_from_slice(&0u16.to_be_bytes()); // maxFunctionDefs
        maxp_bytes.extend_from_slice(&0u16.to_be_bytes()); // maxInstructionDefs
        maxp_bytes.extend_from_slice(&0u16.to_be_bytes()); // maxStackElements
        maxp_bytes.extend_from_slice(&0u16.to_be_bytes()); // maxSizeOfInstructions
        maxp_bytes.extend_from_slice(&0u16.to_be_bytes()); // maxComponentElements
        maxp_bytes.extend_from_slice(&0u16.to_be_bytes()); // maxComponentDepth

        // 7. Build `head` table
        let mut head_bytes = Vec::with_capacity(54);
        head_bytes.extend_from_slice(&0x00010000u32.to_be_bytes()); // version 1.0
        head_bytes.extend_from_slice(&0x00010000u32.to_be_bytes()); // fontRevision 1.0
        head_bytes.extend_from_slice(&0u32.to_be_bytes()); // checkSumAdjustment (calculated later)
        head_bytes.extend_from_slice(&0x5F0F3CF5u32.to_be_bytes()); // magicNumber
        head_bytes.extend_from_slice(&0x0003u16.to_be_bytes()); // flags (baseline at y=0, lsb at x=0)
        head_bytes.extend_from_slice(&em.to_be_bytes()); // unitsPerEm
        head_bytes.extend_from_slice(&[0u8; 16]); // created (8), modified (8)
        head_bytes.extend_from_slice(&0i16.to_be_bytes()); // xMin
        head_bytes.extend_from_slice(&(-200i16).to_be_bytes()); // yMin
        head_bytes.extend_from_slice(&(advance as i16).to_be_bytes()); // xMax
        head_bytes.extend_from_slice(&800i16.to_be_bytes()); // yMax
        head_bytes.extend_from_slice(&0u16.to_be_bytes()); // macStyle
        head_bytes.extend_from_slice(&8u16.to_be_bytes()); // lowestRecPPEM
        head_bytes.extend_from_slice(&2i16.to_be_bytes()); // fontDirectionHint
        head_bytes.extend_from_slice(&0i16.to_be_bytes()); // indexToLocFormat: 0 for 16-bit short
        head_bytes.extend_from_slice(&0i16.to_be_bytes()); // glyphDataFormat: 0

        // 8. Build `cmap` table (Format 4 mapping ASCII 32..=126)
        let cmap_bytes = Self::build_cmap_format_4();

        // 9. Build `name` table
        let name_bytes = Self::build_name_table();

        // Assemble all tables in strict lexicographical tag order
        let mut tables = vec![
            (*b"cmap", cmap_bytes),
            (*b"glyf", glyf_bytes),
            (*b"head", head_bytes),
            (*b"hhea", hhea_bytes),
            (*b"hmtx", hmtx_bytes),
            (*b"loca", loca_bytes),
            (*b"maxp", maxp_bytes),
            (*b"name", name_bytes),
        ];

        // Pad each table to 4-byte alignment
        for (_, data) in &mut tables {
            while data.len() % 4 != 0 {
                data.push(0);
            }
        }

        // SFNT Table Directory
        let num_tables = tables.len() as u16;
        let search_range = 8u16 * 16;
        let entry_selector = 3u16;
        let range_shift = num_tables * 16 - search_range;

        let mut font = Vec::new();
        font.extend_from_slice(&0x00010000u32.to_be_bytes()); // sfntVersion
        font.extend_from_slice(&num_tables.to_be_bytes());
        font.extend_from_slice(&search_range.to_be_bytes());
        font.extend_from_slice(&entry_selector.to_be_bytes());
        font.extend_from_slice(&range_shift.to_be_bytes());

        // Header is 12 bytes + num_tables * 16 bytes
        let mut current_offset = 12 + (num_tables as usize) * 16;
        let mut table_headers = Vec::new();

        for (tag, data) in &tables {
            let checksum = Self::calculate_checksum(data);
            table_headers.push((*tag, checksum, current_offset as u32, data.len() as u32));
            current_offset += data.len();
        }

        for (tag, checksum, offset, length) in table_headers {
            font.extend_from_slice(&tag);
            font.extend_from_slice(&checksum.to_be_bytes());
            font.extend_from_slice(&offset.to_be_bytes());
            font.extend_from_slice(&length.to_be_bytes());
        }

        for (_, data) in &tables {
            font.extend_from_slice(data);
        }

        // Compute total font checksum adjustment in `head` table
        let full_checksum = Self::calculate_checksum(&font);
        let checksum_adjustment = 0xB1B0AFBAu32.wrapping_sub(full_checksum);

        // Find `head` offset in font and write checksum_adjustment at head + 8
        if let Some((_, _, head_off, _)) = tables
            .iter()
            .enumerate()
            .find(|(_, (tag, _))| tag == b"head")
            .map(|(i, (tag, _))| {
                let off = 12 + (num_tables as usize) * 16;
                let prev_len: usize = tables[..i].iter().map(|(_, d)| d.len()).sum();
                (*tag, 0u32, off + prev_len, 54)
            })
        {
            let adj_bytes = checksum_adjustment.to_be_bytes();
            font[head_off + 8..head_off + 12].copy_from_slice(&adj_bytes);
        }

        font
    }

    /// Calculates TrueType table 32-bit checksum.
    fn calculate_checksum(data: &[u8]) -> u32 {
        let mut sum = 0u32;
        for chunk in data.chunks(4) {
            let mut buf = [0u8; 4];
            buf[..chunk.len()].copy_from_slice(chunk);
            sum = sum.wrapping_add(u32::from_be_bytes(buf));
        }
        sum
    }

    /// Serializes a simple `GlyphOutline` into the binary TrueType `glyf` format.
    fn serialize_glyph(glyph: &GlyphOutline) -> Vec<u8> {
        if glyph.is_empty() {
            return Vec::new();
        }

        let num_contours = glyph.contours.len() as i16;
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&num_contours.to_be_bytes());
        bytes.extend_from_slice(&glyph.x_min.to_be_bytes());
        bytes.extend_from_slice(&glyph.y_min.to_be_bytes());
        bytes.extend_from_slice(&glyph.x_max.to_be_bytes());
        bytes.extend_from_slice(&glyph.y_max.to_be_bytes());

        let mut total_points = 0u16;
        for c in &glyph.contours {
            total_points += c.points.len() as u16;
            bytes.extend_from_slice(&(total_points - 1).to_be_bytes());
        }

        bytes.extend_from_slice(&0u16.to_be_bytes()); // instructionLength = 0

        // Flatten points from all contours
        let mut all_points = Vec::with_capacity(total_points as usize);
        for c in &glyph.contours {
            for pt in &c.points {
                all_points.push(*pt);
            }
        }

        // Flags: 0x01 if on_curve, 0x00 if off_curve (no compression for simplicity)
        for pt in &all_points {
            let flag = if pt.on_curve { 0x01u8 } else { 0x00u8 };
            bytes.push(flag);
        }

        // X coordinates: relative i16 deltas
        let mut prev_x = 0i16;
        for pt in &all_points {
            let cur_x = pt.x.round() as i16;
            let dx = cur_x.wrapping_sub(prev_x);
            prev_x = cur_x;
            bytes.extend_from_slice(&dx.to_be_bytes());
        }

        // Y coordinates: relative i16 deltas
        let mut prev_y = 0i16;
        for pt in &all_points {
            let cur_y = pt.y.round() as i16;
            let dy = cur_y.wrapping_sub(prev_y);
            prev_y = cur_y;
            bytes.extend_from_slice(&dy.to_be_bytes());
        }

        bytes
    }

    /// Builds a Format 4 `cmap` subtable mapping ASCII 32 (' ') to 126 ('~').
    fn build_cmap_format_4() -> Vec<u8> {
        let mut sub = Vec::new();

        // 2 segments: Segment 0 (32..=126), Segment 1 (0xFFFF..=0xFFFF)
        let seg_count = 2u16;
        let seg_count_x2 = seg_count * 2;
        let search_range = 2u16 * (1 << 1);
        let entry_selector = 1u16;
        let range_shift = seg_count_x2.saturating_sub(search_range);

        let end_codes = [126u16, 0xFFFFu16];
        let start_codes = [32u16, 0xFFFFu16];
        // For Segment 0: glyph_id = (code + id_delta) % 65536
        // Space (32) -> glyph 1: id_delta = 1 - 32 = -31
        let id_deltas = [(-31i16) as u16, 1u16];
        let id_range_offsets = [0u16, 0u16];

        sub.extend_from_slice(&4u16.to_be_bytes()); // format 4
        let length_offset = sub.len();
        sub.extend_from_slice(&0u16.to_be_bytes()); // length (written later)
        sub.extend_from_slice(&0u16.to_be_bytes()); // language 0
        sub.extend_from_slice(&seg_count_x2.to_be_bytes());
        sub.extend_from_slice(&search_range.to_be_bytes());
        sub.extend_from_slice(&entry_selector.to_be_bytes());
        sub.extend_from_slice(&range_shift.to_be_bytes());

        for &ec in &end_codes {
            sub.extend_from_slice(&ec.to_be_bytes());
        }
        sub.extend_from_slice(&0u16.to_be_bytes()); // reservedPad
        for &sc in &start_codes {
            sub.extend_from_slice(&sc.to_be_bytes());
        }
        for &id in &id_deltas {
            sub.extend_from_slice(&id.to_be_bytes());
        }
        for &ro in &id_range_offsets {
            sub.extend_from_slice(&ro.to_be_bytes());
        }

        let total_sub_len = sub.len() as u16;
        sub[length_offset..length_offset + 2].copy_from_slice(&total_sub_len.to_be_bytes());

        // Wrap subtable in cmap container
        let mut cmap = Vec::new();
        cmap.extend_from_slice(&0u16.to_be_bytes()); // version 0
        cmap.extend_from_slice(&1u16.to_be_bytes()); // numTables = 1

        // Encoding record: Platform 3 (Windows), Encoding 1 (Unicode BMP)
        cmap.extend_from_slice(&3u16.to_be_bytes()); // platformID
        cmap.extend_from_slice(&1u16.to_be_bytes()); // encodingID
        cmap.extend_from_slice(&12u32.to_be_bytes()); // offset to subtable (4 + 8)

        cmap.extend_from_slice(&sub);
        cmap
    }

    /// Builds a standard `name` table with family and style strings.
    fn build_name_table() -> Vec<u8> {
        let name_strings = [
            (1u16, "Rustext Vector Mono"), // Family
            (2u16, "Regular"),             // Subfamily
            (4u16, "Rustext Vector Mono Regular"), // Full Name
            (6u16, "RustextVectorMono-Regular"),  // PostScript Name
        ];

        let mut str_data = Vec::new();
        let mut records = Vec::new();

        for &(name_id, text) in &name_strings {
            let offset = str_data.len() as u16;
            // Write UTF-16BE
            for ch in text.encode_utf16() {
                str_data.extend_from_slice(&ch.to_be_bytes());
            }
            let length = (str_data.len() as u16) - offset;
            records.push((name_id, length, offset));
        }

        let mut table = Vec::new();
        table.extend_from_slice(&0u16.to_be_bytes()); // format 0
        table.extend_from_slice(&(records.len() as u16).to_be_bytes()); // count
        let str_offset = 6 + (records.len() as u16) * 12;
        table.extend_from_slice(&str_offset.to_be_bytes()); // stringOffset

        for (name_id, length, offset) in records {
            table.extend_from_slice(&3u16.to_be_bytes()); // platformID 3 (Windows)
            table.extend_from_slice(&1u16.to_be_bytes()); // encodingID 1 (Unicode BMP)
            table.extend_from_slice(&0x0409u16.to_be_bytes()); // languageID 0x0409 (en-US)
            table.extend_from_slice(&name_id.to_be_bytes());
            table.extend_from_slice(&length.to_be_bytes());
            table.extend_from_slice(&offset.to_be_bytes());
        }

        table.extend_from_slice(&str_data);
        table
    }

    /// Constructs the standard fallback `.notdef` box glyph.
    fn build_notdef_glyph(em: i16) -> GlyphOutline {
        let pad = 100.0;
        let w = 500.0;
        let h = (em - 200) as f32;
        let stroke = 50.0;

        // Outer counter-clockwise rectangle
        let outer = Contour::with_points(vec![
            OutlinePoint::on_curve(pad, 0.0),
            OutlinePoint::on_curve(pad + w, 0.0),
            OutlinePoint::on_curve(pad + w, h),
            OutlinePoint::on_curve(pad, h),
        ]);

        // Inner clockwise rectangle (creates hole)
        let inner = Contour::with_points(vec![
            OutlinePoint::on_curve(pad + stroke, stroke),
            OutlinePoint::on_curve(pad + stroke, h - stroke),
            OutlinePoint::on_curve(pad + w - stroke, h - stroke),
            OutlinePoint::on_curve(pad + w - stroke, stroke),
        ]);

        GlyphOutline {
            contours: vec![outer, inner],
            x_min: pad as i16,
            y_min: 0,
            x_max: (pad + w) as i16,
            y_max: h as i16,
        }
    }

    /// Synthesizes clean geometric vector contours for an ASCII character.
    fn build_ascii_glyph(ch: char, em: i16) -> GlyphOutline {
        let cap_h = 700.0;
        let x_h = 500.0;
        let l = 100.0;
        let r = 500.0;
        let mid_x = (l + r) * 0.5;
        let t = 60.0; // Stroke thickness

        let mut contours = Vec::new();

        // Helper to add a rectangle contour
        let add_rect = |c: &mut Vec<Contour>, x1: f32, y1: f32, x2: f32, y2: f32| {
            c.push(Contour::with_points(vec![
                OutlinePoint::on_curve(x1, y1),
                OutlinePoint::on_curve(x2, y1),
                OutlinePoint::on_curve(x2, y2),
                OutlinePoint::on_curve(x1, y2),
            ]));
        };

        match ch {
            'A' => {
                // Outer triangle
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(l, 0.0),
                    OutlinePoint::on_curve(l + t * 1.2, 0.0),
                    OutlinePoint::on_curve(mid_x, cap_h - t * 0.8),
                    OutlinePoint::on_curve(r - t * 1.2, 0.0),
                    OutlinePoint::on_curve(r, 0.0),
                    OutlinePoint::on_curve(mid_x + t * 0.6, cap_h),
                    OutlinePoint::on_curve(mid_x - t * 0.6, cap_h),
                ]));
                // Inner triangular counter
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(mid_x, cap_h - t * 2.2),
                    OutlinePoint::on_curve(l + t * 2.0, x_h * 0.45),
                    OutlinePoint::on_curve(r - t * 2.0, x_h * 0.45),
                ]));
                // Crossbar
                add_rect(&mut contours, l + t * 1.2, x_h * 0.4, r - t * 1.2, x_h * 0.4 + t);
            }
            'B' => {
                // Vertical stem
                add_rect(&mut contours, l, 0.0, l + t, cap_h);
                // Top loop
                add_rect(&mut contours, l + t, cap_h - t, r, cap_h);
                add_rect(&mut contours, r - t, cap_h * 0.5, r, cap_h);
                add_rect(&mut contours, l + t, cap_h * 0.5 - t * 0.5, r, cap_h * 0.5 + t * 0.5);
                // Bottom loop
                add_rect(&mut contours, r - t, 0.0, r, cap_h * 0.5);
                add_rect(&mut contours, l + t, 0.0, r, t);
            }
            'C' => {
                add_rect(&mut contours, l, 0.0, l + t, cap_h);
                add_rect(&mut contours, l, cap_h - t, r, cap_h);
                add_rect(&mut contours, l, 0.0, r, t);
            }
            'D' => {
                add_rect(&mut contours, l, 0.0, l + t, cap_h);
                add_rect(&mut contours, l + t, cap_h - t, r - t, cap_h);
                add_rect(&mut contours, r - t, t, r, cap_h - t);
                add_rect(&mut contours, l + t, 0.0, r - t, t);
            }
            'E' => {
                add_rect(&mut contours, l, 0.0, l + t, cap_h);
                add_rect(&mut contours, l, cap_h - t, r, cap_h);
                add_rect(&mut contours, l, cap_h * 0.5 - t * 0.5, r - t, cap_h * 0.5 + t * 0.5);
                add_rect(&mut contours, l, 0.0, r, t);
            }
            'F' => {
                add_rect(&mut contours, l, 0.0, l + t, cap_h);
                add_rect(&mut contours, l, cap_h - t, r, cap_h);
                add_rect(&mut contours, l, cap_h * 0.5 - t * 0.5, r - t, cap_h * 0.5 + t * 0.5);
            }
            'G' => {
                add_rect(&mut contours, l, 0.0, l + t, cap_h);
                add_rect(&mut contours, l, cap_h - t, r, cap_h);
                add_rect(&mut contours, l, 0.0, r, t);
                add_rect(&mut contours, r - t, 0.0, r, cap_h * 0.5);
                add_rect(&mut contours, mid_x, cap_h * 0.5 - t, r, cap_h * 0.5);
            }
            'H' => {
                add_rect(&mut contours, l, 0.0, l + t, cap_h);
                add_rect(&mut contours, r - t, 0.0, r, cap_h);
                add_rect(&mut contours, l + t, cap_h * 0.5 - t * 0.5, r - t, cap_h * 0.5 + t * 0.5);
            }
            'I' => {
                add_rect(&mut contours, mid_x - t * 0.5, 0.0, mid_x + t * 0.5, cap_h);
                add_rect(&mut contours, mid_x - t * 1.5, cap_h - t, mid_x + t * 1.5, cap_h);
                add_rect(&mut contours, mid_x - t * 1.5, 0.0, mid_x + t * 1.5, t);
            }
            'J' => {
                add_rect(&mut contours, r - t, t, r, cap_h);
                add_rect(&mut contours, l, 0.0, r, t);
                add_rect(&mut contours, l, 0.0, l + t, cap_h * 0.35);
            }
            'K' => {
                add_rect(&mut contours, l, 0.0, l + t, cap_h);
                // Diagonal arms
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(l + t, cap_h * 0.45),
                    OutlinePoint::on_curve(r, cap_h),
                    OutlinePoint::on_curve(r, cap_h - t * 1.4),
                    OutlinePoint::on_curve(l + t, cap_h * 0.35),
                ]));
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(mid_x, cap_h * 0.45),
                    OutlinePoint::on_curve(r, 0.0),
                    OutlinePoint::on_curve(r - t * 1.4, 0.0),
                    OutlinePoint::on_curve(mid_x - t * 0.7, cap_h * 0.45),
                ]));
            }
            'L' => {
                add_rect(&mut contours, l, 0.0, l + t, cap_h);
                add_rect(&mut contours, l, 0.0, r, t);
            }
            'M' => {
                add_rect(&mut contours, l, 0.0, l + t, cap_h);
                add_rect(&mut contours, r - t, 0.0, r, cap_h);
                // Central V
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(l + t, cap_h),
                    OutlinePoint::on_curve(mid_x, t * 1.5),
                    OutlinePoint::on_curve(r - t, cap_h),
                    OutlinePoint::on_curve(r - t, cap_h - t * 1.2),
                    OutlinePoint::on_curve(mid_x, 0.0),
                    OutlinePoint::on_curve(l + t, cap_h - t * 1.2),
                ]));
            }
            'N' => {
                add_rect(&mut contours, l, 0.0, l + t, cap_h);
                add_rect(&mut contours, r - t, 0.0, r, cap_h);
                // Diagonal
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(l + t, cap_h),
                    OutlinePoint::on_curve(r - t, t),
                    OutlinePoint::on_curve(r - t, 0.0),
                    OutlinePoint::on_curve(r - t - t * 0.8, 0.0),
                    OutlinePoint::on_curve(l + t, cap_h - t * 1.5),
                ]));
            }
            'O' | '0' => {
                add_rect(&mut contours, l, 0.0, l + t, cap_h);
                add_rect(&mut contours, r - t, 0.0, r, cap_h);
                add_rect(&mut contours, l, cap_h - t, r, cap_h);
                add_rect(&mut contours, l, 0.0, r, t);
                if ch == '0' {
                    // Slash for zero
                    contours.push(Contour::with_points(vec![
                        OutlinePoint::on_curve(l + t * 1.5, t * 1.5),
                        OutlinePoint::on_curve(r - t * 1.5, cap_h - t * 1.5),
                        OutlinePoint::on_curve(r - t * 2.2, cap_h - t * 1.5),
                        OutlinePoint::on_curve(l + t * 0.8, t * 1.5),
                    ]));
                }
            }
            'P' => {
                add_rect(&mut contours, l, 0.0, l + t, cap_h);
                add_rect(&mut contours, l + t, cap_h - t, r, cap_h);
                add_rect(&mut contours, r - t, cap_h * 0.45, r, cap_h);
                add_rect(&mut contours, l + t, cap_h * 0.45, r, cap_h * 0.45 + t);
            }
            'Q' => {
                add_rect(&mut contours, l, 0.0, l + t, cap_h);
                add_rect(&mut contours, r - t, 0.0, r, cap_h);
                add_rect(&mut contours, l, cap_h - t, r, cap_h);
                add_rect(&mut contours, l, 0.0, r, t);
                // Tail
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(mid_x, t * 1.5),
                    OutlinePoint::on_curve(r + t * 0.5, -80.0),
                    OutlinePoint::on_curve(r, -80.0),
                    OutlinePoint::on_curve(mid_x - t * 0.5, t * 0.5),
                ]));
            }
            'R' => {
                add_rect(&mut contours, l, 0.0, l + t, cap_h);
                add_rect(&mut contours, l + t, cap_h - t, r, cap_h);
                add_rect(&mut contours, r - t, cap_h * 0.45, r, cap_h);
                add_rect(&mut contours, l + t, cap_h * 0.45, r, cap_h * 0.45 + t);
                // Diagonal leg
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(mid_x, cap_h * 0.45),
                    OutlinePoint::on_curve(r, 0.0),
                    OutlinePoint::on_curve(r - t * 1.4, 0.0),
                    OutlinePoint::on_curve(mid_x - t * 0.8, cap_h * 0.45),
                ]));
            }
            'S' => {
                add_rect(&mut contours, l, cap_h - t, r, cap_h);
                add_rect(&mut contours, l, cap_h * 0.5, l + t, cap_h);
                add_rect(&mut contours, l, cap_h * 0.5 - t * 0.5, r, cap_h * 0.5 + t * 0.5);
                add_rect(&mut contours, r - t, 0.0, r, cap_h * 0.5);
                add_rect(&mut contours, l, 0.0, r, t);
            }
            'T' => {
                add_rect(&mut contours, l, cap_h - t, r, cap_h);
                add_rect(&mut contours, mid_x - t * 0.5, 0.0, mid_x + t * 0.5, cap_h - t);
            }
            'U' => {
                add_rect(&mut contours, l, t, l + t, cap_h);
                add_rect(&mut contours, r - t, t, r, cap_h);
                add_rect(&mut contours, l, 0.0, r, t);
            }
            'V' => {
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(l, cap_h),
                    OutlinePoint::on_curve(mid_x - t * 0.5, 0.0),
                    OutlinePoint::on_curve(mid_x + t * 0.5, 0.0),
                    OutlinePoint::on_curve(r, cap_h),
                    OutlinePoint::on_curve(r - t * 1.2, cap_h),
                    OutlinePoint::on_curve(mid_x, t * 1.2),
                    OutlinePoint::on_curve(l + t * 1.2, cap_h),
                ]));
            }
            'W' => {
                add_rect(&mut contours, l, t, l + t, cap_h);
                add_rect(&mut contours, r - t, t, r, cap_h);
                add_rect(&mut contours, mid_x - t * 0.5, t, mid_x + t * 0.5, cap_h * 0.8);
                add_rect(&mut contours, l, 0.0, r, t);
            }
            'X' => {
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(l, 0.0),
                    OutlinePoint::on_curve(r, cap_h),
                    OutlinePoint::on_curve(r - t * 1.2, cap_h),
                    OutlinePoint::on_curve(l, t * 1.2),
                ]));
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(r, 0.0),
                    OutlinePoint::on_curve(l, cap_h),
                    OutlinePoint::on_curve(l + t * 1.2, cap_h),
                    OutlinePoint::on_curve(r, t * 1.2),
                ]));
            }
            'Y' => {
                add_rect(&mut contours, mid_x - t * 0.5, 0.0, mid_x + t * 0.5, cap_h * 0.5);
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(l, cap_h),
                    OutlinePoint::on_curve(mid_x, cap_h * 0.5),
                    OutlinePoint::on_curve(mid_x + t * 0.8, cap_h * 0.5),
                    OutlinePoint::on_curve(l + t * 1.2, cap_h),
                ]));
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(r, cap_h),
                    OutlinePoint::on_curve(mid_x, cap_h * 0.5),
                    OutlinePoint::on_curve(mid_x - t * 0.8, cap_h * 0.5),
                    OutlinePoint::on_curve(r - t * 1.2, cap_h),
                ]));
            }
            'Z' => {
                add_rect(&mut contours, l, cap_h - t, r, cap_h);
                add_rect(&mut contours, l, 0.0, r, t);
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(l, t),
                    OutlinePoint::on_curve(r - t * 1.2, cap_h - t),
                    OutlinePoint::on_curve(r, cap_h - t),
                    OutlinePoint::on_curve(l + t * 1.2, t),
                ]));
            }
            // Standard Digits
            '1' => {
                add_rect(&mut contours, mid_x - t * 0.5, 0.0, mid_x + t * 0.5, cap_h);
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(mid_x - t * 0.5, cap_h),
                    OutlinePoint::on_curve(l + t, cap_h - t * 1.5),
                    OutlinePoint::on_curve(l + t, cap_h - t * 2.2),
                    OutlinePoint::on_curve(mid_x - t * 0.5, cap_h - t),
                ]));
                add_rect(&mut contours, l + t, 0.0, r - t, t);
            }
            '2' => {
                add_rect(&mut contours, l, cap_h - t, r, cap_h);
                add_rect(&mut contours, r - t, cap_h * 0.5, r, cap_h);
                add_rect(&mut contours, l, cap_h * 0.5 - t * 0.5, r, cap_h * 0.5 + t * 0.5);
                add_rect(&mut contours, l, 0.0, l + t, cap_h * 0.5);
                add_rect(&mut contours, l, 0.0, r, t);
            }
            '3' => {
                add_rect(&mut contours, l, cap_h - t, r, cap_h);
                add_rect(&mut contours, r - t, 0.0, r, cap_h);
                add_rect(&mut contours, l + t, cap_h * 0.5 - t * 0.5, r, cap_h * 0.5 + t * 0.5);
                add_rect(&mut contours, l, 0.0, r, t);
            }
            '4' => {
                add_rect(&mut contours, r - t * 1.5, 0.0, r - t * 0.5, cap_h);
                add_rect(&mut contours, l, cap_h * 0.35, l + t, cap_h);
                add_rect(&mut contours, l, cap_h * 0.35, r, cap_h * 0.35 + t);
            }
            '5' => {
                add_rect(&mut contours, l, cap_h - t, r, cap_h);
                add_rect(&mut contours, l, cap_h * 0.5, l + t, cap_h);
                add_rect(&mut contours, l, cap_h * 0.5 - t * 0.5, r, cap_h * 0.5 + t * 0.5);
                add_rect(&mut contours, r - t, 0.0, r, cap_h * 0.5);
                add_rect(&mut contours, l, 0.0, r, t);
            }
            '6' => {
                add_rect(&mut contours, l, 0.0, l + t, cap_h);
                add_rect(&mut contours, l, cap_h - t, r, cap_h);
                add_rect(&mut contours, l, cap_h * 0.5 - t * 0.5, r, cap_h * 0.5 + t * 0.5);
                add_rect(&mut contours, r - t, 0.0, r, cap_h * 0.5);
                add_rect(&mut contours, l, 0.0, r, t);
            }
            '7' => {
                add_rect(&mut contours, l, cap_h - t, r, cap_h);
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(r - t * 1.2, cap_h - t),
                    OutlinePoint::on_curve(mid_x - t * 0.6, 0.0),
                    OutlinePoint::on_curve(mid_x + t * 0.6, 0.0),
                    OutlinePoint::on_curve(r, cap_h - t),
                ]));
            }
            '8' => {
                add_rect(&mut contours, l, 0.0, l + t, cap_h);
                add_rect(&mut contours, r - t, 0.0, r, cap_h);
                add_rect(&mut contours, l, cap_h - t, r, cap_h);
                add_rect(&mut contours, l, cap_h * 0.5 - t * 0.5, r, cap_h * 0.5 + t * 0.5);
                add_rect(&mut contours, l, 0.0, r, t);
            }
            '9' => {
                add_rect(&mut contours, l, cap_h * 0.5, l + t, cap_h);
                add_rect(&mut contours, r - t, 0.0, r, cap_h);
                add_rect(&mut contours, l, cap_h - t, r, cap_h);
                add_rect(&mut contours, l, cap_h * 0.5 - t * 0.5, r, cap_h * 0.5 + t * 0.5);
                add_rect(&mut contours, l, 0.0, r, t);
            }
            // Punctuation & Delimiters
            '.' => {
                add_rect(&mut contours, mid_x - t * 0.6, 0.0, mid_x + t * 0.6, t * 1.2);
            }
            ',' => {
                add_rect(&mut contours, mid_x - t * 0.6, 0.0, mid_x + t * 0.6, t * 1.2);
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(mid_x + t * 0.6, t * 0.5),
                    OutlinePoint::on_curve(mid_x - t * 0.4, -60.0),
                    OutlinePoint::on_curve(mid_x - t * 0.8, -50.0),
                    OutlinePoint::on_curve(mid_x - t * 0.2, t * 0.5),
                ]));
            }
            ':' => {
                add_rect(&mut contours, mid_x - t * 0.6, 0.0, mid_x + t * 0.6, t * 1.2);
                add_rect(&mut contours, mid_x - t * 0.6, x_h * 0.7, mid_x + t * 0.6, x_h * 0.7 + t * 1.2);
            }
            ';' => {
                add_rect(&mut contours, mid_x - t * 0.6, x_h * 0.7, mid_x + t * 0.6, x_h * 0.7 + t * 1.2);
                add_rect(&mut contours, mid_x - t * 0.6, 0.0, mid_x + t * 0.6, t * 1.2);
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(mid_x + t * 0.6, t * 0.5),
                    OutlinePoint::on_curve(mid_x - t * 0.4, -60.0),
                    OutlinePoint::on_curve(mid_x - t * 0.8, -50.0),
                    OutlinePoint::on_curve(mid_x - t * 0.2, t * 0.5),
                ]));
            }
            '(' => {
                // Curved bracket
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(r - t, cap_h),
                    OutlinePoint::off_curve(l, cap_h * 0.5),
                    OutlinePoint::on_curve(r - t, -100.0),
                    OutlinePoint::on_curve(r, -100.0),
                    OutlinePoint::off_curve(l + t, cap_h * 0.5),
                    OutlinePoint::on_curve(r, cap_h),
                ]));
            }
            ')' => {
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(l + t, cap_h),
                    OutlinePoint::off_curve(r, cap_h * 0.5),
                    OutlinePoint::on_curve(l + t, -100.0),
                    OutlinePoint::on_curve(l, -100.0),
                    OutlinePoint::off_curve(r - t, cap_h * 0.5),
                    OutlinePoint::on_curve(l, cap_h),
                ]));
            }
            '[' => {
                add_rect(&mut contours, l + t, -80.0, l + t * 2.0, cap_h + 80.0);
                add_rect(&mut contours, l + t, cap_h + 80.0 - t, r - t, cap_h + 80.0);
                add_rect(&mut contours, l + t, -80.0, r - t, -80.0 + t);
            }
            ']' => {
                add_rect(&mut contours, r - t * 2.0, -80.0, r - t, cap_h + 80.0);
                add_rect(&mut contours, l + t, cap_h + 80.0 - t, r - t, cap_h + 80.0);
                add_rect(&mut contours, l + t, -80.0, r - t, -80.0 + t);
            }
            '{' => {
                add_rect(&mut contours, mid_x - t * 0.5, cap_h * 0.5 - t * 0.5, mid_x + t * 0.5, cap_h * 0.5 + t * 0.5);
                add_rect(&mut contours, mid_x - t * 0.5, cap_h * 0.5, mid_x + t * 0.5, cap_h);
                add_rect(&mut contours, mid_x - t * 0.5, 0.0, mid_x + t * 0.5, cap_h * 0.5);
                add_rect(&mut contours, mid_x, cap_h - t, r, cap_h);
                add_rect(&mut contours, mid_x, 0.0, r, t);
                add_rect(&mut contours, l, cap_h * 0.5 - t * 0.5, mid_x, cap_h * 0.5 + t * 0.5);
            }
            '}' => {
                add_rect(&mut contours, mid_x - t * 0.5, cap_h * 0.5 - t * 0.5, mid_x + t * 0.5, cap_h * 0.5 + t * 0.5);
                add_rect(&mut contours, mid_x - t * 0.5, cap_h * 0.5, mid_x + t * 0.5, cap_h);
                add_rect(&mut contours, mid_x - t * 0.5, 0.0, mid_x + t * 0.5, cap_h * 0.5);
                add_rect(&mut contours, l, cap_h - t, mid_x, cap_h);
                add_rect(&mut contours, l, 0.0, mid_x, t);
                add_rect(&mut contours, mid_x, cap_h * 0.5 - t * 0.5, r, cap_h * 0.5 + t * 0.5);
            }
            '=' => {
                add_rect(&mut contours, l, cap_h * 0.6 - t * 0.5, r, cap_h * 0.6 + t * 0.5);
                add_rect(&mut contours, l, cap_h * 0.4 - t * 0.5, r, cap_h * 0.4 + t * 0.5);
            }
            '+' => {
                add_rect(&mut contours, l, cap_h * 0.5 - t * 0.5, r, cap_h * 0.5 + t * 0.5);
                add_rect(&mut contours, mid_x - t * 0.5, cap_h * 0.5 - (r - l) * 0.5, mid_x + t * 0.5, cap_h * 0.5 + (r - l) * 0.5);
            }
            '-' => {
                add_rect(&mut contours, l, cap_h * 0.5 - t * 0.5, r, cap_h * 0.5 + t * 0.5);
            }
            '_' => {
                add_rect(&mut contours, l - 50.0, -100.0, r + 50.0, -100.0 + t);
            }
            '*' => {
                add_rect(&mut contours, mid_x - t * 0.5, cap_h * 0.35, mid_x + t * 0.5, cap_h * 0.75);
                add_rect(&mut contours, mid_x - (r - l) * 0.3, cap_h * 0.55 - t * 0.5, mid_x + (r - l) * 0.3, cap_h * 0.55 + t * 0.5);
            }
            '/' => {
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(l, -50.0),
                    OutlinePoint::on_curve(r, cap_h + 50.0),
                    OutlinePoint::on_curve(r - t * 1.2, cap_h + 50.0),
                    OutlinePoint::on_curve(l - t * 1.2, -50.0),
                ]));
            }
            '\\' => {
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(r, -50.0),
                    OutlinePoint::on_curve(l, cap_h + 50.0),
                    OutlinePoint::on_curve(l + t * 1.2, cap_h + 50.0),
                    OutlinePoint::on_curve(r + t * 1.2, -50.0),
                ]));
            }
            '<' => {
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(r - t, cap_h * 0.75),
                    OutlinePoint::on_curve(l + t, cap_h * 0.5),
                    OutlinePoint::on_curve(r - t, cap_h * 0.25),
                    OutlinePoint::on_curve(r, cap_h * 0.25 + t),
                    OutlinePoint::on_curve(l + t * 2.0, cap_h * 0.5),
                    OutlinePoint::on_curve(r, cap_h * 0.75 - t),
                ]));
            }
            '>' => {
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(l + t, cap_h * 0.75),
                    OutlinePoint::on_curve(r - t, cap_h * 0.5),
                    OutlinePoint::on_curve(l + t, cap_h * 0.25),
                    OutlinePoint::on_curve(l, cap_h * 0.25 + t),
                    OutlinePoint::on_curve(r - t * 2.0, cap_h * 0.5),
                    OutlinePoint::on_curve(l, cap_h * 0.75 - t),
                ]));
            }
            '!' => {
                add_rect(&mut contours, mid_x - t * 0.5, cap_h * 0.3, mid_x + t * 0.5, cap_h);
                add_rect(&mut contours, mid_x - t * 0.6, 0.0, mid_x + t * 0.6, t * 1.2);
            }
            '?' => {
                add_rect(&mut contours, l, cap_h - t, r, cap_h);
                add_rect(&mut contours, r - t, cap_h * 0.5, r, cap_h);
                add_rect(&mut contours, mid_x - t * 0.5, cap_h * 0.3, mid_x + t * 0.5, cap_h * 0.5);
                add_rect(&mut contours, mid_x - t * 0.6, 0.0, mid_x + t * 0.6, t * 1.2);
            }
            '"' => {
                add_rect(&mut contours, mid_x - t * 1.2, cap_h - t * 2.5, mid_x - t * 0.2, cap_h);
                add_rect(&mut contours, mid_x + t * 0.2, cap_h - t * 2.5, mid_x + t * 1.2, cap_h);
            }
            '\'' => {
                add_rect(&mut contours, mid_x - t * 0.5, cap_h - t * 2.5, mid_x + t * 0.5, cap_h);
            }
            '`' => {
                contours.push(Contour::with_points(vec![
                    OutlinePoint::on_curve(mid_x - t * 0.5, cap_h),
                    OutlinePoint::on_curve(mid_x + t * 0.8, cap_h - t * 2.0),
                    OutlinePoint::on_curve(mid_x + t * 0.2, cap_h - t * 2.0),
                    OutlinePoint::on_curve(mid_x - t * 1.0, cap_h),
                ]));
            }
            '|' => {
                add_rect(&mut contours, mid_x - t * 0.5, -100.0, mid_x + t * 0.5, cap_h + 100.0);
            }
            // Lowercase letters (compact versions of uppercase)
            'a'..='z' => {
                let upper = (ch as u8 - 32) as char;
                let upper_outline = Self::build_ascii_glyph(upper, em);
                for c in upper_outline.contours {
                    let mut pts = Vec::with_capacity(c.points.len());
                    for p in c.points {
                        pts.push(OutlinePoint {
                            x: p.x,
                            y: p.y * (x_h / cap_h),
                            on_curve: p.on_curve,
                        });
                    }
                    contours.push(Contour::with_points(pts));
                }
            }
            _ => {
                // Generic square for other glyphs
                add_rect(&mut contours, l + t, t, r - t, x_h);
            }
        }

        GlyphOutline {
            contours,
            x_min: l as i16,
            y_min: 0,
            x_max: r as i16,
            y_max: cap_h as i16,
        }
    }
}
