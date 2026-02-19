use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Please provide a word to translate");
        process::exit(2);
    }
    match ydt::get_translation(&args[1]) {
        Ok(text) => println!("{text}"),
        Err(err) => {
            eprintln!("{err}");
            process::exit(1);
        }
    }
}
