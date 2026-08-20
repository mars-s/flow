//! Shared pulse clock for the repeating loaders.
//!
//! Ported from Zeron's motion kit (<https://github.com/zeronsh/comet>, MIT).
//! A repeating `with_animation` element requests a redraw every display frame
//! for as long as it is mounted — one working row pinned the whole window at
//! 120 Hz on a ProMotion panel. Loaders instead read their phase from one
//! shared clock: it ticks at ~30 fps, notifies only views that painted a
//! loader recently, and parks itself once the last lease lapses, so a window
//! with no loader mounted schedules nothing at all. Every loader shares one
//! epoch, keeping multi-instance loaders phase-locked.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use gpui::{
    AnyElement, App, EntityId, Global, IntoElement, RenderOnce, Svg, Transformation, Window,
    ease_out_quint, percentage,
};

/// Repeat-tick interval (~30 fps): visually equivalent for these chunky
/// pulses and spins at a quarter of a ProMotion display's redraws.
const PULSE_TICK: Duration = Duration::from_millis(33);

/// How long a view stays on the tick list after it last painted a loader. One
/// lease outlives a few missed frames; an unmounted loader stops renewing and
/// its view drops off, letting the clock park.
const PULSE_LEASE: Duration = Duration::from_millis(300);

/// The rotating `loader-circle` spinners' period.
const SPINNER_PERIOD: Duration = Duration::from_millis(900);

struct Lease {
    until: Instant,
    /// Notify this view every `stride`-th tick. A view's whole subtree
    /// rebuilds per notify, so a loader on an expensive surface can trade
    /// animation granularity for a cheaper cadence.
    stride: u32,
}

struct PulseClock {
    epoch: Instant,
    leases: HashMap<EntityId, Lease>,
    ticks: u64,
    running: bool,
}

impl Global for PulseClock {}

impl Default for PulseClock {
    fn default() -> Self {
        Self {
            epoch: Instant::now(),
            leases: HashMap::new(),
            ticks: 0,
            running: false,
        }
    }
}

/// Keep `view` re-rendering at [`PULSE_TICK`] until the lease lapses. A caller
/// that stops leasing stops being notified, and the clock parks once no
/// leases remain — quiescence needs no unsubscribe step.
pub fn pulse_lease(view: EntityId, cx: &mut App) {
    pulse_lease_with_stride(view, 1, cx);
}

/// [`pulse_lease`] at every second tick (~15 fps), for animations whose view
/// is expensive to rebuild and whose motion survives the coarser step — a
/// notify re-renders the view's whole subtree, so cadence is priced per
/// tick, not per animation.
pub fn pulse_lease_slow(view: EntityId, cx: &mut App) {
    pulse_lease_with_stride(view, 2, cx);
}

fn pulse_lease_with_stride(view: EntityId, stride: u32, cx: &mut App) {
    let clock = cx.default_global::<PulseClock>();
    let until = Instant::now() + PULSE_LEASE;
    // A view hosting both a full-rate and a strided loader keeps full rate.
    clock
        .leases
        .entry(view)
        .and_modify(|lease| {
            lease.until = until;
            lease.stride = lease.stride.min(stride);
        })
        .or_insert(Lease { until, stride });
    if clock.running {
        return;
    }
    clock.running = true;
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(PULSE_TICK).await;
            let parked = cx.update(|cx| {
                let clock = cx.default_global::<PulseClock>();
                let now = Instant::now();
                clock.ticks += 1;
                let ticks = clock.ticks;
                clock.leases.retain(|_, lease| lease.until > now);
                if clock.leases.is_empty() {
                    clock.running = false;
                    return true;
                }
                let due = clock
                    .leases
                    .iter_mut()
                    .filter(|(_, lease)| ticks % lease.stride.max(1) as u64 == 0)
                    .map(|(view, lease)| {
                        // Strides re-establish on the render this notify
                        // triggers; without the reset, one full-rate lease
                        // would drag its view's cadence down permanently.
                        lease.stride = u32::MAX;
                        *view
                    })
                    .collect::<Vec<_>>();
                for view in due {
                    cx.notify(view);
                }
                false
            });
            if parked {
                break;
            }
        }
    })
    .detach();
}

