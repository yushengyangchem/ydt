use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Please provide a word to translate");
        return;
    }
    println!("{}", ydt::get_translation(&args[1]));
}
