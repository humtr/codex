use crate::ledger::Ledger;

pub fn render(ledger: &Ledger) -> String {
    ledger.iter().map(|(account, balance)| format!("{account}\t{balance}\n")).collect()
}
