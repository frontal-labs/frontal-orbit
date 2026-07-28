use orbit_training::{StyleProfile, StyleTrainer};

#[test]
fn style_score_with_matching_profile() {
    let trainer = StyleTrainer;
    let profile = StyleProfile {
        sample_count: 1,
        preferred_indent: "spaces:4".to_string(),
        max_line_length: 100,
        snake_case_ratio: 1.0,
        notes: vec![],
    };
    let code = "fn parse_input() {\n    let user_name = \"test\";\n}\n";
    let score = trainer.style_score(&profile, code);
    assert!(score > 0.0);
    assert!(score <= 1.0);
}

#[test]
fn style_score_all_conforming() {
    let trainer = StyleTrainer;
    let profile = StyleProfile {
        sample_count: 1,
        preferred_indent: "spaces:4".to_string(),
        max_line_length: 100,
        snake_case_ratio: 1.0,
        notes: vec![],
    };
    let code = "fn alpha_beta() {\n    let gamma_delta = 10;\n    let epsilon_zeta = 20;\n}\n";
    let score = trainer.style_score(&profile, code);
    assert!(score > 0.5);
}

#[test]
fn style_score_none_conforming() {
    let trainer = StyleTrainer;
    let profile = StyleProfile {
        sample_count: 1,
        preferred_indent: "tabs".to_string(),
        max_line_length: 10,
        snake_case_ratio: 0.0,
        notes: vec![],
    };
    let code = "fn alpha_beta() {\n    let gamma_delta = 10;\n}\n";
    let score = trainer.style_score(&profile, code);
    assert!(score < 0.8);
}

#[test]
fn style_score_empty_code() {
    let trainer = StyleTrainer;
    let profile = StyleProfile::default();
    let score = trainer.style_score(&profile, "");
    assert!((0.0_f32 - score).abs() < 0.01 || score >= 0.0);
}

#[test]
fn style_score_no_identifiers() {
    let trainer = StyleTrainer;
    let profile = StyleProfile::default();
    let code = "    \n    \n";
    let score = trainer.style_score(&profile, code);
    assert!(score >= 0.0);
    assert!(score <= 1.0);
}

#[test]
fn style_score_tabs_vs_spaces() {
    let trainer = StyleTrainer;
    let tabs_profile = StyleProfile {
        sample_count: 1,
        preferred_indent: "tabs".to_string(),
        max_line_length: 100,
        snake_case_ratio: 0.5,
        notes: vec![],
    };
    let spaces_profile = StyleProfile {
        sample_count: 1,
        preferred_indent: "spaces:4".to_string(),
        max_line_length: 100,
        snake_case_ratio: 0.5,
        notes: vec![],
    };
    let tabs_code = "fn test() {\n\tlet x = 1;\n}\n";
    let spaces_code = "fn test() {\n    let x = 1;\n}\n";
    let tabs_score = trainer.style_score(&tabs_profile, tabs_code);
    let spaces_score = trainer.style_score(&spaces_profile, spaces_code);
    assert!(tabs_score > 0.0);
    assert!(spaces_score > 0.0);
}

#[test]
fn style_score_clamped_to_zero_one() {
    let trainer = StyleTrainer;
    let profile = StyleProfile::default();
    let code = "a";
    let score = trainer.style_score(&profile, code);
    assert!((0.0..=1.0).contains(&score));
}

#[test]
fn style_score_prefers_conformant_over_non_conformant() {
    let trainer = StyleTrainer;
    let profile = StyleProfile {
        sample_count: 1,
        preferred_indent: "spaces:4".to_string(),
        max_line_length: 60,
        snake_case_ratio: 1.0,
        notes: vec![],
    };
    let good = "fn alpha_beta() {\n    let gamma_delta = 10;\n}\n";
    let bad = "fn alphaBeta(){\n\tlet gammaDelta=10;\n}\n";
    assert!(trainer.style_score(&profile, good) > trainer.style_score(&profile, bad));
}
