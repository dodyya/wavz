use std::env::args;
use wavz::demos::*;
fn main() {
	let mut args = args();
	let _ = args.next();
	let command = args.next().expect("Expected subcommand");
	let file_path: Option<String> = args.next();

	match (command.as_str(), file_path) {
		("mic", _) => mic_vis(),
		("asciimic", _) => mic_ascii(),
		("precomp", Some(file_path)) => precomp_vis(&file_path),
		("vis", Some(file_path)) => realtime_vis(&file_path),
		("play", Some(file_path)) => wav_player(&file_path),
		_ => println!(
			"Supported commands are: asciimic, mic, vis, precomp, play. The last 3 require a file path."
		),
	};
}
