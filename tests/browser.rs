use african_chess::{browser::request, play::Session};
use serde_json::{Value, json};

fn call(action: &str, payload: Value) -> Value {
    serde_json::from_str(&request(action, &payload.to_string()).unwrap()).unwrap()
}

#[test]
fn initial_state_and_replay_match_session() {
    let state = call("state", json!({}));
    assert_eq!(state["turn"], "black");
    assert_eq!(state["pieces"].as_array().unwrap().len(), 32);
    assert_eq!(state["legal_moves"].as_array().unwrap().len(), 26);
    let expected: Value =
        serde_json::from_str(&Session::replay(None, &["e7e5"]).unwrap().state_json()).unwrap();
    assert_eq!(call("state", json!({"moves": ["e7e5"]})), expected);
}

#[test]
fn ai_uses_existing_learned_opponent() {
    let mut session = Session::replay(None, &["e7e5"]).unwrap();
    session.ai_turn("black", "relaxed").unwrap();
    let expected: Value = serde_json::from_str(&session.state_json()).unwrap();
    assert_eq!(
        call("ai", json!({"moves": ["e7e5"], "level": "relaxed"})),
        expected
    );
    let opening = call("ai", json!({"human": "white", "level": "relaxed"}));
    assert_eq!(opening["turn"], "white");
    assert_eq!(opening["history"].as_array().unwrap().len(), 1);
}

#[test]
fn invalid_requests_are_errors() {
    for payload in [
        json!(null),
        json!([]),
        json!({"moves": "a7a6"}),
        json!({"moves": [123]}),
        json!({"moves": ["a7a6;date"]}),
        json!({"moves": ["e2e4"]}),
        json!({"moves": vec!["a7a6"; 2001]}),
        json!({"moves": null}),
        json!({"fen": 3}),
        json!({"fen": "x".repeat(201)}),
        json!({"human": "red"}),
        json!({"human": null}),
        json!({"level": "bad"}),
        json!({"level": false}),
    ] {
        assert!(request("state", &payload.to_string()).is_err(), "{payload}");
    }
    assert!(request("state", "{").is_err());
    assert!(request("state", &" ".repeat(65537)).is_err());
    assert!(request("unknown", "{}").is_err());
    assert!(request("ai", "{}").is_err());
}

#[test]
fn stationary_capture_and_terminal_state_work() {
    let state = call(
        "state",
        json!({
            "fen": "7k/8/1q6/8/8/8/1N6/K7 w - - 0 1", "moves": ["b2b6@"],
        }),
    );
    assert_eq!(state["history"][0]["stationary"], true);
    assert!(
        state["pieces"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["square"] == "b2")
    );
    assert!(
        !state["pieces"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["square"] == "b6")
    );
    let state = call("state", json!({"fen": "7k/6Q1/5K2/8/8/8/8/8 b - - 0 1"}));
    assert_eq!(state["outcome"], "checkmate");
    assert_eq!(state["winner"], "white");
    assert_eq!(state["legal_moves"], json!([]));
    assert_eq!(
        call("state", json!({"fen": null})),
        call("state", json!({}))
    );
}
