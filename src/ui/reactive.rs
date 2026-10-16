//! Fine-Grained Reactive State Engine for Rustext.
//! Pure standard-library Rust with zero third-party dependencies and 100% safe Rust.
//! Provides Signals, Memos (Computed values), Effects, and reactive dirty tracking.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SIGNAL_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_SUBSCRIPTION_ID: AtomicU64 = AtomicU64::new(1);

/// Unique identifier for each reactive signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalId(pub u64);

/// Unique identifier for each reactive subscription callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(pub u64);

type SubscriberCallback = Rc<dyn Fn(u64)>;

struct Subscriber {
    id: SubscriptionId,
    callback: SubscriberCallback,
}

struct SignalInner<T> {
    id: SignalId,
    value: T,
    version: u64,
    subscribers: Vec<Subscriber>,
}

thread_local! {
    /// Active dependency collector during Computed/Effect evaluation.
    static ACTIVE_COLLECTOR: RefCell<Option<Rc<RefCell<Vec<SignalId>>>>> = const { RefCell::new(None) };
}

/// A fine-grained reactive state container.
/// Cloning a Signal creates an ergonomic reference to the same reactive value.
#[derive(Clone)]
pub struct Signal<T> {
    inner: Rc<RefCell<SignalInner<T>>>,
}

impl<T: 'static> Signal<T> {
    /// Creates a new reactive signal with an initial value.
    pub fn new(initial: T) -> Self {
        let id = SignalId(NEXT_SIGNAL_ID.fetch_add(1, Ordering::Relaxed));
        Self {
            inner: Rc::new(RefCell::new(SignalInner {
                id,
                value: initial,
                version: 1,
                subscribers: Vec::new(),
            })),
        }
    }

    /// Unique ID for this signal.
    #[inline]
    pub fn id(&self) -> SignalId {
        self.inner.borrow().id
    }

    /// Current version epoch for this signal.
    #[inline]
    pub fn version(&self) -> u64 {
        self.inner.borrow().version
    }

    /// Reads the value and tracks this signal as a dependency if called inside a Computed or Effect.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.track_read();
        self.inner.borrow().value.clone()
    }

    /// Inspects the value by reference without cloning, registering reactive tracking.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.track_read();
        f(&self.inner.borrow().value)
    }

    /// Sets a new value. If the value differs, increments version and notifies subscribers.
    pub fn set(&self, new_value: T)
    where
        T: PartialEq,
    {
        let changed = {
            let mut inner = self.inner.borrow_mut();
            if inner.value != new_value {
                inner.value = new_value;
                inner.version = inner.version.wrapping_add(1);
                true
            } else {
                false
            }
        };

        if changed {
            self.notify_subscribers();
        }
    }

    /// Sets a new value unconditionally, incrementing version and notifying subscribers.
    pub fn set_unconditional(&self, new_value: T) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.value = new_value;
            inner.version = inner.version.wrapping_add(1);
        }
        self.notify_subscribers();
    }

    /// Mutates the inner value in-place, increments version and notifies subscribers.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        {
            let mut inner = self.inner.borrow_mut();
            f(&mut inner.value);
            inner.version = inner.version.wrapping_add(1);
        }
        self.notify_subscribers();
    }

    /// Subscribes a callback to receive notifications when this signal updates.
    pub fn subscribe(&self, callback: impl Fn(u64) + 'static) -> SubscriptionId {
        let sub_id = SubscriptionId(NEXT_SUBSCRIPTION_ID.fetch_add(1, Ordering::Relaxed));
        self.inner.borrow_mut().subscribers.push(Subscriber {
            id: sub_id,
            callback: Rc::new(callback),
        });
        sub_id
    }

    /// Unsubscribes a previously registered listener.
    pub fn unsubscribe(&self, sub_id: SubscriptionId) {
        self.inner
            .borrow_mut()
            .subscribers
            .retain(|sub| sub.id != sub_id);
    }

    #[inline]
    fn track_read(&self) {
        let sig_id = self.inner.borrow().id;
        ACTIVE_COLLECTOR.with(|cell| {
            if let Some(collector) = cell.borrow().as_ref() {
                collector.borrow_mut().push(sig_id);
            }
        });
    }

    fn notify_subscribers(&self) {
        let (version, callbacks): (u64, Vec<SubscriberCallback>) = {
            let inner = self.inner.borrow();
            let cbs = inner.subscribers.iter().map(|s| s.callback.clone()).collect();
            (inner.version, cbs)
        };

        for cb in callbacks {
            cb(version);
        }
    }
}

