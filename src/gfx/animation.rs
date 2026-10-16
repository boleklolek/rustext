//! Animation Curves, Easing Functions, and Cubic-Bézier Solvers.
//! Provides smooth motion dynamics for cursor transitions, scrolling, dialog reveals, and UI states.
//! 100% pure standard-library Rust with zero third-party dependencies.

use std::f64::consts::PI;

/// Parametric animation easing functions mapping normalized time `t ∈ [0.0, 1.0]` to progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Easing {
    Linear,
    EaseInQuad,
    #[default]
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInQuart,
    EaseOutQuart,
    EaseInOutQuart,
    EaseInExpo,
    EaseOutExpo,
    EaseInOutExpo,
    EaseInCirc,
    EaseOutCirc,
    EaseInOutCirc,
    EaseOutBack,
    EaseInOutBack,
    EaseOutBounce,
    EaseInOutBounce,
    EaseOutElastic,
}

impl Easing {
    /// Evaluates the easing function at normalized parameter `t ∈ [0.0, 1.0]`.
    pub fn evaluate(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseInQuad => t * t,
            Self::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Self::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            Self::EaseInCubic => t * t * t,
            Self::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            Self::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Self::EaseInQuart => t * t * t * t,
            Self::EaseOutQuart => 1.0 - (1.0 - t).powi(4),
            Self::EaseInOutQuart => {
                if t < 0.5 {
                    8.0 * t * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
                }
            }
            Self::EaseInExpo => {
                if t == 0.0 {
                    0.0
                } else {
                    (2.0f64).powf(10.0 * t - 10.0)
                }
            }
            Self::EaseOutExpo => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - (2.0f64).powf(-10.0 * t)
                }
            }
            Self::EaseInOutExpo => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    (2.0f64).powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - (2.0f64).powf(-20.0 * t + 10.0)) / 2.0
                }
            }
            Self::EaseInCirc => 1.0 - (1.0 - t * t).sqrt(),
            Self::EaseOutCirc => (1.0 - (t - 1.0).powi(2)).sqrt(),
            Self::EaseInOutCirc => {
                if t < 0.5 {
                    (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0
                } else {
                    ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0
                }
            }
            Self::EaseOutBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
            }
            Self::EaseInOutBack => {
                let c1 = 1.70158;
                let c2 = c1 * 1.525;
                if t < 0.5 {
                    ((2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2)) / 2.0
                } else {
                    ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (t * 2.0 - 2.0) + c2) + 2.0) / 2.0
                }
            }
            Self::EaseOutBounce => Self::bounce_out(t),
            Self::EaseInOutBounce => {
                if t < 0.5 {
                    (1.0 - Self::bounce_out(1.0 - 2.0 * t)) / 2.0
                } else {
                    (1.0 + Self::bounce_out(2.0 * t - 1.0)) / 2.0
                }
            }
            Self::EaseOutElastic => {
                let c4 = (2.0 * PI) / 3.0;
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    (2.0f64).powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
                }
            }
        }
    }

    fn bounce_out(t: f64) -> f64 {
        let n1 = 7.5625;
        let d1 = 2.75;
        if t < 1.0 / d1 {
            n1 * t * t
        } else if t < 2.0 / d1 {
            let t = t - 1.5 / d1;
            n1 * t * t + 0.75
        } else if t < 2.5 / d1 {
            let t = t - 2.25 / d1;
            n1 * t * t + 0.9375
        } else {
            let t = t - 2.625 / d1;
            n1 * t * t + 0.984375
        }
    }
}

