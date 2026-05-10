mod constants;
mod interpreter;
mod soup;
mod stats;

use std::fs;
use std::io;

use crate::soup::Soup;

fn main() {
    let mut name = String::new();
    println!("Run name:");
    io::stdin().read_line(&mut name).expect("failed to read");
    let name = name.trim(); // strips the trailing newline

    fs::create_dir_all("logs").expect("could not create logs dir");
    fs::create_dir_all("samples").expect("could not create samples dir");
    fs::create_dir_all("replicators").expect("could not create replicators dir");
    let log_path = format!("logs/{}.jsonl", name);
    let samples_path = format!("samples/{}.jsonl", name);
    let repl_path = format!("replicators/{}.jsonl", name);

    let mut soup: Soup = Soup::new(crate::constants::SOUP_SIZE);
    soup.run(&log_path, &samples_path, &repl_path);
}
