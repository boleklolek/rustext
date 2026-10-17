//! Pure Rust TrueType (.ttf) font parser, vector outline reconstructor,
//! and analytical scanline area anti-aliasing rasterizer.
//! Zero third-party dependencies, purely standard library Rust (`std::*`).

pub mod cache;
pub mod generator;
pub mod loader;
pub mod outline;
pub mod parser;
pub mod rasterizer;
pub mod reader;

pub use cache::{CacheStats, GlobalTtfCache, TtfGlyphCache};
pub use generator::TtfFontGenerator;
pub use loader::SystemFontLoader;
pub use outline::{Contour, GlyphOutline, OutlinePoint, Point2D};
pub use parser::{GlyphMetrics, TrueTypeFont};
pub use rasterizer::TtfRasterizer;
pub use reader::{TtfError, TtfReader};
