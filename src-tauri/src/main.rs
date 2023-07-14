// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// mod error;
mod database;

use database::{DataBase, Patient, MyResult};

use std::sync::Mutex;
use tauri::State;

struct Note(Mutex<i64>);

static mut DB: DataBase = DataBase::new();

#[tauri::command]
async fn get_patients() -> Vec<Patient> {

	unsafe {
		let patients = DB.fetch_patients().await;

		match patients {
			Ok(result) => {
				return result;
			},
			Err(e) => {
				println!("Error fetching patients: {}", e);
				return Vec::new();
			},
		}
	}

}

#[tauri::command]
async fn new_patient(name: &str) -> Result<(), ()> {

	unsafe {
		let p = Patient::new(name.to_string());

		let _ = DB.add_patient(p).await;
		let _ = DB.save().await;
	}

	Ok(())

}

#[tauri::command]
async fn update_patient(id: i64, new_level: f32) -> Result<(), ()> {
	
	unsafe {
		let _ = DB.update_patient(id, new_level).await;
		let _ = DB.save().await;
	}

	Ok(())
}

#[tauri::command]
async fn delete_patient(id: i64) -> Result<(), ()> {
	
	unsafe {
		let _ = DB.delete_patient(id).await;
		let _ = DB.save().await;
	}

	Ok(())
}

#[tauri::command]
fn save_id(id: i64, state: State<Note>) {
	*(state.0.lock().unwrap()) = id;
}

#[tauri::command]
fn get_id(state: State<Note>) -> i64 {
	*(state.0.lock().unwrap())
}

#[tauri::command]
async fn get_patient(id: i64) -> Result<Patient, ()> {
	
	unsafe {
		let patient = DB.find_patient(id).await;

		match patient {
			Ok(result) => {
				return Ok(result);
			},
			Err(e) => {
				println!("Error fetching patient: {}", e);
				return Ok(Patient::new("".to_string()));
			},
		}
	}
}

// Main function
#[tokio::main]
async fn main() -> MyResult<()> {
	unsafe {
		DB = DataBase::init().await?;
	}

	tauri::Builder::default()
		.manage(Note(Default::default()))
		.invoke_handler(tauri::generate_handler![
			get_patients,
			new_patient,
			get_patient,
			update_patient,
			delete_patient,
			save_id,
			get_id
		])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");

	Ok(())
}
