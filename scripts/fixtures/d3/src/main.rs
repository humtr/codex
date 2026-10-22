mod parse;
mod ledger;
mod report;

use std::io::{self, Read};

fn run() -> Result<String, String> {
    if std::env::args().len() != 1 { return Err("arguments".into()); }
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).map_err(|e| e.to_string())?;
    let commands = parse::parse(&input)?;
    let mut state = ledger::Ledger::new();
    ledger::apply(&mut state, &commands)?;
    Ok(report::render(&state))
}

fn main() {
    match run() {
        Ok(text) => print!("{text}"),
        Err(error) => { eprintln!("error: {error}"); std::process::exit(2); }
    }
}

#[cfg(test)]
mod original_tests {
    use super::*;
    #[test]
    fn add_and_sort() {
        let mut state = ledger::Ledger::new();
        ledger::apply(&mut state, &parse::parse("add z 9\nadd a 4\nadd a -1\n").unwrap()).unwrap();
        assert_eq!(report::render(&state), "a\t3\nz\t9\n");
    }
    #[test]
    fn invalid_command_and_negative_balance() {
        assert!(parse::parse("bad").is_err());
        assert!(ledger::apply(&mut ledger::Ledger::new(), &parse::parse("add a -1").unwrap()).is_err());
    }
}
