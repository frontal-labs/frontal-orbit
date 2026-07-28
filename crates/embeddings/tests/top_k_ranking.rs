use orbit_embeddings::top_k_by_similarity;

#[test]
fn ranks_candidates_by_similarity() {
    let query = vec![1.0, 0.0];
    let candidates = vec![
        ("most_similar".to_string(), vec![1.0, 0.0]),
        ("somewhat_similar".to_string(), vec![1.0, 1.0]),
        ("different".to_string(), vec![0.0, 1.0]),
        ("opposite".to_string(), vec![-1.0, 0.0]),
    ];
    let result = top_k_by_similarity(&query, candidates, 4);
    assert_eq!(result.len(), 4);
    assert_eq!(result[0].0, "most_similar");
    assert!(result[0].1 >= result[1].1);
    assert!(result[1].1 >= result[2].1);
    assert!(result[2].1 >= result[3].1);
}

#[test]
fn k_of_zero_returns_empty() {
    let query = vec![1.0, 0.0];
    let candidates = vec![("a".to_string(), vec![1.0, 0.0])];
    let result = top_k_by_similarity(&query, candidates, 0);
    assert!(result.is_empty());
}

#[test]
fn k_larger_than_candidate_count() {
    let query = vec![1.0, 0.0];
    let candidates = vec![("only".to_string(), vec![0.5, 0.5])];
    let result = top_k_by_similarity(&query, candidates, 10);
    assert_eq!(result.len(), 1);
}

#[test]
fn single_candidate() {
    let query = vec![1.0, 0.0];
    let candidates = vec![("single".to_string(), vec![1.0, 0.0])];
    let result = top_k_by_similarity(&query, candidates, 1);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "single");
    assert!((result[0].1 - 1.0).abs() < 1e-6);
}

#[test]
fn empty_candidates() {
    let query = vec![1.0, 0.0];
    let candidates: Vec<(String, Vec<f32>)> = vec![];
    let result = top_k_by_similarity(&query, candidates, 5);
    assert!(result.is_empty());
}

#[test]
fn all_candidates_equal() {
    let query = vec![1.0, 0.0];
    let candidates = vec![
        ("a".to_string(), vec![1.0, 0.0]),
        ("b".to_string(), vec![1.0, 0.0]),
        ("c".to_string(), vec![1.0, 0.0]),
    ];
    let result = top_k_by_similarity(&query, candidates, 3);
    assert_eq!(result.len(), 3);
    for (_, score) in &result {
        assert!((*score - 1.0).abs() < 1e-6);
    }
}

#[test]
fn returns_correct_number_up_to_k() {
    let query = vec![1.0, 0.0];
    let candidates = vec![
        ("one".to_string(), vec![1.0, 0.0]),
        ("two".to_string(), vec![0.5, 0.5]),
        ("three".to_string(), vec![0.0, 0.0]),
    ];
    assert_eq!(top_k_by_similarity(&query, candidates.clone(), 1).len(), 1);
    assert_eq!(top_k_by_similarity(&query, candidates.clone(), 2).len(), 2);
    assert_eq!(top_k_by_similarity(&query, candidates, 3).len(), 3);
}

#[test]
fn result_is_sorted_by_score_descending() {
    let query = vec![1.0, 0.0];
    let candidates = vec![
        ("low".to_string(), vec![0.0, 1.0]),
        ("high".to_string(), vec![1.0, 0.0]),
        ("mid".to_string(), vec![0.7, 0.7]),
    ];
    let result = top_k_by_similarity(&query, candidates, 3);
    assert_eq!(result[0].0, "high");
    assert_eq!(result[1].0, "mid");
    assert_eq!(result[2].0, "low");
}

#[test]
fn id_is_preserved_in_result() {
    let query = vec![1.0, 0.0];
    let candidates = vec![
        ("alpha".to_string(), vec![0.5, 0.5]),
        ("beta".to_string(), vec![1.0, 0.0]),
    ];
    let result = top_k_by_similarity(&query, candidates, 2);
    assert_eq!(result[0].0, "beta");
    assert_eq!(result[1].0, "alpha");
}

#[test]
fn query_with_zero_vector_assigns_zero_score_to_all() {
    let query = vec![0.0, 0.0];
    let candidates = vec![
        ("a".to_string(), vec![1.0, 0.0]),
        ("b".to_string(), vec![0.0, 1.0]),
    ];
    let result = top_k_by_similarity(&query, candidates, 2);
    assert_eq!(result.len(), 2);
    for (_, score) in &result {
        assert!((*score - 0.0).abs() < 1e-6);
    }
}
