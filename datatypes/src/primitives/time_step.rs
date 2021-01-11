use std::{cmp::max, ops::Add};

use chrono::{Datelike, Duration, NaiveDate};
use error::Error::NoDateTimeValid;

use crate::error;
use crate::primitives::TimeInstance;
use crate::util::Result;

use super::TimeInterval;

/// A time granularity.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TimeGranularity {
    Seconds,
    Minutes,
    Hours,
    Days,
    Months,
    Years,
}

/// A step in time.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TimeStep {
    pub granularity: TimeGranularity,
    pub step: u32,
}

impl TimeStep {
    /// Resolves how many 'TimeSteps' fit into a given 'TimeInterval'.
    /// Remember that 'TimeInterval' is not inclusive.
    ///
    /// # Errors
    /// This method uses chrono and therefore fails if a 'TimeInstance' is outside chronos valid date range.
    ///
    pub fn num_steps_in_interval(&self, time_interval: TimeInterval) -> Result<u32> {
        let end = time_interval
            .end()
            .as_naive_date_time()
            .ok_or(NoDateTimeValid {
                time_instance: time_interval.end(),
            })?;
        let start = time_interval
            .start()
            .as_naive_date_time()
            .ok_or(NoDateTimeValid {
                time_instance: time_interval.start(),
            })?;

        let duration = end - start;

        if duration.is_zero() {
            return Ok(0);
        }

        let num_steps: i64 = match self.granularity {
            TimeGranularity::Seconds => {
                let s = duration.num_seconds() / self.step as i64;
                if (duration - Duration::seconds(s * self.step as i64)).is_zero() {
                    s - 1
                } else {
                    s
                }
            }
            TimeGranularity::Minutes => {
                let s = duration.num_minutes() / self.step as i64;
                if (duration - Duration::minutes(s * self.step as i64)).is_zero() {
                    s - 1
                } else {
                    s
                }
            }
            TimeGranularity::Hours => {
                let s = duration.num_hours() / self.step as i64;
                if (duration - Duration::hours(s * self.step as i64)).is_zero() {
                    s - 1
                } else {
                    s
                }
            }
            TimeGranularity::Days => {
                let s = duration.num_days() / self.step as i64;
                if (duration - Duration::days(s * self.step as i64)).is_zero() {
                    s - 1
                } else {
                    s
                }
            }
            TimeGranularity::Months => {
                let diff_years = (end.year() - start.year()) as i64;
                let diff_months = (end.month() as i64 - start.month() as i64) + diff_years * 12;
                let steps = diff_months / self.step as i64;

                let shifted_start = (time_interval.start()
                    + TimeStep {
                        granularity: TimeGranularity::Months,
                        step: self.step * steps as u32,
                    })
                .expect("is in valid range");

                if (end
                    - shifted_start
                        .as_naive_date_time()
                        .expect("is in valid range"))
                .is_zero()
                {
                    steps - 1
                } else {
                    steps
                }
            }
            TimeGranularity::Years => {
                let steps = (end.year() - start.year()) as i64 / self.step as i64;

                let shifted_start = start
                    .with_year(start.year() + (self.step as i64 * steps) as i32)
                    .expect("is in valid range");

                if (end - shifted_start).is_zero() {
                    steps - 1
                } else {
                    steps
                }
            }
        };

        Ok(max(0, num_steps as u32))
    }

