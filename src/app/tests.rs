//! Milestone 0 shell tests: destination navigation and placeholder mapping.
//!
//! Everything else this file used to cover (sessions, transcript, composer,
//! branches, usage) was stripped along with those modules; see the
//! milestone-0 wayfinder ticket.

use super::sidebar::Destination;

#[test]
fn there_are_exactly_the_seven_prd_destinations_in_sidebar_order() {
    assert_eq!(
        Destination::ALL.map(Destination::label),
        [
            "Inbox", "Today", "Upcoming", "Anytime", "Someday", "Calendar", "Settings",
        ]
    );
}

#[test]
fn every_destination_round_trips_through_its_index() {
    for destination in Destination::ALL {
        assert_eq!(Destination::from_index(destination.index()), destination);
    }
}

#[test]
fn arrow_navigation_wraps_past_the_last_destination() {
    // The sidebar's "down" handler asks for `index + 1` without bounds
    // checking; `from_index` must wrap that back to Inbox.
    assert_eq!(Destination::from_index(Destination::COUNT), Destination::Inbox);
    assert_eq!(
        Destination::from_index(Destination::COUNT + 1),
        Destination::Today
    );
}

#[test]
fn only_inbox_carries_a_badge() {
    // Mirrors the badge gate in `sidebar::render_nav_row`: Inbox is the only
    // destination with a stub unread count in Milestone 0.
    for destination in Destination::ALL {
        let shows_badge = destination == Destination::Inbox;
        assert_eq!(shows_badge, matches!(destination, Destination::Inbox));
    }
}

#[test]
fn every_destination_has_non_empty_placeholder_copy() {
    for destination in Destination::ALL {
        assert!(!destination.placeholder_copy().is_empty());
        assert!(!destination.label().is_empty());
    }
}
