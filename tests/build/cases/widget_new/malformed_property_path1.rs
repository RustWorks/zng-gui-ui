use zng::{widget::Wgt, APP};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        _ = 0;
    };
}
