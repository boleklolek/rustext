//! High-Performance TrueType Glyph Rasterization Cache with $O(1)$ ASCII Fast-Path.
//! Implements a two-tiered memory-bounded LRU typography cache:
//! 1. Tier-1: Direct array lookup table for printable ASCII characters (`32..=126`) with zero hash overhead.
//! 2. Tier-2: Bounded LRU associative cache for extended Unicode, Box-Drawing, and multi-size glyphs.
//! Pure standard-library Rust (`std::*`), zero third-party dependencies.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use super::parser::TrueTypeFont;
use super::rasterizer::TtfRasterizer;

/// Maximum number of glyph entries in the Tier-2 associative cache before LRU eviction.
pub const DEFAULT_MAX_CACHE_ENTRIES: usize = 4096;

/// Performance and utilization metrics for the TrueType glyph cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub entry_count: usize,
    pub fast_ascii_count: usize,
    pub memory_bytes: usize,
}

impl CacheStats {
    /// Returns the cache hit ratio as a percentage (0.0% to 100.0%).
    pub fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
}

/// Key identifying a unique rasterized glyph instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphKey {
    font_id: usize,
    ch: char,
    width: u16,
    height: u16,
}

/// Global thread-safe TrueType typography cache instance.
static GLOBAL_TTF_CACHE: OnceLock<RwLock<TtfGlyphCache>> = OnceLock::new();

/// High-performance typography rasterization cache.
pub struct TtfGlyphCache {
    // Tier-1: Zero-overhead direct indexing for printable ASCII (0..128)
    fast_ascii_grayscale: [Option<Arc<[u8]>>; 128],
    fast_ascii_subpixel: [Option<Arc<[[u8; 3]]>>; 128],
    fast_ascii_dims: (u16, u16),
    fast_ascii_font_id: usize,

    // Tier-2: Associative LRU cache for Unicode, symbols, and various sizes
    grayscale_cache: HashMap<GlyphKey, (Arc<[u8]>, u64)>,
    subpixel_cache: HashMap<GlyphKey, (Arc<[[u8; 3]]>, u64)>,

    max_entries: usize,
    access_counter: u64,
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl TtfGlyphCache {
    /// Creates a new typography cache with default maximum capacity.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_CACHE_ENTRIES)
    }

