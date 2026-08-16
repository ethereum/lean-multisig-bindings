use lean_multisig::{verify, Claim, SecretKey};

#[test]
fn signs_and_verifies_through_the_public_rust_crate() {
    let claim = Claim::new([42; 32], 5);
    let key = SecretKey::from_seed([1; 32], 0..=10).expect("valid test key");
    let signature = key.sign(&claim).expect("signature");

    verify(&signature, &[key.public_key()], &claim).expect("signature verifies");
}
