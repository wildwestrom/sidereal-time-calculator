use std::str::FromStr;

use anyhow::{anyhow, Result};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use chrono_tz::Tz;
use clap::Parser;
use libastro_sys::{cal_mjd, utc_gst};
use once_cell::sync::Lazy;
use tzf_rs::DefaultFinder;

fn utc_to_float(time: NaiveTime) -> f64 {
	f64::from(time.hour())
		+ (f64::from(time.minute()) / (60.0))
		+ (f64::from(time.second()) / (60.0 * 60.0))
		+ (f64::from(time.nanosecond()) / (60.0 * 60.0 * 1_000_000_000.0))
}

#[must_use]
fn mjd_from_gregorian_date(date: NaiveDate) -> f64 {
	let dy = f64::from(date.day());
	let mn = i32::try_from(date.month()).unwrap();
	let yr = date.year();
	let mut mjd = 0.0;
	unsafe { cal_mjd(mn, dy, yr, std::ptr::addr_of_mut!(mjd)) };
	mjd
}

#[must_use]
fn mjd_from_gregorian_datetime(datetime: NaiveDateTime) -> f64 {
	let mjd = mjd_from_gregorian_date(datetime.date());
	mjd + utc_to_float(datetime.time())
}

#[must_use]
fn greenwich_mean_sidereal_time(datetime: NaiveDateTime) -> f64 {
	let mut gst = 0.0;
	let utc = utc_to_float(datetime.time());
	let mjd = mjd_from_gregorian_date(datetime.date()).floor();
	unsafe { utc_gst(mjd, utc, std::ptr::addr_of_mut!(gst)) };
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
	Tz::from_str(tz_str).map_err(|e| anyhow!("Could not convert string: {e}"))
}

#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
fn decimal_to_time(dec_time: f64) -> Result<NaiveTime> {
	assert!(dec_time.is_sign_positive());
	let hr = dec_time;
	let min = hr.fract() * 60.0;
	let sec = min.fract() * 60.0;
	let ns = sec.fract() * 1_000_000_000.0;

	NaiveTime::from_hms_nano_opt(hr as u32, min as u32, sec as u32, ns as u32)
		.ok_or_else(|| anyhow!("Time conversion failed, time: {dec_time}"))
}

fn local_mean_sidereal_time(gmst: f64, longitude: f64) -> f64 {
	24.0 * ((gmst + longitude / 15.0) / 24.0).fract()
}

fn display_info(latitude: Option<f64>, longitude: f64) -> Result<()> {
	const TIME_FMT_STRING: &str = "%T.%6f";
	const TIME_ZONE_FMT_STRING: &str = "%T.%6f %z/%Z";

	let term = console::Term::buffered_stdout();

	let timezone;

	if let Some(latitude) = latitude {
		timezone = get_timezone(latitude, longitude).ok();
	} else {
		timezone = None
	}

	loop {
		let utc_datetime = Utc::now();

		let mut info = String::new();

		if let (Some(latitude), Some(timezone)) = (latitude, timezone) {
			info.push_str(&format!(
				"           Zone for {:>5.1}, {:>5.1}: {:?}\n",
				latitude, longitude, timezone
			));

			let local_time = utc_datetime.with_timezone(&timezone);
			info.push_str(&format!(
				"                      Local Time: {}\n",
				local_time.format(TIME_ZONE_FMT_STRING)
			));
		} else {
			info.push_str(&format!(
				"                       Longitude: {:>5.1}\n",
				longitude
			))
		}

		let curr_date = utc_datetime.date_naive();

		info.push_str(&format!(
			"                  Gregorian Date: {}\n",
			curr_date
		));

		info.push_str(&format!(
			"                  Universal Time: {}\n",
			utc_datetime.format(TIME_ZONE_FMT_STRING)
		));

		let mjd = mjd_from_gregorian_datetime(utc_datetime.naive_utc());
		info.push_str(&format!("             Modified Julian Day: {}\n", mjd));

		let greenwich_mst = greenwich_mean_sidereal_time(utc_datetime.naive_utc());
		info.push_str(&format!(
			"    Greenwich mean Sidereal Time: {} \n",
			decimal_to_time(greenwich_mst)?.format(TIME_FMT_STRING)
		));

		let local_mst = local_mean_sidereal_time(greenwich_mst, longitude);
		info.push_str(&format!(
			"        Local mean Sidereal Time: {}\n",
			decimal_to_time(local_mst)?.format(TIME_FMT_STRING)
		));

		let time_until_peak = {
			static SPOTISWOODE_PEAK_TIME: Lazy<NaiveTime> =
				Lazy::new(|| NaiveTime::from_hms_opt(13, 30, 0).unwrap());

			let duration = SPOTISWOODE_PEAK_TIME.signed_duration_since(decimal_to_time(local_mst)?);
			if duration.lt(&Duration::zero()) {
				// If the duration is negative, add 24 hours to it to get the time until the next occurrence.
				duration + chrono::Duration::hours(24)
			} else {
				duration
			}
		};
		info.push_str(&format!(
			"Time Until Spotiswoode Peak Time: {}",
			decimal_to_time(
				time_until_peak.num_nanoseconds().unwrap() as f64 / 1_000_000_000.0 / 60.0 / 60.0
			)?
			.format(TIME_FMT_STRING),
		));

		let lines_to_clear = info.chars().into_iter().filter(|c| *c == '\n').count();

		term.write_line(&info)?;
		term.flush()?;
		term.clear_last_lines(lines_to_clear + 1)?;
		std::thread::sleep(std::time::Duration::from_micros(200));
	}
}

#[derive(Parser, Debug)]
#[command(name = "sidtime")]
/// Prints shows the local sidereal time given a longitude.
struct Cli {
	/// Latitude
	#[arg(long)]
	lat: Option<f64>,
	/// Longitude (+ for E - for W)
	#[arg(long)]
	lon: f64,
}

fn main() -> Result<()> {
	let cli = Cli::parse();

	let _ = display_info(cli.lat, cli.lon);
	Ok(())
}
