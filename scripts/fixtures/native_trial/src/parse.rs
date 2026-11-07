#[derive(Debug, PartialEq)]
pub enum Command {
    Add(String, i64),
}

pub fn parse(text: &str) -> Result<Vec<Command>, String> {
    let mut commands = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.is_empty() { continue; }
        match fields.as_slice() {
            ["add", account, amount] => commands.push(Command::Add(
                account.to_string(), amount.parse().map_err(|_| format!("line {}: amount", index + 1))?,
            )),
            _ => return Err(format!("line {}: command", index + 1)),
        }
    }
    Ok(commands)
}
