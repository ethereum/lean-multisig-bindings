use lean_multisig_java_native::{
    lms_buffer_free, lms_secret_key_destroy, lms_secret_key_from_seed, lms_secret_key_sign,
    lms_setup, lms_signature_destroy, lms_signature_to_bytes, LmsBuffer,
};
use std::ptr;

const OK: i32 = 0;

#[test]
fn c_abi_signs_and_serializes_a_claim() {
    std::thread::Builder::new()
        .name("native-abi-test".to_owned())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| unsafe {
            assert_eq!(lms_setup(), OK);

            let mut key = ptr::null_mut();
            assert_eq!(
                lms_secret_key_from_seed([7; 32].as_ptr(), 32, 100, 115, &mut key),
                OK
            );

            let mut signature = ptr::null_mut();
            assert_eq!(
                lms_secret_key_sign(key, [42; 32].as_ptr(), 32, 100, &mut signature),
                OK
            );

            let mut bytes = LmsBuffer::default();
            assert_eq!(lms_signature_to_bytes(signature, &mut bytes), OK);
            assert!(!bytes.data.is_null());
            assert!(bytes.len > 0);

            lms_buffer_free(bytes.data, bytes.len);
            lms_signature_destroy(signature);
            lms_secret_key_destroy(key);
        })
        .expect("test thread starts")
        .join()
        .expect("test thread completes");
}
