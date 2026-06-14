use crate::{NativeError, NativeErrorKind};

pub(super) const SECONDS_PER_MINUTE: i64 = 60;
pub(super) const SECONDS_PER_HOUR: i64 = 60 * SECONDS_PER_MINUTE;
pub(super) const SECONDS_PER_DAY: i64 = 24 * SECONDS_PER_HOUR;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct UtcDateTime {
    pub(super) year: i64,
    pub(super) month: i64,
    pub(super) day: i64,
    pub(super) hour: i64,
    pub(super) min: i64,
    pub(super) sec: i64,
    pub(super) yday: i64,
    pub(super) wday: i64,
}

pub(super) fn utc_seconds_from_civil_time(
    mut year: i64,
    month: i64,
    day: i64,
    hour: i64,
    min: i64,
    sec: i64,
) -> Result<i64, NativeError> {
    let month_index = month - 1;
    year += month_index.div_euclid(12);
    let month = month_index.rem_euclid(12) + 1;
    let days = days_from_civil(year, month, 1)
        .checked_add(day - 1)
        .ok_or_else(time_representability_error)?;
    let seconds = i128::from(days) * i128::from(SECONDS_PER_DAY)
        + i128::from(hour) * i128::from(SECONDS_PER_HOUR)
        + i128::from(min) * i128::from(SECONDS_PER_MINUTE)
        + i128::from(sec);
    i64::try_from(seconds).map_err(|_| time_representability_error())
}

pub(super) fn utc_date_time(seconds: i64) -> UtcDateTime {
    let days = seconds.div_euclid(SECONDS_PER_DAY);
    let seconds_of_day = seconds.rem_euclid(SECONDS_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    UtcDateTime {
        year,
        month,
        day,
        hour: seconds_of_day / SECONDS_PER_HOUR,
        min: (seconds_of_day % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE,
        sec: seconds_of_day % SECONDS_PER_MINUTE,
        yday: days - days_from_civil(year, 1, 1) + 1,
        wday: (days + 4).rem_euclid(7) + 1,
    }
}

fn days_from_civil(mut year: i64, month: i64, day: i64) -> i64 {
    year -= i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

pub(super) fn time_representability_error() -> NativeError {
    NativeErrorKind::RuntimeError {
        message: "time result cannot be represented in this installation".into(),
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::{UtcDateTime, utc_date_time, utc_seconds_from_civil_time};

    #[test]
    fn utc_date_time_converts_unix_epoch() {
        assert_eq!(
            utc_date_time(0),
            UtcDateTime {
                year: 1970,
                month: 1,
                day: 1,
                hour: 0,
                min: 0,
                sec: 0,
                yday: 1,
                wday: 5,
            }
        );
    }

    #[test]
    fn utc_date_time_converts_leap_day() {
        let seconds =
            utc_seconds_from_civil_time(2024, 2, 29, 23, 59, 58).expect("valid timestamp");

        assert_eq!(
            utc_date_time(seconds),
            UtcDateTime {
                year: 2024,
                month: 2,
                day: 29,
                hour: 23,
                min: 59,
                sec: 58,
                yday: 60,
                wday: 5,
            }
        );
    }
}
