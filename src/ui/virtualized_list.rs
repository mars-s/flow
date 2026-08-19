//! Proof that GPUI's own `list()`/`ListState` — the primitive
//! `CLAUDE.md`'s performance section names ("Long collections are
//! virtualized with `list()`") — actually bounds per-frame work to the
//! visible window regardless of total item count. `app::tasks` wires the
//! real task rows to this same primitive directly; this module exists to
//! keep that claim honest with a real, running test rather than trusting
//! it by inspection.
//!
//! This is deliberately the one exception to `app/tasks.rs`'s own stated
//! rule ("this codebase doesn't unit-test GPUI element construction") —
//! that rule is about not snapshot-testing what a row looks like, which
//! stays true here (nothing below asserts on pixels or content). What's
//! tested is a mechanism guarantee: does the list actually call its
//! per-item render closure only for items near the viewport, or does
//! scaling the item count scale the render cost with it. That's a real
//! regression risk (an innocent-looking change to how items are supplied
//! can silently make a list eager again) and worth a running assertion,
//! not a design note nobody re-checks.

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use gpui::{
        Context, IntoElement, ListAlignment, ListOffset, ListState, Render, TestAppContext,
        Window, div, list, point, prelude::*, px, size,
    };

    const ITEM_COUNT: usize = 10_000;
    const ROW_HEIGHT: f32 = 40.0;
    /// A generous viewport plus GPUI's own overdraw margin (`ListState::new`'s
    /// third argument) should still land well under this — the assertion
    /// exists to catch "became eager again", not to pin an exact count that
    /// would need updating every time GPUI's overdraw heuristic changes.
    const MAX_PLAUSIBLE_RENDERS: usize = 200;

    /// A minimal `Render` host for the list — `list()` needs to live inside
    /// a view's `render()` (its state is meant to be held "intrusively" on
    /// one, per its own module doc), not drawn as a bare element the way a
    /// stateless `div()` can be.
    struct TenThousandRows {
        state: ListState,
        render_calls: Rc<Cell<usize>>,
    }

    impl Render for TenThousandRows {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let render_calls = self.render_calls.clone();
            list(self.state.clone(), move |ix, _window, _cx| {
                render_calls.set(render_calls.get() + 1);
                // A row cheap enough that if this test ever regressed into
                // rendering all 10,000, it would still finish — the count
                // assertion is what actually catches the regression, not a
                // timeout.
                div()
                    .h(px(ROW_HEIGHT))
                    .w_full()
                    .child(format!("row {ix}"))
                    .into_any_element()
            })
            .w_full()
            .h_full()
        }
    }

    #[gpui::test]
    fn ten_thousand_items_only_render_the_visible_window(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let render_calls = Rc::new(Cell::new(0));
        let state = ListState::new(ITEM_COUNT, ListAlignment::Top, px(200.0));

        let calls = render_calls.clone();
        cx.draw(
            point(px(0.0), px(0.0)),
            size(px(400.0), px(800.0)),
            move |_, cx| {
                cx.new(|_| TenThousandRows { state, render_calls: calls })
                    .into_any_element()
            },
        );

        let calls = render_calls.get();
        assert!(calls > 0, "expected the visible rows to render at all");
        assert!(
            calls < MAX_PLAUSIBLE_RENDERS,
            "expected only the ~800px/{ROW_HEIGHT}px viewport (plus overdraw) to render out \
             of {ITEM_COUNT} items, got {calls} render_item calls — the list stopped \
             virtualizing"
        );
    }

    #[gpui::test]
    fn scrolling_deep_into_ten_thousand_items_still_only_renders_the_window(
        cx: &mut TestAppContext,
    ) {
        let cx = cx.add_empty_window();
        let render_calls = Rc::new(Cell::new(0));
        let state = ListState::new(ITEM_COUNT, ListAlignment::Top, px(200.0));
        // Deep into the list, not the first screenful — proves scrolling
        // through 10,000 items stays cheap throughout, not just at the top
        // where an implementation could special-case the initial paint.
        state.scroll_to(ListOffset { item_ix: 9_000, offset_in_item: px(0.0) });

        let calls = render_calls.clone();
        cx.draw(
            point(px(0.0), px(0.0)),
            size(px(400.0), px(800.0)),
            move |_, cx| {
                cx.new(|_| TenThousandRows { state, render_calls: calls })
                    .into_any_element()
            },
        );

        let calls = render_calls.get();
        assert!(
            calls > 0,
            "expected the rows around item 9,000 to render at all"
        );
        assert!(
            calls < MAX_PLAUSIBLE_RENDERS,
            "expected only the window around item 9,000 to render, not everything scrolled \
             past to get there — got {calls} render_item calls"
        );
    }
}
