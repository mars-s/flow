//! Deterministic local time-language parsing, per
//! `docs/PRODUCT_REQUIREMENTS.md` §6.4. Pure and offline: no network, no
//! LLM, task titles never leave the process. Every ambiguous or impossible
//! form (a bare "at 8" with no am/pm, "next week", a month/day that has
//! already passed this year, an impossible calendar date) is deliberately
//! left unrecognized rather than guessed — "the system must not guess
//! silently" is PRD principle 3, and this module is where that principle
//! actually gets enforced.
//!
//! Only a phrase at the very end of the title is recognized and stripped,
//! matching the PRD's "only a recognized temporal phrase at the end of the
//! title is removed." `parse` takes `today` as a parameter rather than
//! reading the clock itself, so callers (and every test here) are
//! deterministic regardless of when they run.

use std::sync::LazyLock;

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Weekday};
use regex::Regex;

/// What the parser found, if anything. `cleaned_title` always has the
/// recognized suffix removed and is trimmed; it equals the input unchanged
/// when nothing was recognized.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ParsedTitle {
    pub cleaned_title: String,
    /// The exact substring removed from the title, e.g. "8 am tomorrow" —
    /// kept so a future composer preview/undo can show and restore it
    /// (PRD: "Backspace restores the original text").
    pub source_phrase: Option<String>,
    /// Byte range `source_phrase` occupied in the *input* title (not
    /// `cleaned_title`, which has it removed) — lets a caller highlight the
    /// recognized phrase in place while the user types, e.g. the composer's
    /// live date/time preview.
    pub source_range: Option<std::ops::Range<usize>>,
    pub date: Option<NaiveDate>,
    pub time: Option<NaiveTime>,
}

const MONTH_NAMES: &str = r"jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:tember)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?";

const DATE_GROUP: &str = r"(?:today|tomorrow|in\s+\d{1,3}\s+days?|(?:next\s+)?(?:monday|tuesday|wednesday|thursday|friday|saturday|sunday)|\d{4}-\d{2}-\d{2}|(?:MONTHS)\.?\s+\d{1,2}(?:st|nd|rd|th)?|\d{1,2}(?:st|nd|rd|th)?\s+(?:MONTHS)\.?)";
const TIME_GROUP: &str = r"(?:\d{1,2}(?::\d{2})?\s*(?:am|pm)|(?:[01]?\d|2[0-3]):[0-5]\d)";

fn date_group() -> String {
    DATE_GROUP.replace("MONTHS", MONTH_NAMES)
}

/// The four shapes a trailing phrase can take, tried longest-first so a
/// combined match (date + time) always wins over a partial one.
static DATE_THEN_TIME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)\b(?P<date>{})\s+(?P<time>{TIME_GROUP})$",
        date_group()
    ))
    .expect("DATE_THEN_TIME pattern")
});
static TIME_THEN_DATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)\b(?P<time>{TIME_GROUP})\s+(?P<date>{})$",
        date_group()
    ))
    .expect("TIME_THEN_DATE pattern")
});
static DATE_ONLY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"(?i)\b(?P<date>{})$", date_group())).expect("DATE_ONLY pattern")
});
static TIME_ONLY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"(?i)\b(?P<time>{TIME_GROUP})$")).expect("TIME_ONLY pattern")
});

/// Parses `title` against `today` (the caller's local calendar day).
/// `today` should come from `chrono::Local::now().date_naive()` in
/// production and a fixed date in tests, so results are reproducible.
pub fn parse(title: &str, today: NaiveDate) -> ParsedTitle {
    let trimmed = title.trim_end();

    for (regex, wants_date, wants_time) in [
        (&*DATE_THEN_TIME, true, true),
        (&*TIME_THEN_DATE, true, true),
        (&*DATE_ONLY, true, false),
        (&*TIME_ONLY, false, true),
    ] {
        let Some(caps) = regex.captures(trimmed) else {
            continue;
        };
        let date = wants_date
            .then(|| caps.name("date").and_then(|m| parse_date_phrase(m.as_str(), today)))
            .flatten();
        let time = wants_time
            .then(|| caps.name("time").and_then(|m| parse_time_phrase(m.as_str())))
            .flatten();
        // A phrase that matched the pattern but failed its own semantic
        // check (an impossible date, a month/day already past with no
        // explicit year) is ambiguous, not a partial match — PRD principle
        // 3 says don't guess, so fall through to unrecognized rather than
        // salvaging whichever half did parse.
        if wants_date && date.is_none() {
            continue;
        }
        if wants_time && time.is_none() {
            continue;
        }
        let whole = caps.get(0).expect("group 0 always matches");
        return ParsedTitle {
            cleaned_title: trimmed[..whole.start()].trim_end().to_string(),
            source_phrase: Some(whole.as_str().to_string()),
            source_range: Some(whole.start()..whole.end()),
            date,
            time,
        };
    }

    ParsedTitle {
        cleaned_title: trimmed.to_string(),
        ..Default::default()
    }
}

