//! Stateless browser boundary shared by native tests and WebAssembly.
use crate::play::Session;
use serde_json::Value;

pub fn request(action: &str, payload: &str) -> Result<String, String> {
    if !matches!(action, "state" | "ai") {
        return Err("Invalid game request".into());
    }
    if payload.len() > 65536 {
        return Err("Game request is too large".into());
    }
    let value: Value = serde_json::from_str(payload).map_err(|_| "Invalid game request")?;
    let object = value.as_object().ok_or("Invalid game request")?;
    let mut moves = Vec::new();
    if let Some(value) = object.get("moves") {
        let list = value.as_array().ok_or("Invalid move transcript")?;
        if list.len() > 2000 {
            return Err("Move transcript is too long".into());
        }
        for value in list {
            let text = value.as_str().ok_or("Invalid move transcript")?;
            let bytes = text.as_bytes();
            if !(4..=5).contains(&bytes.len())
                || !(b'a'..=b'h').contains(&bytes[0])
                || !(b'1'..=b'8').contains(&bytes[1])
                || !(b'a'..=b'h').contains(&bytes[2])
                || !(b'1'..=b'8').contains(&bytes[3])
                || (bytes.len() == 5 && !b"qbnr@".contains(&bytes[4]))
            {
                return Err("Invalid move transcript".into());
            }
            moves.push(text);
        }
    }
    let human = match object.get("human") {
        None => "black",
        Some(Value::String(s)) if matches!(s.as_str(), "black" | "white") => s,
        _ => return Err("Invalid side".into()),
    };
    let level = match object.get("level") {
        None => "balanced",
        Some(Value::String(s)) if matches!(s.as_str(), "relaxed" | "balanced" | "challenging") => s,
        _ => return Err("Invalid opponent strength".into()),
    };
    let fen = match object.get("fen") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if s.chars().count() <= 200 => Some(s.as_str()),
        _ => return Err("Invalid starting position".into()),
    };
    let mut session = Session::replay(fen, &moves)?;
    if action == "ai" {
        session.ai_turn(human, level)?;
    }
    Ok(session.state_json())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn game_request(action: &str, payload: &str) -> Result<String, String> {
    request(action, payload)
}
