use std::env::args;
use wavz::demos::*;
use wavz::graphics::ColorScheme;

fn main() {
	let args: Vec<String> = args().collect();
	let mut color = ColorScheme::default();
	let mut positional = Vec::new();

	let mut i = 1;
	while i < args.len() {
		if args[i] == "-c" {
			if let Some(n) = args.get(i + 1).and_then(|s| s.parse().ok()) {
				color = ColorScheme::new(n);
				i += 2;
				continue;
			}
		}
		positional.push(args[i].as_str());
		i += 1;
	}

	let command = positional.first().copied();
	let file_path = positional.get(1).copied();

	match (command, file_path) {
		(Some("mic"), _) => mic_vis(color),
		(Some("asciimic"), _) => mic_ascii(),
		(Some("precomp"), Some(file_path)) => precomp_vis(file_path, color),
		(Some("vis"), Some(file_path)) => realtime_vis(file_path, color),
		(Some("play"), Some(file_path)) => wav_player(file_path),
		_ => println!(
			"Usage: wavz [-c 1|2|3] <command> [file]\n\
			 Commands: asciimic, mic, vis, precomp, play\n\
			 -c: color scheme (1=viridis, 2=inferno, 3=bone)"
		),
	};
}