static RE_TODAY_TOMORROW: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(today|tomorrow)$").unwrap());
static RE_IN_DAYS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^in\s+(\d{1,3})\s+days?$").unwrap());
static RE_WEEKDAY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(next\s+)?(monday|tuesday|wednesday|thursday|friday|saturday|sunday)$")
        .unwrap()
});
static RE_ISO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{4})-(\d{2})-(\d{2})$").unwrap());
static RE_MONTH_DAY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)^({MONTH_NAMES})\.?\s+(\d{{1,2}})(?:st|nd|rd|th)?$"
    ))
    .unwrap()
});
static RE_DAY_MONTH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)^(\d{{1,2}})(?:st|nd|rd|th)?\s+({MONTH_NAMES})\.?$"
    ))
    .unwrap()
});

fn parse_date_phrase(phrase: &str, today: NaiveDate) -> Option<NaiveDate> {
    let phrase = phrase.trim();

    if let Some(caps) = RE_TODAY_TOMORROW.captures(phrase) {
        return match caps[1].to_lowercase().as_str() {
            "today" => Some(today),
            _ => Some(today + Duration::days(1)),
        };
    }

    if let Some(caps) = RE_IN_DAYS.captures(phrase) {
        let n: u32 = caps[1].parse().ok()?;
        if !(1..=365).contains(&n) {
            return None;
        }
        return Some(today + Duration::days(i64::from(n)));
    }

    if let Some(caps) = RE_WEEKDAY.captures(phrase) {
        let forced_next_week = caps.get(1).is_some();
        let target = weekday_from_name(&caps[2])?;
        return Some(next_weekday(today, target, forced_next_week));
    }

    if let Some(caps) = RE_ISO.captures(phrase) {
        let year: i32 = caps[1].parse().ok()?;
        let month: u32 = caps[2].parse().ok()?;
        let day: u32 = caps[3].parse().ok()?;
        return NaiveDate::from_ymd_opt(year, month, day);
    }

    if let Some(caps) = RE_MONTH_DAY.captures(phrase) {
        let month = month_from_name(&caps[1])?;
        let day: u32 = caps[2].parse().ok()?;
        return date_this_year_or_ambiguous(today, month, day);
    }

    if let Some(caps) = RE_DAY_MONTH.captures(phrase) {
        let day: u32 = caps[1].parse().ok()?;
        let month = month_from_name(&caps[2])?;
        return date_this_year_or_ambiguous(today, month, day);
    }

    None
}

/// A month/day phrase carries no year, so it defaults to the current one —
/// unless that date has already passed, which PRD §6.4 calls out as
/// ambiguous rather than something to roll forward to next year silently.
fn date_this_year_or_ambiguous(today: NaiveDate, month: u32, day: u32) -> Option<NaiveDate> {
    let candidate = NaiveDate::from_ymd_opt(today.year(), month, day)?;
    (candidate >= today).then_some(candidate)
}

fn next_weekday(today: NaiveDate, target: Weekday, forced_next_week: bool) -> NaiveDate {
    let today_index = today.weekday().num_days_from_monday() as i64;
    let target_index = target.num_days_from_monday() as i64;
    let mut delta = (target_index - today_index).rem_euclid(7);
    if delta == 0 && forced_next_week {
        delta = 7;
    }
    today + Duration::days(delta)
}