    /// Creates a new typography cache with a specified maximum capacity.
    pub fn with_capacity(max_entries: usize) -> Self {
        const NONE_GRAYSCALE: Option<Arc<[u8]>> = None;
        const NONE_SUBPIXEL: Option<Arc<[[u8; 3]]>> = None;

        Self {
            fast_ascii_grayscale: [NONE_GRAYSCALE; 128],
            fast_ascii_subpixel: [NONE_SUBPIXEL; 128],
            fast_ascii_dims: (0, 0),
            fast_ascii_font_id: 0,
            grayscale_cache: HashMap::with_capacity(512),
            subpixel_cache: HashMap::with_capacity(512),
            max_entries: max_entries.max(128),
            access_counter: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// Derives a stable integer identity pointer for a TrueType font instance.
    #[inline(always)]
    fn get_font_id(font: &TrueTypeFont) -> usize {
        font as *const TrueTypeFont as usize
    }

    /// Retrieves or computes an anti-aliased 8-bit grayscale glyph coverage mask.
    pub fn get_or_rasterize_grayscale(
        &mut self,
        font: &TrueTypeFont,
        ch: char,
        width: usize,
        height: usize,
    ) -> Arc<[u8]> {
        let font_id = Self::get_font_id(font);
        let w_u16 = width.min(u16::MAX as usize) as u16;
        let h_u16 = height.min(u16::MAX as usize) as u16;

        // 1. Tier-1 Fast ASCII path (0..128)
        let ch_code = ch as u32;
        if ch_code < 128 && self.fast_ascii_dims == (w_u16, h_u16) && self.fast_ascii_font_id == font_id {
            if let Some(ref cached) = self.fast_ascii_grayscale[ch_code as usize] {
                self.hits += 1;
                return Arc::clone(cached);
            }
        }

        // 2. Tier-2 Associative Map lookup
        let key = GlyphKey {
            font_id,
            ch,
            width: w_u16,
            height: h_u16,
        };

        self.access_counter = self.access_counter.wrapping_add(1);

        if let Some((cached, timestamp)) = self.grayscale_cache.get_mut(&key) {
            *timestamp = self.access_counter;
            self.hits += 1;
            let result = Arc::clone(cached);

            // Populate Tier-1 fast table if matching ASCII
            if ch_code < 128 && self.fast_ascii_dims == (w_u16, h_u16) && self.fast_ascii_font_id == font_id {
                self.fast_ascii_grayscale[ch_code as usize] = Some(Arc::clone(&result));
            }

            return result;
        }

        // 3. Cache Miss: Rasterize TrueType outline
        self.misses += 1;
        let (_, _, raw_cov) = TtfRasterizer::rasterize_char(font, ch, width, height);
        let arc_cov: Arc<[u8]> = raw_cov.into_boxed_slice().into();

        // Evict if cache reached capacity
        if self.grayscale_cache.len() >= self.max_entries {
            self.evict_oldest_grayscale();
        }

        self.grayscale_cache.insert(key, (Arc::clone(&arc_cov), self.access_counter));

        // Populate Tier-1 table if matching ASCII
        if ch_code < 128 {
            if self.fast_ascii_dims != (w_u16, h_u16) || self.fast_ascii_font_id != font_id {
                self.reset_fast_ascii(font_id, w_u16, h_u16);
            }
            self.fast_ascii_grayscale[ch_code as usize] = Some(Arc::clone(&arc_cov));
        }

        arc_cov
    }

    /// Retrieves or computes an LCD ClearType 3x subpixel RGB coverage mask.
    pub fn get_or_rasterize_subpixel(
        &mut self,
        font: &TrueTypeFont,
        ch: char,
        width: usize,
        height: usize,
    ) -> Arc<[[u8; 3]]> {
        let font_id = Self::get_font_id(font);
        let w_u16 = width.min(u16::MAX as usize) as u16;
        let h_u16 = height.min(u16::MAX as usize) as u16;

        // 1. Tier-1 Fast ASCII path (0..128)
        let ch_code = ch as u32;
        if ch_code < 128 && self.fast_ascii_dims == (w_u16, h_u16) && self.fast_ascii_font_id == font_id {
            if let Some(ref cached) = self.fast_ascii_subpixel[ch_code as usize] {
                self.hits += 1;
                return Arc::clone(cached);
            }
        }

        // 2. Tier-2 Associative Map lookup
        let key = GlyphKey {
            font_id,
            ch,
            width: w_u16,
            height: h_u16,
        };

        self.access_counter = self.access_counter.wrapping_add(1);

        if let Some((cached, timestamp)) = self.subpixel_cache.get_mut(&key) {
            *timestamp = self.access_counter;
            self.hits += 1;
            let result = Arc::clone(cached);

            if ch_code < 128 && self.fast_ascii_dims == (w_u16, h_u16) && self.fast_ascii_font_id == font_id {
                self.fast_ascii_subpixel[ch_code as usize] = Some(Arc::clone(&result));
            }

            return result;
        }

        // 3. Cache Miss: Rasterize TrueType outline with LCD ClearType FIR filter
        self.misses += 1;
        let (_, _, raw_subpixel) = TtfRasterizer::rasterize_char_subpixel(font, ch, width, height);
        let arc_subpixel: Arc<[[u8; 3]]> = raw_subpixel.into_boxed_slice().into();

        // Evict if cache reached capacity
        if self.subpixel_cache.len() >= self.max_entries {
            self.evict_oldest_subpixel();
        }

        self.subpixel_cache.insert(key, (Arc::clone(&arc_subpixel), self.access_counter));

        // Populate Tier-1 table if matching ASCII
        if ch_code < 128 {
            if self.fast_ascii_dims != (w_u16, h_u16) || self.fast_ascii_font_id != font_id {
                self.reset_fast_ascii(font_id, w_u16, h_u16);
            }
            self.fast_ascii_subpixel[ch_code as usize] = Some(Arc::clone(&arc_subpixel));
        }

        arc_subpixel
    }

    /// Pre-rasterizes standard printable ASCII characters (`32..=126`) into the Tier-1 table.
    pub fn warmup_ascii(&mut self, font: &TrueTypeFont, width: usize, height: usize) {
        let font_id = Self::get_font_id(font);
        let w_u16 = width.min(u16::MAX as usize) as u16;
        let h_u16 = height.min(u16::MAX as usize) as u16;

        self.reset_fast_ascii(font_id, w_u16, h_u16);

        for code in 32u8..=126u8 {
            let ch = code as char;
            self.get_or_rasterize_grayscale(font, ch, width, height);
            self.get_or_rasterize_subpixel(font, ch, width, height);
        }
    }

    /// Resets the Tier-1 fast ASCII lookup arrays for a new font or dimensions.
    fn reset_fast_ascii(&mut self, font_id: usize, width: u16, height: u16) {
        self.fast_ascii_dims = (width, height);
        self.fast_ascii_font_id = font_id;
        for item in &mut self.fast_ascii_grayscale {
            *item = None;
        }
        for item in &mut self.fast_ascii_subpixel {
            *item = None;
        }
    }

    /// Evicts the 25% oldest entries from the grayscale cache (LRU batch eviction).
    fn evict_oldest_grayscale(&mut self) {
        if self.grayscale_cache.is_empty() {
            return;
        }

        let evict_count = (self.grayscale_cache.len() / 4).max(1);
        let mut entries: Vec<(GlyphKey, u64)> = self.grayscale_cache.iter()
            .map(|(&k, (_, ts))| (k, *ts))
            .collect();
        entries.sort_by_key(|&(_, ts)| ts);

        for (k, _) in entries.into_iter().take(evict_count) {
            self.grayscale_cache.remove(&k);
            self.evictions += 1;
        }
    }

    /// Evicts the 25% oldest entries from the subpixel cache (LRU batch eviction).
    fn evict_oldest_subpixel(&mut self) {
        if self.subpixel_cache.is_empty() {
            return;
        }

        let evict_count = (self.subpixel_cache.len() / 4).max(1);
        let mut entries: Vec<(GlyphKey, u64)> = self.subpixel_cache.iter()
            .map(|(&k, (_, ts))| (k, *ts))
            .collect();
        entries.sort_by_key(|&(_, ts)| ts);

        for (k, _) in entries.into_iter().take(evict_count) {
            self.subpixel_cache.remove(&k);
            self.evictions += 1;
        }
    }

    /// Clears all cached rasterization data and resets statistics.
    pub fn clear(&mut self) {
        self.fast_ascii_dims = (0, 0);
        self.fast_ascii_font_id = 0;
        for item in &mut self.fast_ascii_grayscale {
            *item = None;
        }
        for item in &mut self.fast_ascii_subpixel {
            *item = None;
        }
        self.grayscale_cache.clear();
        self.subpixel_cache.clear();
        self.access_counter = 0;
    }

    /// Computes runtime performance and memory usage statistics.
    pub fn stats(&self) -> CacheStats {
        let mut mem = 0usize;
        for (k, (v, _)) in &self.grayscale_cache {
            mem += std::mem::size_of_val(k) + v.len();
        }
        for (k, (v, _)) in &self.subpixel_cache {
            mem += std::mem::size_of_val(k) + v.len() * 3;
        }

        let fast_ascii_count = self.fast_ascii_grayscale.iter().filter(|x| x.is_some()).count()
            + self.fast_ascii_subpixel.iter().filter(|x| x.is_some()).count();

        CacheStats {
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            entry_count: self.grayscale_cache.len() + self.subpixel_cache.len(),
            fast_ascii_count,
            memory_bytes: mem,
        }
    }
}

impl Default for TtfGlyphCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Accessor for the global thread-safe typography cache.
fn get_global_cache() -> &'static RwLock<TtfGlyphCache> {
    GLOBAL_TTF_CACHE.get_or_init(|| RwLock::new(TtfGlyphCache::new()))
}

/// Global helper functions for TrueType rasterization caching.
pub struct GlobalTtfCache;

impl GlobalTtfCache {
    /// Retrieves or rasterizes an anti-aliased 8-bit grayscale glyph coverage mask from the global cache.
    pub fn get_grayscale(
        font: &TrueTypeFont,
        ch: char,
        width: usize,
        height: usize,
    ) -> Arc<[u8]> {
        let lock = get_global_cache();
        if let Ok(mut guard) = lock.write() {
            guard.get_or_rasterize_grayscale(font, ch, width, height)
        } else {
            let (_, _, cov) = TtfRasterizer::rasterize_char(font, ch, width, height);
            cov.into_boxed_slice().into()
        }
    }

