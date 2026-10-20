#![cfg(target_os = "linux")]

use glib::prelude::*;

#[test]
fn string_variant_iteration_uses_valid_c_output_pointers() {
    let variant = ["first", "middle", "last"].to_variant();
    assert_eq!(
        variant.array_iter_str().unwrap().collect::<Vec<_>>(),
        ["first", "middle", "last"]
    );
    let mut iter = variant.array_iter_str().unwrap();
    assert_eq!(iter.next(), Some("first"));
    assert_eq!(iter.next_back(), Some("last"));
    assert_eq!(iter.next(), Some("middle"));
    assert_eq!(iter.next_back(), None);
    assert_eq!(variant.array_iter_str().unwrap().nth(1), Some("middle"));
    assert_eq!(variant.array_iter_str().unwrap().last(), Some("last"));
}
