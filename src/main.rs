#![allow(
	clippy::module_name_repetitions,
	clippy::unused_self,
	clippy::missing_const_for_fn // TODO: Remove when #![feature(const_precise_live_drops)] gets stabilized
)]

use std::collections::BTreeMap;

use chrono::SubsecRound;
use clap::{crate_authors, crate_version, load_yaml, App, AppSettings};

use crate::beacon::Beacon;
use crate::error::VerifierError;
use crate::framework::Framework;
use crate::model::Model;
use crate::output::BeaconOutput;

mod beacon;
mod endpoint;
mod error;
mod framework;
mod interface;
mod model;
mod output;
mod utils;

pub type Json = serde_json::Value;

fn main() -> Result<(), VerifierError> {
	// Get args
	let yaml = load_yaml!("../cli.yaml");
	let matches = App::from(yaml)
		.version(crate_version!())
		.author(crate_authors!())
		.global_setting(AppSettings::ArgRequiredElseHelp)
		.global_setting(AppSettings::ColorAlways)
		.global_setting(AppSettings::ColoredHelp)
		.get_matches();

	// Verbose

	if matches.is_present("quiet") {
		log::set_max_level(log::LevelFilter::Off);
	}
	if matches.is_present("verbose") {
		std::env::set_var("RUST_LOG", "debug");
	}
	else {
		std::env::set_var("RUST_LOG", "info");
	}
	pretty_env_logger::init();

	// Load framework
	let framework_location = url::Url::parse(
		matches
			.value_of("framework")
			.expect("No --framework passed as argument"),
	)
	.map_err(|_| VerifierError::ArgNotURL("--framework"))?;
	log::debug!("Loading framework from: {}", &framework_location);
	let framework = Framework::load(&framework_location).expect("Loading framework failed");
	log::debug!("Framework loaded");

	// Load model
	let model_location = url::Url::parse(matches.value_of("model").expect("No --model passed as argument"))
		.map_err(|_| VerifierError::ArgNotURL("--model"))?;
	log::debug!("Loading model from: {}", model_location);
	let model = Model::load(&model_location).expect("Loading model failed");
	let n_entitites = model.validate(&framework);
	log::info!("Valid model (number of entities: {})", n_entitites);

	// Validate beacons
	if !matches.is_present("only-model") {
		// Load beacon
		let beacon_url =
			url::Url::parse(matches.value_of("URL").expect("No URL")).map_err(|_| VerifierError::ArgNotURL("URL"))?;
		log::info!("Validating implementation on {}", beacon_url);
		let output = match Beacon::new(model, framework, &beacon_url) {
			Ok(beacon) => beacon.validate(),
			Err(e) => BeaconOutput {
				name: format!("Unknown Beacon ({})", e),
				url: beacon_url,
				last_updated: chrono::offset::Utc::now().naive_utc().round_subsecs(6),
				entities: BTreeMap::new(),
			},
		};

		let payload = serde_json::to_string_pretty(&output).unwrap();
		println!("{}", payload);
	}

	Ok(())
}