    /// Snaps a 'TimeInstance' relative to a given reference 'TimeInstance'.
    ///
    /// # Errors
    /// This method uses chrono and therefore fails if a 'TimeInstance' is outside chronos valid date range.
    ///
    pub fn snap_relative(
        &self,
        reference: TimeInstance,
        time_to_snap: TimeInstance,
    ) -> Result<TimeInstance> {
        let ref_date_time = reference.as_naive_date_time().ok_or(NoDateTimeValid {
            time_instance: reference,
        })?;
        let time_to_snap_date_time = time_to_snap.as_naive_date_time().ok_or(NoDateTimeValid {
            time_instance: time_to_snap,
        })?;

        let snapped_date_time = match self.granularity {
            TimeGranularity::Seconds => {
                let diff_duration = time_to_snap_date_time - ref_date_time;
                let snapped_hours =
                    (diff_duration.num_seconds() / self.step as i64) * self.step as i64;
                ref_date_time + Duration::seconds(snapped_hours)
            }
            TimeGranularity::Minutes => {
                let diff_duration = time_to_snap_date_time - ref_date_time;
                let snapped_hours =
                    (diff_duration.num_minutes() / self.step as i64) * self.step as i64;
                ref_date_time + Duration::minutes(snapped_hours)
            }
            TimeGranularity::Hours => {
                let diff_duration = time_to_snap_date_time - ref_date_time;
                let snapped_hours =
                    (diff_duration.num_hours() / self.step as i64) * self.step as i64;
                ref_date_time + Duration::hours(snapped_hours)
            }
            TimeGranularity::Days => {
                let diff_duration = time_to_snap_date_time - ref_date_time;
                let snapped_days = (diff_duration.num_days() / self.step as i64) * self.step as i64;
                ref_date_time + Duration::days(snapped_days)
            }
            TimeGranularity::Months => {
                // first, calculate the total difference in months
                let diff_months = (time_to_snap_date_time.year() - ref_date_time.year()) * 12
                    + (time_to_snap_date_time.month() as i32 - ref_date_time.month() as i32);

                // get the difference in time steps
                let snapped_months = (diff_months / self.step as i32) * self.step as i32;

                let (snapped_year, snapped_month) = if diff_months.is_negative() {
                    // if difference is negative, go one year more back in any case
                    let snapped_year = ref_date_time.year() + (snapped_months / 12) as i32 - 1;
                    // calculate the month, avoid negative values and values > 12
                    let snapped_month =
                        (ref_date_time.month() as i32 + 12 + (snapped_months % 12)) % 12;

                    (snapped_year, snapped_month)
                } else {
                    let snapped_year = ref_date_time.year() + (snapped_months / 12) as i32;

                    let snapped_month = ref_date_time.month() as i32 + snapped_months % 12;

                    (snapped_year, snapped_month)
                };

                NaiveDate::from_ymd(snapped_year, snapped_month as u32, ref_date_time.day())
                    .and_time(ref_date_time.time())
            }
            TimeGranularity::Years => {
                let diff = (time_to_snap_date_time.year() - ref_date_time.year()) as i32;
                let snapped_year =
                    ref_date_time.year() + ((diff / self.step as i32) * self.step as i32);

                NaiveDate::from_ymd(snapped_year, ref_date_time.month(), ref_date_time.day())
                    .and_time(ref_date_time.time())
            }
        };

        Ok(TimeInstance::from(snapped_date_time))
    }
}

impl Add<TimeStep> for TimeInstance {
    type Output = Result<TimeInstance>;

