use african_chess::opponent::{Limits, choose};
use african_chess::play::Session;
use african_chess::{Color, Game, Outcome, Position};

#[test]
fn deterministic_opponent_finds_mate_on_either_side() {
    for (fen, winner) in [
        ("7k/8/5KQ1/8/8/8/8/8 w - - 0 1", Color::White),
        ("8/8/8/8/8/5kq1/8/7K b - - 0 1", Color::Black),
    ] {
        let mut g = Game::new(Position::from_fen(fen).unwrap());
        let a = choose(
            &g,
            Limits {
                depth: 3,
                nodes: 5000,
            },
        )
        .unwrap();
        assert_eq!(
            a,
            choose(
                &g,
                Limits {
                    depth: 3,
                    nodes: 5000
                }
            )
            .unwrap()
        );
        g.play(a.best.unwrap()).unwrap();
        assert_eq!(g.outcome(), Outcome::Checkmate(winner));
        assert!(a.score > 90000);
    }
}

#[test]
fn opponent_uses_stationary_capture_to_win_material() {
    let g = Game::new(Position::from_fen("7k/8/1q6/8/8/8/1N6/K7 w - - 0 1").unwrap());
    let a = choose(
        &g,
        Limits {
            depth: 2,
            nodes: 8000,
        },
    )
    .unwrap();
    assert_eq!(a.best.unwrap().to_string(), "b2b6@");
}

#[test]
fn opponent_escapes_check_and_obeys_small_node_budget() {
    let g = Game::new(Position::from_fen("q6k/8/2B5/8/8/8/8/K7 w - - 0 1").unwrap());
    let a = choose(&g, Limits { depth: 4, nodes: 1 }).unwrap();
    assert!(a.nodes <= 1);
    let next = g.position().play(a.best.unwrap()).unwrap();
    assert!(!next.in_check(Color::White));
    assert!(
        choose(
            &g,
            Limits {
                depth: 0,
                nodes: 100
            }
        )
        .is_err()
    );
    assert!(choose(&g, Limits { depth: 2, nodes: 0 }).is_err());
}

#[test]
fn state_reports_moves_and_freezing_from_engine() {
    let s = Session::replay(None, &[]).unwrap();
    assert!(s.state_json().contains("\"turn\":\"black\""));
    assert!(s.state_json().contains("\"move\":\"a8c6\""));
    let frozen = Session::replay(Some("7k/8/8/8/3bR3/3NQ3/8/K7 w - - 0 1"), &[]).unwrap();
    assert!(frozen.state_json().contains("\"frozen\":true"));
    assert!(!frozen.state_json().contains("\"move\":\"e4f4\""));
}

#[test]
fn session_records_stationary_capture_and_promotions() {
    let s = Session::replay(Some("7k/8/1q6/8/8/8/1N6/K7 w - - 17 1"), &["b2b6@"]).unwrap();
    assert!(s.state_json().contains("\"stationary\":true"));
    assert_eq!(s.game.position().to_fen(), "7k/8/8/8/8/8/1N6/K7 b - - 0 1");
    for suffix in ["q", "b", "n", "r"] {
        let mv = format!("a7a8{suffix}");
        assert!(Session::replay(Some("7k/P7/8/8/8/8/8/7K w - - 0 1"), &[&mv]).is_ok());
    }
}

#[test]
fn ai_turn_obeys_human_color_and_terminal_state() {
    let mut s = Session::replay(None, &[]).unwrap();
    assert!(s.ai_turn("black", "balanced").is_err());
    assert!(s.ai_turn("purple", "balanced").is_err());
    assert!(s.ai_turn("white", "unknown").is_err());
    s.ai_turn("white", "relaxed").unwrap();
    assert_eq!(s.game.position().side, Color::White);
    let mut terminal = Session::replay(Some("7k/6Q1/5K2/8/8/8/8/8 b - - 0 1"), &[]).unwrap();
    assert!(terminal.ai_turn("white", "balanced").is_err());
    assert!(!terminal.state_json().contains("\"move\":"));
}