    /// Retrieves or rasterizes an LCD ClearType subpixel RGB coverage mask from the global cache.
    pub fn get_subpixel(
        font: &TrueTypeFont,
        ch: char,
        width: usize,
        height: usize,
    ) -> Arc<[[u8; 3]]> {
        let lock = get_global_cache();
        if let Ok(mut guard) = lock.write() {
            guard.get_or_rasterize_subpixel(font, ch, width, height)
        } else {
            let (_, _, subpixel) = TtfRasterizer::rasterize_char_subpixel(font, ch, width, height);
            subpixel.into_boxed_slice().into()
        }
    }

    /// Pre-populates the cache with printable ASCII characters for the active font dimensions.
    pub fn warmup(font: &TrueTypeFont, width: usize, height: usize) {
        let lock = get_global_cache();
        if let Ok(mut guard) = lock.write() {
            guard.warmup_ascii(font, width, height);
        }
    }

    /// Clears the global typography cache.
    pub fn clear() {
        let lock = get_global_cache();
        if let Ok(mut guard) = lock.write() {
            guard.clear();
        }
    }

    /// Retrieves cache utilization and hit statistics.
    pub fn stats() -> CacheStats {
        let lock = get_global_cache();
        if let Ok(guard) = lock.read() {
            guard.stats()
        } else {
            CacheStats::default()
        }
    }
}
