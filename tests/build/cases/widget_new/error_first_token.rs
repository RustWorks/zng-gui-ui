use zng::{gesture::is_pressed, widget::Wgt, APP};

fn test_1() {
    let _ = Wgt! {
        =
    };
}

fn test_2() {
    let _ = Wgt! {
        when *#is_pressed {
            =
        }
    };
}

fn main() {
    let _scope = APP.minimal();
    test_1();
    test_2();
}
