// #![allow(unused)]
use std::{
	io::{stdout, Write},
	thread::sleep,
};

use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveTime, Timelike, Utc};
use tzf_rs::DefaultFinder;

mod libastro;

fn get_timezone(latitude: f64, longitude: f64) -> String {
	// Find the timezone for the given coordinates
	let finder = DefaultFinder::new();
	let timezone = finder.get_tz_name(longitude, latitude);
	timezone.to_string()
}

// fn naive_to_astro_date(gdt: DateTime<Utc>) -> astro::time::Date {
// 	let day_of_month = astro::time::DayOfMonth {
// 		day: gdt.date_naive().day() as u8,
// 		hr: gdt.time().hour() as u8,
// 		min: gdt.time().minute() as u8,
// 		sec: gdt.time().second() as f64 + (gdt.time().nanosecond() as f64 / 1_000_000_000.0),
// 		time_zone: 0.0,
// 	};
// 	let dec_day = astro::time::decimal_day(&day_of_month);
// 	astro::time::Date {
// 		year: gdt.date_naive().year() as i16,
// 		month: gdt.date_naive().month() as u8,
// 		decimal_day: dec_day,
// 		cal_type: astro::time::CalType::Gregorian,
// 	}
// }

// fn radians_to_time(rad: f64) -> f64 {
// 	let a = ((24 * 60 * 60) as f64 * rad) / std::f64::consts::PI;
// 	dbg!(a);
// 	a
// }

fn main() -> Result<()> {
	// don't use more than two digits of precision for coordinates
	let latitude = 36.717;
	let longitude = 127.837;
	let tz = get_timezone(latitude, longitude);

	loop {
		println!("Zone for {latitude}, {longitude}: {tz}");
		let now = Utc::now();
		let curr_date = now.date_naive();
		let curr_time = now.time();
		println!("Gregorian Date: {}", curr_date);
		println!("UTC: {}", curr_time.format("%T.%6f"));

		let jd = libastro::mjd_from_gregorian(now.naive_utc());

		// println!("Julian Day: {}", jd);
		// let mean_st = mn_sidr(jd);
		// let _st_disp = radians_to_time(mean_st);
		// println!("Mean Sidereal Time: {}", mean_st);
		// let apparent_st = apprnt_sidr!(jd);
		// println!("Apparent Sidereal Time: {}", apparent_st);
		// stdout().flush().unwrap();
		// sleep(std::time::Duration::from_millis(100));
		// print!("{}", termion::clear::All);
	}
}