    fn add(self, rhs: TimeStep) -> Self::Output {
        let date_time = self.as_naive_date_time().ok_or(NoDateTimeValid {
            time_instance: self,
        })?;

        let res_date_time = match rhs.granularity {
            TimeGranularity::Seconds => date_time + Duration::seconds(rhs.step as i64),
            TimeGranularity::Minutes => date_time + Duration::minutes(rhs.step as i64),
            TimeGranularity::Hours => date_time + Duration::hours(rhs.step as i64),
            TimeGranularity::Days => date_time + Duration::days(rhs.step as i64),
            TimeGranularity::Months => {
                let months = date_time.month0() + rhs.step as u32;
                let month = months % 12 + 1;
                let years_from_months = (months / 12) as i32;
                let year = date_time.year() + years_from_months;
                NaiveDate::from_ymd(year, month, date_time.day()).and_time(date_time.time())
            }
            TimeGranularity::Years => {
                let year = date_time.year() + rhs.step as i32;
                NaiveDate::from_ymd(year, date_time.month(), date_time.day())
                    .and_time(date_time.time())
            }
        };

        Ok(TimeInstance::from(res_date_time))
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;

    use super::*;

    fn test_snap(
        granularity: TimeGranularity,
        t_step: u32,
        t_start: &str,
        t_in: &str,
        t_expect: &str,
    ) -> () {
        let t_ref = TimeInstance::from(
            NaiveDateTime::parse_from_str(t_start, "%Y-%m-%dT%H:%M:%S").unwrap(),
        );
        let t_1 =
            TimeInstance::from(NaiveDateTime::parse_from_str(t_in, "%Y-%m-%dT%H:%M:%S").unwrap());
        let t_exp = TimeInstance::from(
            NaiveDateTime::parse_from_str(t_expect, "%Y-%m-%dT%H:%M:%S").unwrap(),
        );

        let time_snapper = TimeStep {
            granularity,
            step: t_step,
        };

        assert_eq!(time_snapper.snap_relative(t_ref, t_1).unwrap(), t_exp)
    }

    fn test_num_steps(
        granularity: TimeGranularity,
        t_step: u32,
        t_1: &str,
        t_2: &str,
        steps_expect: u32,
    ) -> () {
        let t_1 =
            TimeInstance::from(NaiveDateTime::parse_from_str(t_1, "%Y-%m-%dT%H:%M:%S").unwrap());
        let t_2 =
            TimeInstance::from(NaiveDateTime::parse_from_str(t_2, "%Y-%m-%dT%H:%M:%S").unwrap());

        let time_snapper = TimeStep {
            granularity,
            step: t_step,
        };

        assert_eq!(
            time_snapper
                .num_steps_in_interval(TimeInterval::new_unchecked(t_1, t_2))
                .unwrap(),
            steps_expect
        )
    }

    fn test_add(granularity: TimeGranularity, t_step: u32, t_1: &str, t_expect: &str) -> () {
        let t_1 =
            TimeInstance::from(NaiveDateTime::parse_from_str(t_1, "%Y-%m-%dT%H:%M:%S").unwrap());
        let t_expect = TimeInstance::from(
            NaiveDateTime::parse_from_str(t_expect, "%Y-%m-%dT%H:%M:%S").unwrap(),
        );

        let time_step = TimeStep {
            granularity,
            step: t_step,
        };

        assert_eq!((t_1 + time_step).unwrap(), t_expect)
    }

    #[test]
    fn test_add_y_0() {
        test_add(
            TimeGranularity::Years,
            0,
            "2000-01-01T00:00:00",
            "2000-01-01T00:00:00",
        )
    }

    #[test]
    fn test_add_y_1() {
        test_add(
            TimeGranularity::Years,
            1,
            "2000-01-01T00:00:00",
            "2001-01-01T00:00:00",
        )
    }

    #[test]
    fn test_add_m_0() {
        test_add(
            TimeGranularity::Months,
            0,
            "2000-01-01T00:00:00",
            "2000-01-01T00:00:00",
        )
    }

    #[test]
    fn test_add_m_1() {
        test_add(
            TimeGranularity::Months,
            1,
            "2000-01-01T00:00:00",
            "2000-02-01T00:00:00",
        )
    }

    #[test]
    fn test_add_m_11() {
        test_add(
            TimeGranularity::Months,
            11,
            "2000-01-01T00:00:00",
            "2000-12-01T00:00:00",
        )
    }

    #[test]
    fn test_add_m_12() {
        test_add(
            TimeGranularity::Months,
            12,
            "2000-01-01T00:00:00",
            "2001-01-01T00:00:00",
        )
    }

    #[test]
    fn test_add_d_0() {
        test_add(
            TimeGranularity::Days,
            0,
            "2000-01-01T00:00:00",
            "2000-01-01T00:00:00",
        )
    }

    #[test]
    fn test_add_d_1() {
        test_add(
            TimeGranularity::Days,
            1,
            "2000-01-01T00:00:00",
            "2000-01-02T00:00:00",
        )
    }

    #[test]
    fn test_add_d_31() {
        test_add(
            TimeGranularity::Days,
            31,
            "2000-01-01T00:00:00",
            "2000-02-01T00:00:00",
        )
    }

    #[test]
    fn test_add_h_0() {
        test_add(
            TimeGranularity::Hours,
            0,
            "2000-01-01T00:00:00",
            "2000-01-01T00:00:00",
        )
    }

    #[test]
    fn test_add_h_1() {
        test_add(
            TimeGranularity::Hours,
            1,
            "2000-01-01T00:00:00",
            "2000-01-01T01:00:00",
        )
    }

    #[test]
    fn test_add_h_24() {
        test_add(
            TimeGranularity::Hours,
            24,
            "2000-01-01T00:00:00",
            "2000-01-02T00:00:00",
        )
    }

    #[test]
    fn test_add_min_0() {
        test_add(
            TimeGranularity::Minutes,
            0,
            "2000-01-01T00:00:00",
            "2000-01-01T00:00:00",
        )
    }

    #[test]
    fn test_add_min_1() {
        test_add(
            TimeGranularity::Minutes,
            1,
            "2000-01-01T00:00:00",
            "2000-01-01T00:01:00",
        )
    }

    #[test]
    fn test_add_min_60() {
        test_add(
            TimeGranularity::Minutes,
            60,
            "2000-01-01T00:00:00",
            "2000-01-01T01:00:00",
        )
    }

    #[test]
    fn test_add_s_0() {
        test_add(
            TimeGranularity::Seconds,
            0,
            "2000-01-01T00:00:00",
            "2000-01-01T00:00:00",
        )
    }

    #[test]
    fn test_add_s_1() {
        test_add(
            TimeGranularity::Seconds,
            1,
            "2000-01-01T00:00:00",
            "2000-01-01T00:00:01",
        )
    }

    #[test]
    fn test_add_s_60() {
        test_add(
            TimeGranularity::Seconds,
            60,
            "2000-01-01T00:00:00",
            "2000-01-01T00:01:00",
        )
    }

    #[test]
    fn time_snap_month_n1() {
        test_snap(
            TimeGranularity::Months,
            1,
            "2000-01-01T00:00:00",
            "1999-11-01T00:00:00",
            "1999-11-01T00:00:00",
        );
    }

    #[test]
    fn time_snap_month_1() {
        test_snap(
            TimeGranularity::Months,
            1,
            "2000-01-01T00:00:00",
            "2000-11-11T11:11:11",
            "2000-11-01T00:00:00",
        );
    }

    #[test]
    fn time_snap_month_3() {
        test_snap(
            TimeGranularity::Months,
            3,
            "2000-01-01T00:00:00",
            "2000-11-11T11:11:11",
            "2000-10-01T00:00:00",
        );
    }

    #[test]
    fn time_snap_month_7() {
        test_snap(
            TimeGranularity::Months,
            7,
            "2000-01-01T00:00:00",
            "2001-01-01T11:11:11",
            "2000-08-01T00:00:00",
        );
    }

    #[test]
    fn time_snap_year_1() {
        test_snap(
            TimeGranularity::Years,
            1,
            "2010-01-01T00:00:00",
            "2014-01-03T01:01:00",
            "2014-01-01T00:00:00",
        );
    }

    #[test]
    fn time_snap_year_3() {
        test_snap(
            TimeGranularity::Years,
            3,
            "2010-01-01T00:00:00",
            "2014-01-03T01:01:00",
            "2013-01-01T00:00:00",
        );
    }

    #[test]
    fn time_snap_year_3_2() {
        test_snap(
            TimeGranularity::Years,
            3,
            "2010-01-01T00:02:00",
            "2014-01-03T01:01:00",
            "2013-01-01T00:02:00",
        );
    }

    #[test]
    fn time_snap_day_1() {
        test_snap(
            TimeGranularity::Days,
            1,
            "2010-01-01T00:00:00",
            "2013-01-01T01:00:00",
            "2013-01-01T00:00:00",
        );
    }

    #[test]
    fn time_snap_day_1_2() {
        test_snap(
            TimeGranularity::Days,
            1,
            "2010-01-01T00:02:03",
            "2013-01-01T00:00:00",
            "2012-12-31T00:02:03",
        );
    }

    #[test]
    fn time_snap_day_16() {
        test_snap(
            TimeGranularity::Days,
            16,
            "2018-01-01T00:00:00",
            "2018-02-16T01:00:00",
            "2018-02-02T00:00:00",
        );
    }

    #[test]
    fn time_snap_hour_1() {
        test_snap(
            TimeGranularity::Hours,
            1,
            "2010-01-01T00:00:00",
            "2013-01-01T01:12:00",
            "2013-01-01T01:00:00",
        );
    }

    #[test]
    fn time_snap_hour_13() {
        test_snap(
            TimeGranularity::Hours,
            13,
            "2010-01-01T00:00:00",
            "2010-01-02T04:00:00",
            "2010-01-02T02:00:00",
        );
    }

    #[test]
    fn time_snap_hour_13_2() {
        test_snap(
            TimeGranularity::Hours,
            13,
            "2010-01-01T00:00:01",
            "2010-01-02T01:00:02",
            "2010-01-01T13:00:01",
        );
    }

    #[test]
    fn time_snap_minute_1() {
        test_snap(
            TimeGranularity::Minutes,
            1,
            "2010-01-01T00:00:00",
            "2013-01-01T01:12:00",
            "2013-01-01T01:12:00",
        );
    }

    #[test]
    fn time_snap_minute_1_2() {
        test_snap(
            TimeGranularity::Minutes,
            1,
            "2010-01-01T00:00:03",
            "2013-01-01T01:12:05",
            "2013-01-01T01:12:03",
        );
    }

    #[test]
    fn time_snap_minute_15() {
        test_snap(
            TimeGranularity::Minutes,
            15,
            "2010-01-01T00:00:00",
            "2013-01-01T01:16:00",
            "2013-01-01T01:15:00",
        );
    }

    #[test]
    fn time_snap_minute_31() {
        test_snap(
            TimeGranularity::Minutes,
            31,
            "2010-01-01T00:00:00",
            "2010-01-01T01:01:00",
            "2010-01-01T00:31:00",
        );
    }

    #[test]
    fn time_snap_second_1() {
        test_snap(
            TimeGranularity::Seconds,
            1,
            "2010-01-01T00:00:00",
            "2010-01-01T01:01:12",
            "2010-01-01T01:01:12",
        );
    }

    #[test]
    fn time_snap_second_15() {
        test_snap(
            TimeGranularity::Seconds,
            1,
            "2010-01-01T00:00:00",
            "2010-01-01T01:01:12",
            "2010-01-01T01:01:12",
        );
    }

    #[test]
    fn time_snap_second_31() {
        test_snap(
            TimeGranularity::Seconds,
            31,
            "2010-01-01T23:59:00",
            "2010-01-02T00:00:02",
            "2010-01-02T00:00:02",
        );
    }

    #[test]
    fn time_snap_second_31_2() {
        test_snap(
            TimeGranularity::Seconds,
            31,
            "2010-01-01T23:59:00",
            "2010-01-02T00:00:01",
            "2010-01-01T23:59:31",
        )
    }

    #[test]
    fn num_steps_y_1_0() {
        test_num_steps(
            TimeGranularity::Years,
            1,
            "2001-01-01T01:01:01",
            "2001-01-01T01:01:01",
            0,
        )
    }

    #[test]
    fn num_steps_y_1_1() {
        test_num_steps(
            TimeGranularity::Years,
            1,
            "2001-01-01T01:01:01",
            "2002-01-01T01:01:01",
            0,
        )
    }

    #[test]
    fn num_steps_y_1_2() {
        test_num_steps(
            TimeGranularity::Years,
            1,
            "2001-01-01T01:01:01",
            "2002-01-01T01:01:02",
            1,
        )
    }

    #[test]
    fn num_steps_y_1_3() {
        test_num_steps(
            TimeGranularity::Years,
            1,
            "2001-01-01T01:01:01",
            "2003-01-01T01:01:02",
            2,
        )
    }

    #[test]
    fn num_steps_y_6() {
        test_num_steps(
            TimeGranularity::Years,
            2,
            "2001-01-01T01:01:01",
            "2013-02-02T02:02:02",
            6,
        )
    }

    #[test]
    fn num_steps_m_0() {
        test_num_steps(
            TimeGranularity::Months,
            2,
            "2001-01-01T01:01:01",
            "2001-02-01T01:01:01",
            0,
        )
    }

    #[test]
    fn num_steps_m_1() {
        test_num_steps(
            TimeGranularity::Months,
            1,
            "2001-01-01T01:01:01",
            "2001-02-02T02:02:02",
            1,
        )
    }

    #[test]
    fn num_steps_m_43() {
        test_num_steps(
            TimeGranularity::Months,
            3,
            "2001-01-01T01:01:01",
            "2011-10-02T02:02:02",
            43,
        )
    }

    #[test]
    fn num_steps_d_1() {
        test_num_steps(
            TimeGranularity::Days,
            1,
            "2001-01-01T01:01:01",
            "2001-01-02T02:02:02",
            1,
        )
    }

    #[test]
    fn num_steps_d_366() {
        test_num_steps(
            TimeGranularity::Days,
            2,
            "2001-01-01T01:01:01",
            "2003-01-03T02:02:02",
            366,
        )
    }

    #[test]
    fn num_steps_h_0() {
        test_num_steps(
            TimeGranularity::Hours,
            1,
            "2001-01-01T01:01:01",
            "2001-01-01T01:01:01",
            0,
        )
    }

    #[test]
    fn num_steps_h_1() {
        test_num_steps(
            TimeGranularity::Hours,
            1,
            "2001-01-01T01:01:01",
            "2001-01-01T02:02:02",
            1,
        )
    }

    #[test]
    fn num_steps_h_11() {
        test_num_steps(
            TimeGranularity::Hours,
            6,
            "2001-01-01T01:01:01",
            "2001-01-03T19:01:02",
            11,
        )
    }

    #[test]
    fn num_steps_min_1() {
        test_num_steps(
            TimeGranularity::Minutes,
            1,
            "2001-01-01T01:01:01",
            "2001-01-01T01:02:02",
            1,
        )
    }

    #[test]
    fn num_steps_min_7() {
        test_num_steps(
            TimeGranularity::Minutes,
            10,
            "2001-01-01T01:01:01",
            "2001-01-01T02:11:02",
            7,
        )
    }

    #[test]
    fn num_steps_sec_0() {
        test_num_steps(
            TimeGranularity::Seconds,
            1,
            "2001-01-01T01:01:01",
            "2001-01-01T01:01:01",
            0,
        )
    }

    #[test]
    fn num_steps_sec_0_1() {
        test_num_steps(
            TimeGranularity::Seconds,
            1,
            "2001-01-01T01:01:01",
            "2001-01-01T01:01:02",
            0,
        )
    }

    #[test]
    fn num_steps_sec_1() {
        test_num_steps(
            TimeGranularity::Seconds,
            1,
            "2001-01-01T01:01:01",
            "2001-01-01T01:01:03",
            1,
        )
    }

    #[test]
    fn num_steps_sec_7() {
        test_num_steps(
            TimeGranularity::Seconds,
            10,
            "2001-01-01T01:01:01",
            "2001-01-01T01:02:12",
            7,
        )
    }
}