/// Phase `[0,1)` of a repeating cycle of `period`, plus a lease keeping `view`
/// re-rendering while its loader stays mounted. Under reduce-motion this is a
/// constant 0 — the cycle's first frame, matching what a repeating
/// `with_animation` held — and nothing is scheduled.
fn pulse_phase(period: Duration, stride: u32, view: EntityId, cx: &mut App) -> f32 {
    if cx.reduce_motion() {
        return 0.0;
    }
    let clock = cx.default_global::<PulseClock>();
    let phase = (clock.epoch.elapsed().as_secs_f32() / period.as_secs_f32()).fract();
    pulse_lease_with_stride(view, stride, cx);
    phase
}

/// A loader element styled from the shared clock's phase. Resolving the phase
/// is deferred to render, where the owning view is known, so call sites need
/// neither a `Window` nor an `EntityId` in scope.
pub fn pulse(period: Duration, render: impl FnOnce(f32) -> AnyElement + 'static) -> Pulse {
    Pulse {
        period,
        stride: 1,
        render: Box::new(render),
    }
}

/// A rotating loader icon riding the shared clock.
pub fn spin(icon: Svg) -> AnyElement {
    spin_with_stride(icon, 1)
}

/// A rotating loader at every second tick (~15 fps — the classic
/// discrete-step spinner cadence). For loaders on expensive surfaces: the
/// sidebar rebuilds its whole subtree per notify, and a session row's working
/// spinner is not worth pricing that at full rate.
pub fn spin_slow(icon: Svg) -> AnyElement {
    spin_with_stride(icon, 2)
}

fn spin_with_stride(icon: Svg, stride: u32) -> AnyElement {
    let mut pulse = pulse(SPINNER_PERIOD, move |phase| {
        icon.with_transformation(Transformation::rotate(percentage(phase)))
            .into_any_element()
    });
    pulse.stride = stride;
    pulse.into_any_element()
}

#[derive(IntoElement)]
pub struct Pulse {
    period: Duration,
    stride: u32,
    render: Box<dyn FnOnce(f32) -> AnyElement>,
}

impl Pulse {
    /// Tick every `stride`-th pulse instead of every one. A view's whole
    /// subtree rebuilds per notify — the pane ticks at the fastest of its
    /// lessees — so a loader mounted for a whole turn on an expensive
    /// surface should ride the coarser cadence.
    pub fn every(mut self, stride: u32) -> Self {
        self.stride = stride.max(1);
        self
    }
}

impl RenderOnce for Pulse {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let phase = pulse_phase(self.period, self.stride, window.current_view(), cx);
        (self.render)(phase)
    }
}

/// PRD §7: "Navigation cross-fades or slides only the main pane (120–160
/// ms)". Shared by anything that reads as *a new place appearing* rather
/// than an existing row changing in place — the main pane's own cross-fade
/// and the task detail card's mount.
pub const REVEAL: Duration = Duration::from_millis(140);

/// A routine state change acknowledging an action in place: a row fading in
/// on mount or collapsing out on completion, a toast fading in, a disclosure
/// section's rows appearing. Distinct from [`REVEAL`] even though the two
/// currently sit close together, so list-row motion can drift independently
/// of whole-surface motion once real content calls for it.
pub const TRANSITION: Duration = Duration::from_millis(180);

/// How long a side panel takes to slide open or shut. Zeron's panel
/// transition (`crates/ui/src/motion.rs` `RESIZE`) is 200ms — long enough to
/// read as travel rather than a jump cut, short enough that the layout is
/// settled before the pointer arrives anywhere else.
pub const PANEL_SLIDE: Duration = Duration::from_millis(200);

/// How long the "just completed" checkbox pop takes — short and punchy
/// rather than a settle, since it's acknowledging one decisive action, not
/// introducing new content the way [`REVEAL`] does.
pub const POP: Duration = Duration::from_millis(220);