fn weekday_from_name(name: &str) -> Option<Weekday> {
    match name.to_lowercase().as_str() {
        "monday" => Some(Weekday::Mon),
        "tuesday" => Some(Weekday::Tue),
        "wednesday" => Some(Weekday::Wed),
        "thursday" => Some(Weekday::Thu),
        "friday" => Some(Weekday::Fri),
        "saturday" => Some(Weekday::Sat),
        "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

fn month_from_name(name: &str) -> Option<u32> {
    let lower = name.to_lowercase();
    let abbreviation = &lower[..3.min(lower.len())];
    let months = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    months
        .iter()
        .position(|m| *m == abbreviation)
        .map(|index| index as u32 + 1)
}

static RE_TIME_12H: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(\d{1,2})(?::(\d{2}))?\s*(am|pm)$").unwrap());
static RE_TIME_24H: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([01]?\d|2[0-3]):([0-5]\d)$").unwrap());

fn parse_time_phrase(phrase: &str) -> Option<NaiveTime> {
    let phrase = phrase.trim();

    if let Some(caps) = RE_TIME_12H.captures(phrase) {
        let mut hour: u32 = caps[1].parse().ok()?;
        if !(1..=12).contains(&hour) {
            return None;
        }
        let minute: u32 = caps
            .get(2)
            .map(|m| m.as_str().parse().ok())
            .unwrap_or(Some(0))?;
        let is_pm = caps[3].eq_ignore_ascii_case("pm");
        if hour == 12 {
            hour = 0;
        }
        if is_pm {
            hour += 12;
        }
        return NaiveTime::from_hms_opt(hour, minute, 0);
    }

    if let Some(caps) = RE_TIME_24H.captures(phrase) {
        let hour: u32 = caps[1].parse().ok()?;
        let minute: u32 = caps[2].parse().ok()?;
        return NaiveTime::from_hms_opt(hour, minute, 0);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn time(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    /// PRD §6.4's exact acceptance case, 2026-08-18 in Australia/Melbourne
    /// (chrono handles the local-vs-UTC distinction upstream of this
    /// function — `parse` only ever sees an already-local `today`).
    #[test]
    fn prd_acceptance_case_time_then_date() {
        let title = "take out laundry 8 am tomorrow";
        let result = parse(title, date(2026, 8, 18));
        assert_eq!(result.cleaned_title, "take out laundry");
        assert_eq!(result.date, Some(date(2026, 8, 19)));
        assert_eq!(result.time, Some(time(8, 0)));
        assert_eq!(result.source_phrase.as_deref(), Some("8 am tomorrow"));
        let range = result.source_range.expect("a match should carry a range");
        assert_eq!(&title[range], "8 am tomorrow");
    }

    #[test]
    fn source_range_points_at_the_phrase_in_the_original_title() {
        let title = "call mom tomorrow 5pm";
        let result = parse(title, date(2026, 8, 18));
        let range = result.source_range.expect("a match should carry a range");
        assert_eq!(&title[range.clone()], result.source_phrase.as_deref().unwrap());
        assert_eq!(&title[range], "tomorrow 5pm");
    }

    #[test]
    fn no_match_leaves_source_range_empty() {
        let result = parse("Just a plain title", date(2026, 8, 18));
        assert_eq!(result.source_range, None);
    }

    #[test]
    fn prd_acceptance_case_in_n_days() {
        let result = parse("bring Mya cake in 3 days", date(2026, 8, 18));
        assert_eq!(result.cleaned_title, "bring Mya cake");
        assert_eq!(result.date, Some(date(2026, 8, 21)));
        assert_eq!(result.time, None);
    }

    #[test]
    fn today_and_tomorrow() {
        assert_eq!(parse("water plants today", date(2026, 8, 18)).date, Some(date(2026, 8, 18)));
        assert_eq!(
            parse("water plants tomorrow", date(2026, 8, 18)).date,
            Some(date(2026, 8, 19))
        );
    }

    #[test]
    fn date_then_time_order_also_works() {
        let result = parse("call mom tomorrow 5pm", date(2026, 8, 18));
        assert_eq!(result.cleaned_title, "call mom");
        assert_eq!(result.date, Some(date(2026, 8, 19)));
        assert_eq!(result.time, Some(time(17, 0)));
    }

    #[test]
    fn explicit_dates_all_three_forms() {
        assert_eq!(parse("renew passport Aug 23", date(2026, 1, 1)).date, Some(date(2026, 8, 23)));
        assert_eq!(parse("renew passport 23 Aug", date(2026, 1, 1)).date, Some(date(2026, 8, 23)));
        assert_eq!(
            parse("renew passport 2026-08-23", date(2026, 1, 1)).date,
            Some(date(2026, 8, 23))
        );
    }

    #[test]
    fn bare_weekday_can_mean_today() {
        // 2026-08-17 is a Monday.
        let monday = date(2026, 8, 17);
        assert_eq!(parse("standup monday", monday).date, Some(monday));
    }

    #[test]
    fn next_weekday_skips_to_the_following_week_when_today_matches() {
        let monday = date(2026, 8, 17);
        assert_eq!(
            parse("standup next monday", monday).date,
            Some(monday + Duration::days(7))
        );
    }

    #[test]
    fn next_weekday_behaves_like_bare_weekday_when_today_does_not_match() {
        // 2026-08-18 is a Tuesday; the next Monday either way is the 24th.
        let tuesday = date(2026, 8, 18);
        assert_eq!(parse("standup monday", tuesday).date, Some(date(2026, 8, 24)));
        assert_eq!(parse("standup next monday", tuesday).date, Some(date(2026, 8, 24)));
    }

    #[test]
    fn time_forms() {
        assert_eq!(parse("call at 08:30", date(2026, 8, 18)).time, Some(time(8, 30)));
        assert_eq!(parse("call at 8:30 pm", date(2026, 8, 18)).time, Some(time(20, 30)));
        assert_eq!(parse("call at 12am", date(2026, 8, 18)).time, Some(time(0, 0)));
        assert_eq!(parse("call at 12pm", date(2026, 8, 18)).time, Some(time(12, 0)));
    }

    #[test]
    fn bare_hour_without_am_pm_is_ambiguous_and_left_alone() {
        let result = parse("call at 8", date(2026, 8, 18));
        assert_eq!(result.cleaned_title, "call at 8");
        assert_eq!(result.date, None);
        assert_eq!(result.time, None);
    }

    #[test]
    fn next_week_is_not_recognized() {
        let result = parse("plan trip next week", date(2026, 8, 18));
        assert_eq!(result.cleaned_title, "plan trip next week");
        assert_eq!(result.date, None);
    }

    #[test]
    fn a_month_day_already_past_this_year_is_ambiguous_not_rolled_to_next_year() {
        // "Jan 3" evaluated on Dec 30 has already passed this year; PRD
        // says don't silently assume next year.
        let result = parse("file taxes jan 3", date(2026, 12, 30));
        assert_eq!(result.cleaned_title, "file taxes jan 3");
        assert_eq!(result.date, None);
    }

    #[test]
    fn an_impossible_date_is_not_recognized() {
        let result = parse("plan party feb 30", date(2026, 1, 1));
        assert_eq!(result.cleaned_title, "plan party feb 30");
        assert_eq!(result.date, None);
    }

    #[test]
    fn in_days_out_of_range_is_not_recognized() {
        let result = parse("someday task in 400 days", date(2026, 8, 18));
        assert_eq!(result.date, None);
        assert_eq!(result.cleaned_title, "someday task in 400 days");
    }

    #[test]
    fn no_recognized_phrase_leaves_the_title_untouched() {
        let result = parse("Just a plain title", date(2026, 8, 18));
        assert_eq!(result.cleaned_title, "Just a plain title");
        assert_eq!(result.source_phrase, None);
        assert_eq!(result.date, None);
        assert_eq!(result.time, None);
    }

    #[test]
    fn only_the_trailing_phrase_is_stripped_not_an_earlier_occurrence() {
        // "today" earlier in the sentence is not a recognized suffix; only
        // the trailing "tomorrow" is.
        let result = parse("today's plan tomorrow", date(2026, 8, 18));
        assert_eq!(result.cleaned_title, "today's plan");
        assert_eq!(result.date, Some(date(2026, 8, 19)));
    }
}
