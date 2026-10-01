use african_chess::play::Session;

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or("Expected state or ai command")?;
    if !["state", "ai"].contains(&command.as_str()) {
        return Err("Expected state or ai command".into());
    }
    let mut fen = None;
    let mut moves = String::new();
    let mut human = "black".to_owned();
    let mut level = "balanced".to_owned();
    while let Some(key) = args.next() {
        let value = args.next().ok_or("Option is missing its value")?;
        match key.as_str() {
            "--fen" => fen = Some(value),
            "--moves" => moves = value,
            "--human" => human = value,
            "--level" => level = value,
            _ => return Err(format!("Unknown option: {key}")),
        }
    }
    let transcript: Vec<_> = moves.split_whitespace().collect();
    let mut session = Session::replay(fen.as_deref(), &transcript)?;
    if command == "ai" {
        session.ai_turn(&human, &level)?
    }
    println!("{}", session.state_json());
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2)
    }
}
