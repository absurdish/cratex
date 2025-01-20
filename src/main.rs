use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: cratex <crate-name>[@version] [args...]");
        process::exit(1);
    }

    let crate_spec = &args[1];
    let (crate_name, version) = if let Some((name, ver)) = crate_spec.split_once('@') {
        (name, Some(ver.to_string()))
    } else {
        (crate_spec.as_str(), None)
    };

    let run_args = if args.len() > 2 {
        args[2..].to_vec()
    } else {
        vec![]
    };

    if let Err(e) = cratex::run(crate_name, version, run_args) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}