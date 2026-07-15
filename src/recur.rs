use chrono::{Datelike, Duration, NaiveDate, Weekday};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recur {
    Daily,
    Weekly(Vec<Weekday>),
    EveryDays(u32),
}

pub fn parse(s: &str) -> Option<Recur> {
    if s == "daily" {
        return Some(Recur::Daily);
    }
    if let Some(days) = s.strip_prefix("weekly:") {
        if days.is_empty() {
            return None;
        }
        let wd: Option<Vec<Weekday>> = days.split(',').map(weekday).collect();
        return wd.map(Recur::Weekly);
    }
    if let Some(n) = s.strip_prefix("every:") {
        let n: u32 = n.strip_suffix('d')?.parse().ok()?;
        if n > 0 {
            return Some(Recur::EveryDays(n));
        }
    }
    None
}

fn weekday(s: &str) -> Option<Weekday> {
    Some(match s {
        "mon" => Weekday::Mon,
        "tue" => Weekday::Tue,
        "wed" => Weekday::Wed,
        "thu" => Weekday::Thu,
        "fri" => Weekday::Fri,
        "sat" => Weekday::Sat,
        "sun" => Weekday::Sun,
        _ => return None,
    })
}

/// Next occurrence strictly after `from`. `from` is the completion date (today),
/// per spec: nothing pre-materialized, no backlog of missed occurrences.
pub fn next_after(rule: &Recur, from: NaiveDate) -> NaiveDate {
    match rule {
        Recur::Daily => from + Duration::days(1),
        Recur::EveryDays(n) => from + Duration::days(*n as i64),
        Recur::Weekly(days) => {
            let mut d = from + Duration::days(1);
            while !days.contains(&d.weekday()) {
                d += Duration::days(1);
            }
            d
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Weekday;

    fn d(s: &str) -> chrono::NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn parses_all_forms() {
        assert_eq!(parse("daily"), Some(Recur::Daily));
        assert_eq!(
            parse("weekly:mon,thu"),
            Some(Recur::Weekly(vec![Weekday::Mon, Weekday::Thu]))
        );
        assert_eq!(parse("every:3d"), Some(Recur::EveryDays(3)));
    }

    #[test]
    fn rejects_garbage() {
        for bad in [
            "",
            "weekly:",
            "weekly:funday",
            "every:0d",
            "every:3",
            "monthly",
        ] {
            assert_eq!(parse(bad), None, "should reject {bad:?}");
        }
    }

    #[test]
    fn next_after_daily_and_every() {
        assert_eq!(next_after(&Recur::Daily, d("2026-07-15")), d("2026-07-16"));
        assert_eq!(
            next_after(&Recur::EveryDays(3), d("2026-07-15")),
            d("2026-07-18")
        );
    }

    #[test]
    fn next_after_weekly_picks_next_listed_weekday() {
        // 2026-07-15 is a Wednesday
        let rule = Recur::Weekly(vec![Weekday::Mon, Weekday::Thu]);
        assert_eq!(next_after(&rule, d("2026-07-15")), d("2026-07-16")); // Thu
        assert_eq!(next_after(&rule, d("2026-07-16")), d("2026-07-20")); // next Mon
                                                                         // completing on a listed day moves to the NEXT occurrence, not the same day
        assert_eq!(next_after(&rule, d("2026-07-20")), d("2026-07-23"));
    }
}
