use african_chess::learning::{FEATURE_NAMES, Model, features};
use african_chess::opponent::{Limits, choose_with};
use african_chess::{Game, Position};

#[test]
fn model_roundtrip_is_strict_and_finite() {
    let m = Model::default();
    assert_eq!(Model::decode(&m.encode()).unwrap().weights, m.weights);
    assert!(Model::decode("garbage").is_err());
    assert!(Model::decode(&m.encode().replace("3.6", "NaN")).is_err());
    assert_eq!(FEATURE_NAMES.len(), m.weights.len());
}

#[test]
fn features_change_perspective_and_report_frozen_material() {
    let p = Position::from_fen("7k/8/8/8/3bR3/3NQ3/8/K7 w - - 0 1").unwrap();
    let mut other = p.clone();
    other.side = p.side.other();
    let a = features(&p);
    let b = features(&other);
    for i in 0..a.len() - 1 {
        assert!((a[i] + b[i]).abs() < 1e-6);
    }
    assert_eq!(a[a.len() - 1], 1.0);
    // MG + EG frozen counts for white elephant and giraffe, black bishop unfrozen.
    assert!((a[13] + a[18] - 1.0).abs() < 1e-6);
    assert!((a[11] + a[16] - 1.0).abs() < 1e-6);
    assert_eq!(a[12] + a[17], 0.0);
}

#[test]
fn trained_coefficients_affect_evaluation_and_search() {
    let p = Position::from_fen("7k/8/1q6/8/8/8/1N6/K7 w - - 0 1").unwrap();
    let m = Model::default();
    let mut changed = m.clone();
    changed.weights[4] = 15.0;
    changed.weights[9] = 15.0;
    assert!(changed.evaluate(&p) < m.evaluate(&p));
    let analysis = choose_with(
        &Game::new(p),
        Limits {
            depth: 2,
            nodes: 8000,
        },
        &|p| m.evaluate(p),
    )
    .unwrap();
    assert_eq!(analysis.best.unwrap().to_string(), "b2b6@");
}
