/// Defines some useful extensions for functional programming which reorder the control flow.
pub mod functional;

#[macro_export]
macro_rules! assert_symbolic_eq {
    ($x:expr, $y:expr) => {
        assert!($x.iff(&$y).is_true())
    };
}

#[macro_export]
macro_rules! assert_symbolic_ne {
    ($x:expr, $y:expr) => {
        assert!(!$x.xor(&$y).is_false())
    };
}
