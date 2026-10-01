use african_chess::{Color, Kind, Position, square};

fn pos(fen: &str) -> Position {
    Position::from_fen(fen).unwrap()
}
fn moves(p: &Position, from: &str) -> Vec<String> {
    p.legal_moves()
        .iter()
        .filter(|m| m.from == square(from).unwrap())
        .map(ToString::to_string)
        .collect()
}

#[test]
fn black_starts_with_twenty_six_moves() {
    let p = Position::initial();
    assert_eq!(p.side, Color::Black);
    assert_eq!(p.legal_moves().len(), 26);
    assert_eq!(moves(&p, "a8"), vec!["a8a6", "a8b6", "a8c6"]);
}

#[test]
fn bishop_freezes_diagonal_and_orthogonal_enemies() {
    let p = pos("7k/8/8/8/3bR3/3NQ3/8/K7 w - - 0 1");
    for s in ["e4", "d3", "e3"] {
        assert!(p.frozen(square(s).unwrap()));
        assert!(moves(&p, s).is_empty());
    }
    assert!(!p.frozen(square("d4").unwrap()));
}

#[test]
fn frozen_bishops_keep_aura_and_capture_thaws_survivor() {
    let p = pos("7k/8/8/8/3bB3/8/8/K2Q4 w - - 0 1");
    assert!(p.frozen(square("d4").unwrap()));
    assert!(p.frozen(square("e4").unwrap()));
    let next = p.play_text("d1d4").unwrap();
    assert!(!next.frozen(square("e4").unwrap()));
    assert_eq!(
        next.piece(square("e4").unwrap()).unwrap().kind,
        Kind::Bishop
    );
}

#[test]
fn another_adjacent_bishop_keeps_a_piece_frozen() {
    let p = pos("7k/8/8/8/3bBb2/8/8/K2Q4 w - - 0 1");
    let next = p.play_text("d1d4").unwrap();
    assert!(next.frozen(square("e4").unwrap()));
}

#[test]
fn giraffe_captures_forward_without_relocating_and_consumes_turn() {
    let p = pos("7k/8/1q6/8/8/8/1N6/K7 w - - 17 1");
    let next = p.play_text("b2b6@").unwrap();
    assert!(next.piece(square("b6").unwrap()).is_none());
    assert_eq!(
        next.piece(square("b2").unwrap()).unwrap().kind,
        Kind::Giraffe
    );
    assert_eq!(next.halfmove, 0);
    assert_eq!(next.side, Color::Black);
}

#[test]
fn giraffe_cannot_reach_past_friendly_or_enemy_blocker() {
    let friendly = pos("7k/8/1q6/8/1P6/8/1N6/K7 w - - 0 1");
    assert!(!moves(&friendly, "b2").iter().any(|m| m.ends_with('@')));
    let enemy = pos("7k/8/1q6/8/1p6/8/1N6/K7 w - - 0 1");
    assert!(moves(&enemy, "b2").contains(&"b2b4@".into()));
    assert!(!moves(&enemy, "b2").contains(&"b2b6@".into()));
}

#[test]
fn black_giraffe_points_toward_rank_one_and_retains_knight_captures() {
    let p = pos("7k/1n6/8/2P5/8/1Q6/8/K7 b - - 0 1");
    assert!(moves(&p, "b7").contains(&"b7b3@".into()));
    assert!(moves(&p, "b7").contains(&"b7c5".into()));
}

#[test]
fn frozen_giraffe_cannot_capture_or_give_check() {
    let p = pos("1k6/8/8/8/8/2b5/1N6/K7 w - - 0 1");
    assert!(moves(&p, "b2").is_empty());
    assert!(!p.in_check(Color::Black));
}

#[test]
fn elephant_has_twenty_four_destinations_and_can_jump() {
    let p = pos("7k/8/8/8/3R4/8/8/K7 w - - 0 1");
    assert_eq!(moves(&p, "d4").len(), 24);
    let blocked = pos("7k/8/8/8/3RPp2/8/8/K7 w - - 0 1");
    assert!(moves(&blocked, "d4").contains(&"d4f4".into()));
    assert!(!moves(&blocked, "d4").contains(&"d4e4".into()));
    assert!(!moves(&blocked, "d4").contains(&"d4g4".into()));
}

#[test]
fn bishop_and_queen_cannot_jump() {
    let p = pos("7k/8/8/8/8/2P5/1B6/KQ6 w - - 0 1");
    assert!(!moves(&p, "b2").contains(&"b2d4".into()));
    assert!(!moves(&p, "b1").contains(&"b1b3".into()));
}

#[test]
fn thawing_an_enemy_attacker_cannot_expose_own_king() {
    let p = pos("q6k/1B6/8/8/8/8/8/K7 w - - 0 1");
    assert!(!p.in_check(Color::White));
    assert!(!moves(&p, "b7").contains(&"b7c6".into()));
    assert!(moves(&p, "b7").contains(&"b7a8".into()));
}

#[test]
fn freeze_can_save_king_from_check() {
    let p = pos("q6k/8/2B5/8/8/8/8/K7 w - - 0 1");
    assert!(p.in_check(Color::White));
    let next = p.play_text("c6b7").unwrap();
    assert!(!next.in_check(Color::White));
}

