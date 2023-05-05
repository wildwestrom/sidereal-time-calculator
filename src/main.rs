use std::{
	io::{stdout, Write},
	str::FromStr,
	thread::sleep,
};

use anyhow::{anyhow, Result};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use chrono_tz::Tz;
use libastro_sys::{cal_mjd, utc_gst};
use tzf_rs::DefaultFinder;

fn utc_to_float(time: NaiveTime) -> f64 {
	(time.hour() as f64)
		+ (time.minute() as f64 / (60.0))
		+ (time.second() as f64 / (60.0 * 60.0))
		+ (time.nanosecond() as f64 / (60.0 * 60.0 * 1_000_000_000.0))
}

pub fn mjd_from_gregorian_date(date: NaiveDate) -> f64 {
	let dy = date.day() as f64;
	let mn = date.month() as i32;
	let yr = date.year() as i32;
	let mut mjd = 0.0;
	unsafe { cal_mjd(mn, dy, yr, &mut mjd as *mut f64) };
	mjd
}

pub fn mjd_from_gregorian_datetime(datetime: NaiveDateTime) -> f64 {
	let mjd = mjd_from_gregorian_date(datetime.date());
	mjd + utc_to_float(datetime.time())
}

pub fn greenwich_mean_sidereal_time(datetime: NaiveDateTime) -> f64 {
	let mut gst = 0.0;
	let utc = utc_to_float(datetime.time());
	let mjd = mjd_from_gregorian_date(datetime.date()).floor();
	unsafe { utc_gst(mjd, utc, &mut gst as *mut f64) };
	gst
}

/// Find the timezone for the given coordinates
fn get_timezone(latitude: f64, longitude: f64) -> Result<Tz> {
	let finder = DefaultFinder::new();
	let timezone = finder.get_tz_names(longitude, latitude);
	let tz_str = match timezone.len() {
		0 => Err(anyhow!("No timezones found")),
		1 => Ok(timezone.first().expect("already checked").to_owned()),
		_ => Err(anyhow!("Todo: Allow picking a timezone name")),
	}?;
	if let Ok(tz) = Tz::from_str(tz_str) {
		Ok(tz)
	} else {
		Err(anyhow!("Could not convert {tz_str} into a timezone."))
	}
}

fn decimal_to_time(dec_time: f64) -> Result<NaiveTime> {
	let hr = dec_time;
	let min = hr.fract() * 60.0;
	let sec = min.fract() * 60.0;
	let ns = sec.fract() * 1_000_000_000.0;

	NaiveTime::from_hms_nano_opt(hr as u32, min as u32, sec as u32, ns as u32)
		.ok_or(anyhow!("Time conversion failed, time: {dec_time}"))
}

fn local_mean_sidereal_time(gmst: f64, longitude: f64) -> f64 {
	24.0 * ((gmst + longitude / 15.0) / 24.0).fract()
}

const TIME_FMT_STRING: &str = "%T.%6f";
const TIME_ZONE_FMT_STRING: &str = "%T.%6f %z/%Z";

fn main() -> Result<()> {
	// don't use more than two digits of precision for coordinates
	let latitude = 36.755;
	let longitude = 127.869;
	let tz = get_timezone(latitude, longitude)?;
	let spotiswoode_peak_time = NaiveTime::from_hms_opt(13, 30, 0).unwrap();

	let term = console::Term::stdout();

	loop {
		let mut lines_to_clear = 0;

		println!(
			"       Zone for {:>5.1}, {:>5.1}: {:?}",
			latitude, longitude, tz
		);
		lines_to_clear += 1;

		let now = Utc::now();
		let curr_date = now.date_naive();
		let local_time = now.with_timezone(&tz);

		println!("                  Gregorian Date: {}", curr_date);
		lines_to_clear += 1;

		println!(
			"                  Universal Time: {}",
			now.format(TIME_ZONE_FMT_STRING)
		);
		lines_to_clear += 1;

		println!(
			"                      Local Time: {}",
			local_time.format(TIME_ZONE_FMT_STRING)
		);
		lines_to_clear += 1;

		let mjd = mjd_from_gregorian_datetime(now.naive_utc());
		println!("             Modified Julian Day: {}", mjd);
		lines_to_clear += 1;

		let gmst = greenwich_mean_sidereal_time(now.naive_utc());
		println!(
			"    Greenwich mean Sidereal Time: {} ",
			decimal_to_time(gmst)?.format(TIME_FMT_STRING)
		);
		lines_to_clear += 1;

		let lmst = local_mean_sidereal_time(gmst, longitude);
		println!(
			"        Local mean Sidereal Time: {}",
			decimal_to_time(lmst)?.format(TIME_FMT_STRING)
		);
		lines_to_clear += 1;

		let time_until_peak = {
			let duration = spotiswoode_peak_time.signed_duration_since(decimal_to_time(lmst)?);
			if duration.lt(&Duration::zero()) {
				// If the duration is negative, add 24 hours to it to get the time until the next occurrence.
				duration + chrono::Duration::hours(24)
			} else {
				duration
			}
		};
		println!(
			"Time Until Spotiswoode Peak Time: {}",
			decimal_to_time(time_until_peak.num_nanoseconds().unwrap() as f64 / 1_000_000_000.0 / 60.0 / 60.0)?
				.format(TIME_FMT_STRING),
		);
		lines_to_clear += 1;

		stdout().flush().unwrap();
		sleep(std::time::Duration::from_millis(5));
		let _ = term.clear_last_lines(lines_to_clear)?;
	}
}
