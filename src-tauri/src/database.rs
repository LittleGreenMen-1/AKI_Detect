use std::fs;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;
use unix_ts::Timestamp;

// Define custom error type
#[derive(Debug)]
struct MyError(String);

impl std::fmt::Display for MyError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "Error: {}", self.0)
	}
}

impl std::error::Error for MyError {}

// Define custom Result types
pub type MyResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// The Patient struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Patient {
    id: i64,
	pub name: String,
	pub s_cr_levels: BTreeMap<String, f32>,

	#[serde(skip_deserializing)]
    pub aki_score: i32,

	#[serde(skip_deserializing)]
	pub difference: f32,
}

impl Patient {
	pub fn new(name: String) -> Self {
		let mut rng = rand::thread_rng();
		let mut current_timestamp = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("Time went backwards")
			.as_secs() as i64;
		
		// Make sure the id is unique
		current_timestamp = current_timestamp * 1000 + rng.gen_range(0..999);

		Self {
			id: current_timestamp,
			name,
			s_cr_levels: BTreeMap::new(),
            aki_score: 0,
			difference: 0.0,
		}
	}

    pub fn set_aki(&mut self, (aki_score, difference): (i32, f32)) {
        self.aki_score = aki_score;
		self.difference = difference;
    }
}

// Database Model
pub struct DataBase {
	pub data: Vec<Patient>,
    file_path: String,
}

impl DataBase {
	pub const fn new() -> Self {
		Self {
			data: Vec::new(),
			file_path: String::new(),
		}
	}

	pub async fn init() -> MyResult<Self> {
		// Check if database file exists
		let file_path = "database.json";

		match fs::metadata(file_path) {
			Ok(_) => {
                // Read data from file
                let data = fs::read_to_string(file_path)?;
                let mut patients: Vec<Patient> = serde_json::from_str(&data)?;

                patients.iter_mut().for_each(|p|
					p.set_aki( detect_aki(&p) )
                );
                
                Ok(Self {
                    data: patients,
                    file_path: file_path.to_string(),
                })
            }
			Err(_) => {
                // Create file
                let _file = fs::File::create(file_path)?;

                Ok(Self {
                    data: Vec::new(),
                    file_path: file_path.to_string(),
                })
            }
		}
	}

    pub async fn fetch_patients(&self) -> MyResult<Vec<Patient>> {
        Ok(self.data.clone())
    }

    pub async fn find_patient(&self, id: i64) -> MyResult<Patient> {
        let patient = self.data.iter().find(|p| p.id == id);

        match patient {
            Some(p) => Ok(p.clone()),
            None => Err(Box::new(MyError("Patient not found".into()))),
        }
    }

    pub async fn add_patient(&mut self, patient: Patient) -> MyResult<()> {
		self.data.push(patient);

        Ok(())
    }

    pub async fn update_patient(&mut self, id: i64, level: f32) -> MyResult<()> {
        let index = self.data.iter().position(|p| p.id == id);

		let current_timestamp = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("Time went backwards")
			.as_secs() as i64;
		
        match index {
            Some(i) => {
				let keys: Vec<_> = self.data[i].s_cr_levels.keys().cloned().collect();
				let latest_key = keys.last().unwrap().to_string();
				let latest_timestamp = latest_key.parse::<i64>().unwrap();

				if same_day(current_timestamp, latest_timestamp) {
					if let Some(x) = self.data[i].s_cr_levels.get_mut(&latest_key) {
						*x = level;
					}
				} else {
					self.data[i].s_cr_levels.insert(
						current_timestamp.to_string(),
						level
					);
				}

                let p = &mut self.data[i];
                p.set_aki( detect_aki(&p) );

                Ok(())
            }
            None => Err(Box::new(MyError("Patient not found".into()))),
        }
    }

    pub async fn delete_patient(&mut self, id: i64) -> MyResult<()> {
        let index = self.data.iter().position(|p| p.id == id);

        match index {
            Some(i) => {
                self.data.remove(i);
                Ok(())
            }
            None => Err(Box::new(MyError("Patient not found".into()))),
        }
    }

    pub async fn save(&self) -> MyResult<()> {
        let data = serde_json::to_string(&self.data)?;

        fs::write(&self.file_path, data)?;

        Ok(())
    }
}

