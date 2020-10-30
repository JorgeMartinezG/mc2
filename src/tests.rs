mod bar;
mod foo;

use bar::other_ops;
use foo::ops;

#[test]
fn test_add_two() {
    assert_eq!(ops::add_two(2), 4);
}

#[test]
fn test_add_three() {
    assert_eq!(other_ops::add_three(2), 5);
}