/// A cached, lazily evaluated derived computation.
/// Re-computes only when invalidated or marked dirty.
pub struct Computed<T> {
    compute_fn: Rc<dyn Fn() -> T>,
    cached_value: RefCell<Option<T>>,
    is_dirty: Rc<RefCell<bool>>,
    eval_count: Rc<RefCell<usize>>,
}

impl<T: Clone + 'static> Computed<T> {
    /// Creates a new computed value given a computation closure.
    pub fn new(compute_fn: impl Fn() -> T + 'static) -> Self {
        Self {
            compute_fn: Rc::new(compute_fn),
            cached_value: RefCell::new(None),
            is_dirty: Rc::new(RefCell::new(true)),
            eval_count: Rc::new(RefCell::new(0)),
        }
    }

    /// Creates a computed value linked to a specific signal that invalidates it on updates.
    pub fn from_signal<S: 'static, F: Fn(&S) -> T + 'static>(
        signal: Signal<S>,
        compute_fn: F,
    ) -> Self {
        let is_dirty = Rc::new(RefCell::new(true));
        let is_dirty_clone = is_dirty.clone();

        signal.subscribe(move |_version| {
            *is_dirty_clone.borrow_mut() = true;
        });

        let sig = signal;
        let compute = Rc::new(move || {
            sig.with(|val| compute_fn(val))
        });

        Self {
            compute_fn: compute,
            cached_value: RefCell::new(None),
            is_dirty,
            eval_count: Rc::new(RefCell::new(0)),
        }
    }

    /// Reads the computed value, evaluating lazily only when dirty.
    pub fn get(&self) -> T {
        if *self.is_dirty.borrow() || self.cached_value.borrow().is_none() {
            let new_value = (self.compute_fn)();
            *self.cached_value.borrow_mut() = Some(new_value.clone());
            *self.is_dirty.borrow_mut() = false;
            *self.eval_count.borrow_mut() += 1;
            new_value
        } else {
            self.cached_value.borrow().as_ref().unwrap().clone()
        }
    }

    /// Explicitly marks the computation as dirty for lazy recomputation.
    pub fn invalidate(&self) {
        *self.is_dirty.borrow_mut() = true;
    }

    /// Total number of times this computed function actually evaluated.
    pub fn eval_count(&self) -> usize {
        *self.eval_count.borrow()
    }
}

/// Dispatches an action automatically when dependent signals update.
pub struct Effect {
    subscription_ids: Vec<SubscriptionId>,
}

impl Effect {
    /// Connects an action to trigger whenever the given signal emits a change.
    pub fn on_signal<T: 'static>(
        signal: &Signal<T>,
        action: impl FnMut() + 'static,
    ) -> Self {
        let action_cell = Rc::new(RefCell::new(action));

        // Execute immediately upon creation
        (action_cell.borrow_mut())();

        let action_clone = action_cell.clone();
        let sub_id = signal.subscribe(move |_ver| {
            (action_clone.borrow_mut())();
        });

        Self {
            subscription_ids: vec![sub_id],
        }
    }

    /// Returns the active subscription IDs managed by this effect.
    pub fn subscription_ids(&self) -> &[SubscriptionId] {
        &self.subscription_ids
    }
}

/// Reactive dirty tracking for fine-grained UI invalidation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DirtyFlags {
    pub layout: bool,
    pub paint: bool,
}

impl DirtyFlags {
    pub const fn clean() -> Self {
        Self { layout: false, paint: false }
    }

    pub const fn all_dirty() -> Self {
        Self { layout: true, paint: true }
    }

    #[inline]
    pub fn mark_layout_dirty(&mut self) {
        self.layout = true;
        self.paint = true; // Layout changes always require repainting
    }

    #[inline]
    pub fn mark_paint_dirty(&mut self) {
        self.paint = true;
    }

    #[inline]
    pub fn clear(&mut self) {
        self.layout = false;
        self.paint = false;
    }

    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.layout || self.paint
    }
}