#[test]
fn frozen_king_is_not_automatically_in_check() {
    let p = pos("7k/8/8/8/8/8/b6P/K7 w - - 0 1");
    assert!(p.frozen(square("a1").unwrap()));
    assert!(!p.in_check(Color::White));
    assert!(moves(&p, "a1").is_empty());
}

#[test]
fn giraffe_ray_gives_check_but_king_is_never_captured() {
    let p = pos("1k6/8/8/8/8/8/1N6/K7 b - - 0 1");
    assert!(p.in_check(Color::Black));
    assert!(!moves(&p, "b8").contains(&"b8b7".into()));
    let malformed_turn = pos("1k6/8/8/8/8/8/1N6/K7 w - - 0 1");
    assert!(!moves(&malformed_turn, "b2").contains(&"b2b8@".into()));
}

#[test]
fn castling_moves_elephant_and_updates_rights() {
    let p = pos("4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1");
    let next = p.play_text("e1g1").unwrap();
    assert_eq!(
        next.piece(square("f1").unwrap()).unwrap().kind,
        Kind::Elephant
    );
    assert!(next.piece(square("h1").unwrap()).is_none());
    assert_eq!(next.to_fen(), "4k3/8/8/8/8/8/8/R4RK1 b - - 1 1");
}

#[test]
fn castling_forbidden_through_check_or_with_frozen_elephant() {
    let attacked = pos("4kq2/8/8/8/8/8/8/4K2R w K - 0 1");
    assert!(!moves(&attacked, "e1").contains(&"e1g1".into()));
    let frozen = pos("4k3/8/8/8/8/8/7b/4K2R w K - 0 1");
    assert!(!moves(&frozen, "e1").contains(&"e1g1".into()));
}

#[test]
fn en_passant_is_illegal_even_with_legacy_fen_metadata() {
    for (fen, from, capture) in [
        ("7k/8/8/3pP3/8/8/8/K7 w - d6 0 1", "e5", "e5d6"),
        ("7k/8/8/8/3Pp3/8/8/K7 b - d3 0 1", "e4", "e4d3"),
    ] {
        let p = pos(fen);
        assert!(!moves(&p, from).contains(&capture.into()));
        assert!(p.play_text(capture).is_err());
        assert_eq!(p.to_fen().split_whitespace().nth(3), Some("-"));
    }
}

#[test]
fn pawn_double_steps_do_not_enable_en_passant() {
    for (fen, double_step, capture) in [
        ("7k/3p4/8/4P3/8/8/8/K7 b - - 0 1", "d7d5", "e5d6"),
        ("7k/8/8/8/4p3/8/3P4/K7 w - - 0 1", "d2d4", "e4d3"),
    ] {
        let next = pos(fen).play_text(double_step).unwrap();
        assert!(next.play_text(capture).is_err());
        assert_eq!(next.to_fen().split_whitespace().nth(3), Some("-"));
    }
}

#[test]
fn pawns_still_capture_occupied_diagonals() {
    for (fen, capture, destination) in [
        ("7k/8/3p4/4P3/8/8/8/K7 w - - 0 1", "e5d6", "d6"),
        ("7k/8/8/8/4p3/3P4/8/K7 b - - 0 1", "e4d3", "d3"),
    ] {
        let p = pos(fen);
        let next = p.play_text(capture).unwrap();
        assert_eq!(
            next.piece(square(destination).unwrap()).unwrap().color,
            p.side
        );
    }
}

#[test]
fn en_passant_cannot_uncover_check() {
    let p = pos("7k/8/8/K2pP2q/8/8/8/8 w - d6 0 1");
    assert!(!moves(&p, "e5").contains(&"e5d6".into()));
}

#[test]
fn promotion_includes_all_variant_pieces() {
    let p = pos("7k/P7/8/8/8/8/8/7K w - - 0 1");
    let mut actual = moves(&p, "a7");
    actual.sort();
    assert_eq!(actual, vec!["a7a8b", "a7a8n", "a7a8q", "a7a8r"]);
    assert_eq!(
        p.play_text("a7a8n")
            .unwrap()
            .piece(square("a8").unwrap())
            .unwrap()
            .kind,
        Kind::Giraffe
    );
}

#[test]
fn fen_roundtrip_and_bad_inputs_fail_safely() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1";
    assert_eq!(pos(fen).to_fen(), fen);
    for s in [
        "",
        "8/8/8/8/8/8/8/8 w - - 0 1",
        "7k/8/8/8/8/8/8/K7 x - - 0 1",
        "7k/8/8/8/8/8/8/K7 w X - 0 1",
        "7k/8/8/8/8/8/8/K7 w - a4 0 1",
        "7k/8/8/8/8/8/8/K7 w - - 0 0",
        "7k/8/8/8/8/8/8/K7 w - - 0 1 extra",
        "王",
    ] {
        assert!(Position::from_fen(s).is_err(), "accepted {s}");
    }
    let p = Position::initial();
    for s in ["", "a7a9", "e2e4", "王", "b8b1@"] {
        assert!(p.play_text(s).is_err());
    }
}
