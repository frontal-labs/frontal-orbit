use orbit_embeddings::cosine_similarity;

#[test]
fn identical_vectors() {
    let v = vec![1.0, 0.0, 0.0];
    let similarity = cosine_similarity(&v, &v);
    assert!((similarity - 1.0).abs() < 1e-6);
}

#[test]
fn opposite_vectors() {
    let a = vec![1.0, 0.0];
    let b = vec![-1.0, 0.0];
    let similarity = cosine_similarity(&a, &b);
    assert!((similarity - (-1.0)).abs() < 1e-6);
}

#[test]
fn orthogonal_vectors() {
    let a = vec![1.0, 0.0];
    let b = vec![0.0, 1.0];
    let similarity = cosine_similarity(&a, &b);
    assert!(similarity.abs() < 1e-6);
}

#[test]
fn parallel_vectors_same_direction() {
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![2.0, 4.0, 6.0];
    let similarity = cosine_similarity(&a, &b);
    assert!((similarity - 1.0).abs() < 1e-6);
}

#[test]
fn empty_vectors_return_zero() {
    let a: Vec<f32> = vec![];
    let b: Vec<f32> = vec![];
    let similarity = cosine_similarity(&a, &b);
    assert!((similarity - 0.0).abs() < 1e-6);
}

#[test]
fn mismatched_dimensions_return_zero() {
    let a = vec![1.0, 0.0];
    let b = vec![1.0];
    let similarity = cosine_similarity(&a, &b);
    assert!((similarity - 0.0).abs() < 1e-6);
}

#[test]
fn zero_vectors_return_zero() {
    let a = vec![0.0, 0.0, 0.0];
    let b = vec![0.0, 0.0, 0.0];
    let similarity = cosine_similarity(&a, &b);
    assert!((similarity - 0.0).abs() < 1e-6);
}

#[test]
fn one_vector_is_zero_returns_zero() {
    let a = vec![1.0, 2.0];
    let b = vec![0.0, 0.0];
    let similarity = cosine_similarity(&a, &b);
    assert!((similarity - 0.0).abs() < 1e-6);
}

#[test]
fn single_element_vectors_same_sign() {
    let a = vec![3.0];
    let b = vec![5.0];
    let similarity = cosine_similarity(&a, &b);
    assert!((similarity - 1.0).abs() < 1e-6);
}

#[test]
fn single_element_vectors_opposite_sign() {
    let a = vec![3.0];
    let b = vec![-5.0];
    let similarity = cosine_similarity(&a, &b);
    assert!((similarity - (-1.0)).abs() < 1e-6);
}

#[test]
fn symmetric_result() {
    let a = vec![1.0, 2.0, 3.0, 4.0];
    let b = vec![4.0, 3.0, 2.0, 1.0];
    let ab = cosine_similarity(&a, &b);
    let ba = cosine_similarity(&b, &a);
    assert!((ab - ba).abs() < 1e-6);
}

#[test]
fn known_cosine_value() {
    let a = vec![1.0, 0.0];
    let b = vec![1.0, 1.0];
    let similarity = cosine_similarity(&a, &b);
    let expected = 1.0 / 2.0_f32.sqrt();
    assert!((similarity - expected).abs() < 1e-6);
}

#[allow(clippy::cast_precision_loss)]
#[test]
fn large_vectors_produce_valid_results() {
    let a: Vec<f32> = (0..1000).map(|i| i as f32).collect();
    let b: Vec<f32> = (0..1000).map(|i| (1000 - i) as f32).collect();
    let similarity = cosine_similarity(&a, &b);
    assert!((-1.0..=1.0).contains(&similarity));
}

#[test]
fn proportional_vectors_have_positive_similarity() {
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![2.0, 4.0, 6.0];
    let similarity = cosine_similarity(&a, &b);
    assert!((similarity - 1.0).abs() < 1e-6);
}

#[test]
fn moderate_values_produce_cosine_between_minus_one_and_one() {
    let a = vec![0.1, 0.2, 0.3, 0.4, 0.5];
    let b = vec![0.5, 0.4, 0.3, 0.2, 0.1];
    let similarity = cosine_similarity(&a, &b);
    assert!((-1.0..1.0).contains(&similarity));
    assert!(similarity > 0.0);
}
