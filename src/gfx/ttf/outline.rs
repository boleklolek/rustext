//! Vectorial TrueType glyph outline representation, point decomposition,
//! and quadratic Bézier curve flattening.
//! Zero third-party dependencies, purely standard library Rust.

/// 2D coordinate in floating-point pixel or design space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2D {
    pub x: f32,
    pub y: f32,
}

impl Point2D {
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn midpoint(p1: Self, p2: Self) -> Self {
        Self {
            x: (p1.x + p2.x) * 0.5,
            y: (p1.y + p2.y) * 0.5,
        }
    }

    #[inline]
    pub fn distance_squared(p1: Self, p2: Self) -> f32 {
        let dx = p1.x - p2.x;
        let dy = p1.y - p2.y;
        dx * dx + dy * dy
    }
}

/// Outline point from the TrueType `glyf` table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutlinePoint {
    pub x: f32,
    pub y: f32,
    /// True if point lies on the curve contour; false if it is an off-curve quadratic control point.
    pub on_curve: bool,
}

impl OutlinePoint {
    #[inline]
    pub const fn on_curve(x: f32, y: f32) -> Self {
        Self { x, y, on_curve: true }
    }

    #[inline]
    pub const fn off_curve(x: f32, y: f32) -> Self {
        Self { x, y, on_curve: false }
    }

    #[inline]
    pub const fn to_point2d(self) -> Point2D {
        Point2D::new(self.x, self.y)
    }
}

/// A closed contour of TrueType points.
#[derive(Debug, Clone, PartialEq)]
pub struct Contour {
    pub points: Vec<OutlinePoint>,
}

impl Contour {
    #[inline]
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    #[inline]
    pub fn with_points(points: Vec<OutlinePoint>) -> Self {
        Self { points }
    }

    /// Resolves implicit on-curve points and normalizes the contour
    /// so that every off-curve control point is flanked by on-curve points.
    /// In TrueType, two consecutive off-curve points have an implicit on-curve point at their midpoint.
    pub fn regularize_points(&self) -> Vec<OutlinePoint> {
        if self.points.is_empty() {
            return Vec::new();
        }

        let n = self.points.len();
        if n == 1 {
            return self.points.clone();
        }

        // Count how many off-curve points are first/last
        let mut result = Vec::with_capacity(n * 2);

        // Find starting on-curve point, or create one if the whole contour or ends are off-curve
        let first = self.points[0];
        let last = self.points[n - 1];

        let mut start_idx = 0;
        if !first.on_curve {
            if last.on_curve {
                // We can rotate the points to start on `last`
                start_idx = n - 1;
            } else {
                // Both first and last are off-curve: insert synthetic midpoint at the start
                let mid = OutlinePoint::on_curve((first.x + last.x) * 0.5, (first.y + last.y) * 0.5);
                result.push(mid);
            }
        }

        for i in 0..n {
            let curr = self.points[(start_idx + i) % n];
            let next = self.points[(start_idx + i + 1) % n];

            result.push(curr);

            if !curr.on_curve && !next.on_curve {
                // Implicit midpoint between consecutive off-curve points
                let mid = OutlinePoint::on_curve((curr.x + next.x) * 0.5, (curr.y + next.y) * 0.5);
                result.push(mid);
            }
        }

        result
    }
}

impl Default for Contour {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete vector outline of a glyph.
#[derive(Debug, Clone, PartialEq)]
pub struct GlyphOutline {
    pub contours: Vec<Contour>,
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
}

impl GlyphOutline {
    /// Creates a new empty glyph outline.
    #[inline]
    pub fn new() -> Self {
        Self::empty()
    }

    /// Creates an empty glyph outline (such as whitespace or non-printing characters).
    pub fn empty() -> Self {
        Self {
            contours: Vec::new(),
            x_min: 0,
            y_min: 0,
            x_max: 0,
            y_max: 0,
        }
    }

