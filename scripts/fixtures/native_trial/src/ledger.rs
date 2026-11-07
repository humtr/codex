use std::collections::BTreeMap;
use crate::parse::Command;

pub type Ledger = BTreeMap<String, i64>;

pub fn apply(ledger: &mut Ledger, commands: &[Command]) -> Result<(), String> {
    for command in commands {
        match command {
            Command::Add(account, amount) => {
                let value = ledger.get(account).copied().unwrap_or(0)
                    .checked_add(*amount).ok_or("overflow")?;
                if value < 0 { return Err("negative balance".into()); }
                ledger.insert(account.clone(), value);
            }
        }
    }
    Ok(())
}