/// A back-out overshoot curve: 0 at `t=0`, past 1.0 partway through, back to
/// exactly 1.0 at `t=1`. The standard "easeOutBack" formula — a cheap,
/// honest stand-in for a real damped spring when only one scalar (a size, a
/// scale factor) is being driven, not full spring physics.
///
/// **Not an easing function** — never pass this to `Animation::
/// with_easing()`. GPUI's own animation driver `debug_assert!`s that an
/// easing function's output stays within `0.0..=1.0` (`elements/animation.rs`
/// checks this on every frame in debug builds, which is exactly what `bun
/// ./scripts/dev.ts` runs), and this curve deliberately violates that by
/// design — the overshoot *is* the pop. Call it from inside a
/// `with_animation` *animator* closure instead, on the delta GPUI already
/// handed you post-easing (already safely `0.0..=1.0`), to shape a concrete
/// style value (a `px()` size, a `Transformation::scale()`) — the same
/// "animate a real box property, not a transform" technique this file
/// already leans on elsewhere for anything a plain `Div` can't scale.
pub fn overshoot(delta: f32) -> f32 {
    const C1: f32 = 1.702;
    const C3: f32 = C1 + 1.0;
    let x = delta - 1.0;
    1.0 + C3 * x * x * x + C1 * x * x
}

/// A one-shot value slide, evaluated from `render` instead of wrapped around
/// an element — generic over whatever f32 is being animated (a panel's
/// width, a segmented control's thumb position, ...).
///
/// `with_animation` cannot drive this in two situations: when the animated
/// value feeds a *sibling's* layout rather than the animated element's own
/// style (a panel's width setting how much room its neighbor gets), or when
/// the value must resume from wherever it currently sits on interruption
/// rather than replay from a fixed endpoint (a toggle reversed mid-slide, or
/// a control switched back to its other state before the first slide
/// finished) — `with_animation` keys its state by element-id path, so a
/// value that has to carry over across a *different* target can't just be
/// re-triggered by mounting a fresh id the way this codebase's opacity
/// reveals do. Evaluating by hand keeps the element tree's shape constant: a
/// finished or dropped tween is exactly the steady state.
#[derive(Clone, Copy, Debug)]
pub struct Tween {
    from: f32,
    started: Instant,
    duration: Duration,
}

impl Tween {
    /// Start a slide from the value the control currently shows, so a
    /// toggle mid-slide reverses from where it actually is instead of
    /// jumping back to the far end.
    pub fn new(from: f32, duration: Duration) -> Self {
        Self {
            from,
            started: Instant::now(),
            duration,
        }
    }

    /// Eased value on the way to `target`, or `None` once the slide is
    /// over — the caller then drops the tween and reads `target` directly,
    /// which is also what retires a finished slide from the element tree
    /// (or, for a persistent control like a segmented thumb, just stops
    /// re-evaluating it every frame).
    pub fn toward(&self, target: f32) -> Option<f32> {
        tween_at(self.from, target, self.started.elapsed(), self.duration)
    }
}

fn tween_at(from: f32, target: f32, elapsed: Duration, duration: Duration) -> Option<f32> {
    let progress = elapsed.as_secs_f32() / duration.as_secs_f32();
    (progress < 1.0).then(|| from + (target - from) * ease_out_quint()(progress.max(0.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slide_eases_out_and_then_retires() {
        let start =
            tween_at(0.0, 260.0, Duration::ZERO, PANEL_SLIDE).expect("a fresh slide is in flight");
        assert!(start.abs() < 0.01, "the slide opens from its start width");

        let half = tween_at(0.0, 260.0, PANEL_SLIDE / 2, PANEL_SLIDE).expect("halfway is in flight");
        assert!(
            half > 130.0,
            "ease-out covers most of the distance early, got {half}"
        );

        assert_eq!(
            tween_at(0.0, 260.0, PANEL_SLIDE, PANEL_SLIDE),
            None,
            "an elapsed slide reports no width so the caller settles on the target"
        );
    }

    #[test]
    fn a_slide_reversed_mid_flight_leaves_from_where_it_is() {
        let interrupted =
            tween_at(0.0, 260.0, PANEL_SLIDE / 4, PANEL_SLIDE).expect("in flight");
        let reversed = tween_at(interrupted, 0.0, Duration::ZERO, PANEL_SLIDE).expect("in flight");
        assert!(
            (reversed - interrupted).abs() < 0.01,
            "the reversed slide starts at the interrupted width"
        );
    }
}