    /// Adds a contour to this glyph outline.
    #[inline]
    pub fn add_contour(&mut self, contour: Contour) {
        self.contours.push(contour);
    }
}

impl Default for GlyphOutline {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl GlyphOutline {

    /// Whether this glyph has zero contours.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.contours.is_empty() || self.contours.iter().all(|c| c.points.is_empty())
    }

    /// Decomposes and scales the outline from font design units into pixel space,
    /// flattening all quadratic Bézier curves into linear line segments using adaptive subdivision.
    pub fn scale_and_flatten(
        &self,
        scale_x: f32,
        scale_y: f32,
        offset_x: f32,
        offset_y: f32,
        tolerance: f32,
    ) -> Vec<Vec<Point2D>> {
        let mut flattened_contours = Vec::with_capacity(self.contours.len());
        let tol_sq = tolerance.max(0.05) * tolerance.max(0.05);

        for contour in &self.contours {
            let regular = contour.regularize_points();
            if regular.len() < 2 {
                continue;
            }

            let mut poly = Vec::new();

            // Transform function
            let tx = |p: Point2D| -> Point2D {
                Point2D::new(p.x * scale_x + offset_x, p.y * scale_y + offset_y)
            };

            let m = regular.len();
            let mut i = 0;
            let start_pt = tx(regular[0].to_point2d());
            poly.push(start_pt);

            while i < m {
                let p0 = tx(regular[i % m].to_point2d());
                let p1 = regular[(i + 1) % m];

                if p1.on_curve {
                    // Straight line segment
                    let p1_pt = tx(p1.to_point2d());
                    poly.push(p1_pt);
                    i += 1;
                } else {
                    // Quadratic Bézier curve: p0 (on-curve) -> p1 (off-curve) -> p2 (on-curve)
                    let p2 = regular[(i + 2) % m];
                    let p1_pt = tx(p1.to_point2d());
                    let p2_pt = tx(p2.to_point2d());

                    Self::subdivide_quadratic(p0, p1_pt, p2_pt, tol_sq, 0, &mut poly);
                    poly.push(p2_pt);
                    i += 2;
                }
            }

            if poly.len() >= 3 {
                // Ensure closed contour
                if let (Some(first), Some(last)) = (poly.first(), poly.last()) {
                    if Point2D::distance_squared(*first, *last) > 1e-4 {
                        poly.push(*first);
                    }
                }
                flattened_contours.push(poly);
            }
        }

        flattened_contours
    }

    /// Recursively subdivides a quadratic Bézier curve until its deviation from a straight line
    /// is within the squared tolerance `tol_sq`.
    fn subdivide_quadratic(
        p0: Point2D,
        p1: Point2D,
        p2: Point2D,
        tol_sq: f32,
        depth: usize,
        out: &mut Vec<Point2D>,
    ) {
        if depth >= 10 {
            return;
        }

        // Midpoint of the quadratic curve B(0.5)
        let mid_x = 0.25 * p0.x + 0.5 * p1.x + 0.25 * p2.x;
        let mid_y = 0.25 * p0.y + 0.5 * p1.y + 0.25 * p2.y;

        // Midpoint of the baseline chord (P0 + P2) / 2
        let chord_x = (p0.x + p2.x) * 0.5;
        let chord_y = (p0.y + p2.y) * 0.5;

        let dev_sq = (mid_x - chord_x) * (mid_x - chord_x) + (mid_y - chord_y) * (mid_y - chord_y);

        if dev_sq <= tol_sq {
            // Flat enough, no intermediate point needed
            return;
        }

        // De Casteljau subdivision at t = 0.5
        let q0 = Point2D::midpoint(p0, p1);
        let q1 = Point2D::midpoint(p1, p2);
        let r = Point2D::midpoint(q0, q1);

        Self::subdivide_quadratic(p0, q0, r, tol_sq, depth + 1, out);
        out.push(r);
        Self::subdivide_quadratic(r, q1, p2, tol_sq, depth + 1, out);
    }
}