/// Parametric Cubic-Bézier curve solver matching the CSS `cubic-bezier(x1, y1, x2, y2)` standard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubicBezier {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl CubicBezier {
    pub const fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// CSS standard `ease` curve `(0.25, 0.1, 0.25, 1.0)`
    pub const fn ease() -> Self {
        Self::new(0.25, 0.1, 0.25, 1.0)
    }

    /// CSS standard `ease-in` curve `(0.42, 0.0, 1.0, 1.0)`
    pub const fn ease_in() -> Self {
        Self::new(0.42, 0.0, 1.0, 1.0)
    }

    /// CSS standard `ease-out` curve `(0.0, 0.0, 0.58, 1.0)`
    pub const fn ease_out() -> Self {
        Self::new(0.0, 0.0, 0.58, 1.0)
    }

    /// CSS standard `ease-in-out` curve `(0.42, 0.0, 0.58, 1.0)`
    pub const fn ease_in_out() -> Self {
        Self::new(0.42, 0.0, 0.58, 1.0)
    }

    /// Solves for Y given X using Newton-Raphson iteration with binary bisection fallback.
    pub fn sample(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        if x >= 1.0 {
            return 1.0;
        }

        // Newton-Raphson to find parameter t such that B_x(t) = x
        let mut t = x;
        for _ in 0..8 {
            let x_est = self.calc_bezier_coord(t, self.x1, self.x2);
            let slope = self.calc_bezier_slope(t, self.x1, self.x2);
            if slope.abs() < 1e-6 {
                break;
            }
            let diff = x_est - x;
            if diff.abs() < 1e-5 {
                return self.calc_bezier_coord(t, self.y1, self.y2);
            }
            t -= diff / slope;
            t = t.clamp(0.0, 1.0);
        }

        // Fallback: binary bisection
        let mut t0 = 0.0;
        let mut t1 = 1.0;
        t = x;
        for _ in 0..12 {
            let x_est = self.calc_bezier_coord(t, self.x1, self.x2);
            if (x_est - x).abs() < 1e-5 {
                break;
            }
            if x > x_est {
                t0 = t;
            } else {
                t1 = t;
            }
            t = (t0 + t1) * 0.5;
        }

        self.calc_bezier_coord(t, self.y1, self.y2)
    }

    #[inline]
    fn calc_bezier_coord(&self, t: f64, p1: f64, p2: f64) -> f64 {
        let inv_t = 1.0 - t;
        3.0 * inv_t * inv_t * t * p1 + 3.0 * inv_t * t * t * p2 + t * t * t
    }

    #[inline]
    fn calc_bezier_slope(&self, t: f64, p1: f64, p2: f64) -> f64 {
        let inv_t = 1.0 - t;
        3.0 * inv_t * inv_t * p1 + 6.0 * inv_t * t * (p2 - p1) + 3.0 * t * t * (1.0 - p2)
    }
}

/// Stateful animated scalar value with easing transitions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimatedFloat {
    pub current: f64,
    pub start: f64,
    pub target: f64,
    pub duration_ms: f64,
    pub elapsed_ms: f64,
    pub easing: Easing,
    pub is_finished: bool,
}

impl AnimatedFloat {
    pub fn new(initial: f64) -> Self {
        Self {
            current: initial,
            start: initial,
            target: initial,
            duration_ms: 200.0,
            elapsed_ms: 200.0,
            easing: Easing::EaseOutQuad,
            is_finished: true,
        }
    }

    /// Sets a new target to animate toward.
    pub fn animate_to(&mut self, target: f64, duration_ms: f64, easing: Easing) {
        self.start = self.current;
        self.target = target;
        self.duration_ms = duration_ms.max(1.0);
        self.elapsed_ms = 0.0;
        self.easing = easing;
        self.is_finished = (self.start - self.target).abs() < 1e-4;
    }

    /// Advances the animation by `delta_ms` milliseconds.
    pub fn update(&mut self, delta_ms: f64) -> f64 {
        if self.is_finished {
            return self.current;
        }

        self.elapsed_ms += delta_ms;
        if self.elapsed_ms >= self.duration_ms {
            self.current = self.target;
            self.is_finished = true;
        } else {
            let t = (self.elapsed_ms / self.duration_ms).clamp(0.0, 1.0);
            let progress = self.easing.evaluate(t);
            self.current = self.start + (self.target - self.start) * progress;
        }

        self.current
    }
}

/// Damped harmonic spring oscillator for organic, physics-based UI bounce.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringAnimation {
    pub value: f64,
    pub velocity: f64,
    pub target: f64,
    pub stiffness: f64,
    pub damping: f64,
    pub mass: f64,
}

impl SpringAnimation {
    pub fn new(initial: f64) -> Self {
        Self {
            value: initial,
            velocity: 0.0,
            target: initial,
            stiffness: 180.0,
            damping: 18.0,
            mass: 1.0,
        }
    }

    pub fn set_target(&mut self, target: f64) {
        self.target = target;
    }

    /// Advances spring physics by `dt` seconds (e.g. 0.016 for 60fps).
    pub fn step(&mut self, dt: f64) -> f64 {
        let spring_force = -self.stiffness * (self.value - self.target);
        let damping_force = -self.damping * self.velocity;
        let acceleration = (spring_force + damping_force) / self.mass;

        self.velocity += acceleration * dt;
        self.value += self.velocity * dt;

        if (self.value - self.target).abs() < 1e-3 && self.velocity.abs() < 1e-3 {
            self.value = self.target;
            self.velocity = 0.0;
        }

        self.value
    }
}
