use super::*;

const CA: &str = include_str!("fixtures/ca.pem");
const OTHER: &str = include_str!("fixtures/other.pem");
const SERVER: &str = include_str!("fixtures/server.pem");
const CLIENT: &str = include_str!("fixtures/client.pem");
const CLIENT_KEY: &str = include_str!("fixtures/client-pkcs8.key");

#[test]
fn the_fingerprint_is_the_sha256_of_the_authority() {
    assert_eq!(
        authority_fingerprint(CA).unwrap(),
        "a6e6bd0f4910eef14f02d592bebef0afc97d812df2d5e353542459683430f772"
    );
    assert_eq!(authority_fingerprint("no certificate"), None);
    assert!(certificates("").is_empty());
}

#[test]
fn a_server_is_trusted_only_with_a_certificate_of_its_authority_whatever_its_names() {
    let chain = certificates(SERVER);
    assert!(issued_by(CA, &chain));
    assert!(!issued_by(OTHER, &chain));
    assert!(!issued_by(CA, &[]));
    assert!(!issued_by("garbage", &chain));
}

#[test]
fn the_settings_of_a_link_need_a_real_authority_and_a_real_key() {
    assert!(pinned(CA, None).is_ok());
    assert!(pinned(CA, Some((CLIENT, CLIENT_KEY))).is_ok());
    assert_eq!(pinned("garbage", None).err(), Some(ClientError::NotANas));
    assert_eq!(
        pinned(CA, Some((CLIENT, "not a key"))).err(),
        Some(ClientError::SecretUnavailable)
    );
    assert!(capturing().is_ok());
}
