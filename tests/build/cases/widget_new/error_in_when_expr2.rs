use zero_ui::properties::margin;
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        margin = 0;
        when *# {
            margin = 10;
        }
    };
}