fn unix_time_to_human_readable(seconds: i64) -> String {
	let mut ans = String::from("");

	let days_of_month = [ 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31 ];

	let (mut curr_year, mut days_till_now);
	let (mut extra_days, mut index, date, mut month);
	let mut flag = 0;

	// Calculate total days unix time T
	days_till_now = (seconds / (24 * 60 * 60)) as i64;
	curr_year = 1970;

	// Calculating current year
	loop {
		if curr_year % 400 == 0 || (curr_year % 4 == 0 && curr_year % 100 != 0) {
			if days_till_now < 366 {
				break;
			}
			days_till_now -= 366;
		} else {
			if days_till_now < 365 {
				break;
			}
			days_till_now -= 365;
		}
		curr_year += 1;
	}

	// Updating extradays because it
	// will give days till previous day
	// and we have include current day
	extra_days = days_till_now + 1;

	if curr_year % 400 == 0 || (curr_year % 4 == 0 && curr_year % 100 != 0) {
		flag = 1;
	}

	// Calculating MONTH and DATE
	month = 0; index = 0;
	if flag == 1 {
		loop {
			if index == 1 {
				if extra_days - 29 < 0 {
					break;
				}

				month += 1;
				extra_days -= 29;
			} else {
				if extra_days - days_of_month[index] < 0 {
					break;
				}
				month += 1;
				extra_days -= days_of_month[index];
			}
			index += 1;
		}
	} else {
		loop {
			if extra_days - days_of_month[index] < 0 {
				break;
			}
			month += 1;
			extra_days -= days_of_month[index];
			index += 1;
		}
	}

	// Current Month
	if extra_days > 0 {
		month += 1;
		date = extra_days;
	} else {
		if month == 2 && flag == 1 {
			date = 29;
		} else {
			date = days_of_month[month - 1];
		}
	}

	ans += &date.to_string();
	ans += "/";
	ans += &month.to_string();
	ans += "/";
	ans += &curr_year.to_string();

	// Return the time
	return ans;
}

fn same_day(day1: i64, day2: i64) -> bool {
	let date1 = unix_time_to_human_readable(day1);
	let date2 = unix_time_to_human_readable(day2);

	return date1 == date2;
}

fn detect_aki(p: &Patient) -> (i32, f32) {
	let levels = &p.s_cr_levels;
	
	// Find baseline creatinine
	let mut baseline = f32::MAX;

	levels.values().for_each(|v| {
		if *v < baseline {
			baseline = *v;
		}
	});

	// Check for AKI : increase of more than 0.3 mg/dl within 48 hours
	let mut i = 0;
	
	while i < levels.len() {
		let date_i = Timestamp::from(
			levels.keys().nth(i).unwrap()
				.parse::<i64>().unwrap()
		);

		let mut j = i + 1;

		while j < levels.len() {
			let date_j = Timestamp::from(
				levels.keys().nth(j).unwrap()
					.parse::<i64>().unwrap()
			);

			if (date_j - date_i).seconds() > 172800 { // more than 48h
				break;
			}

			let diff = levels.values().nth(j).unwrap() - levels.values().nth(i).unwrap();

			if diff >= 0.3 { // AKI detected
				// Find what type of AKI it is
				let curr_level = levels.values().nth(j).unwrap();

				if curr_level >= &(baseline * 3.0) {
					return (3, diff);
				}

				if curr_level >= &(baseline * 2.0) {
					return (2, diff);
				}

				if curr_level >= &(baseline * 1.5) {
					return (1, diff);
				}

				return (3, diff);
			}

			j += 1;
		}

		i += 1;
	}

	// Check for AKI : increase of more than 1.5 times baseline within 7 days
	let mut i = 0;

	while i < levels.len() {
		let date_i = Timestamp::from(
			levels.keys().nth(i).unwrap()
				.parse::<i64>().unwrap()
		);

		let mut j = i + 1;

		while j < levels.len() {
			let date_j = Timestamp::from(
				levels.keys().nth(j).unwrap()
					.parse::<i64>().unwrap()
			);

			if (date_j - date_i).seconds() > 604800 { // more than 7 days
				break;
			}

			let diff = levels.values().nth(j).unwrap() - levels.values().nth(i).unwrap();

			if diff >= baseline * 1.5 {
				// Find what type of AKI it is
				let curr_level = levels.values().nth(j).unwrap();

				if curr_level >= &(baseline * 3.0) {
					return (3, diff);
				}

				if curr_level >= &(baseline * 2.0) {
					return (2, diff);
				}

				if curr_level >= &(baseline * 1.5) {
					return (1, diff);
				}

				return (3, diff);
			}

			j += 1;
		}

		i += 1;
	}

	return (0, 0.0);
}