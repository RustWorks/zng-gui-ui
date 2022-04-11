#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use meval::eval_str;
use std::convert::TryInto;
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    //let rec = examples_util::record_profile("profile-calculator.json.gz", &[("example", "calculator")], |_| true);

    // zero_ui_view::run_same_process(app_main);
    app_main();

    //rec.finish();
}

fn app_main() {
    App::default().run_window(|ctx| {
        set_fallback_font(ctx);

        let calc = var(Calculator::default());
        window! {
            title = "Calculator";
            // zero_ui::widgets::inspector::show_bounds = true;
            resizable = false;
            auto_size = true;
            padding = 5;
            content = v_stack! {
                spacing = 5;
                items = widgets![
                    text! {
                        text = calc.map_ref(|c| c.text());
                        align = Align::RIGHT;
                        font_size = 32.pt();
                        color = calc.map_ref(|c| c.color());
                    },
                    controls(calc)
                ];
            };
        }
    })
}

fn controls(calc: RcVar<Calculator>) -> impl Widget {
    let bn = |c| btn(calc.clone(), c);
    let b_squre = btn_square(calc.clone());
    let b_sroot = btn_square_root(calc.clone());
    let b_clear = btn_clear(calc.clone());
    let b_back = btn_backspace(calc.clone());
    let b_equal = btn_eval(calc.clone());

    uniform_grid! {
        spacing = 2;
        columns = 4;
        font_size = 14.pt();
        items = widgets![
            b_squre,  b_sroot,  b_clear,  b_back,
            bn('7'),  bn('8'),  bn('9'),  bn('/'),
            bn('4'),  bn('5'),  bn('6'),  bn('*'),
            bn('1'),  bn('2'),  bn('3'),  bn('-'),
            bn('0'),  bn('.'),  b_equal,  bn('+'),
        ];
    }
}

fn btn_square(calc: RcVar<Calculator>) -> impl Widget {
    button! {
        on_click = hn!(|ctx, _| calc.modify(ctx.vars, |mut c|c.square()));
        content = text("x²");
    }
}

fn btn_square_root(calc: RcVar<Calculator>) -> impl Widget {
    button! {
        on_click = hn!(|ctx, _| calc.modify(ctx.vars, |mut c|c.square_root()));
        content = text("√x");
    }
}

fn btn_clear(calc: RcVar<Calculator>) -> impl Widget {
    button! {
        on_click = hn!(|ctx, _| calc.modify(ctx.vars, |mut c|c.clear()));
        click_shortcut = shortcut!(Escape);
        content = text("C");
    }
}

fn btn_backspace(calc: RcVar<Calculator>) -> impl Widget {
    button! {
        on_click = hn!(|ctx, _| calc.modify(ctx.vars, |mut c|c.backspace()));
        click_shortcut = shortcut!(Backspace);
        content = text("⌫");
    }
}

fn btn(calc: RcVar<Calculator>, c: char) -> impl Widget {
    button! {
        on_click = hn!(|ctx, _| {
            calc.modify(ctx.vars, move |mut b| b.push(c))
        });
        click_shortcut = {
            let shortcuts: Shortcuts = c.try_into().unwrap_or_default();
            assert!(!shortcuts.0.is_empty());
            shortcuts
        };
        content = text(c.to_string());
    }
}

fn btn_eval(calc: RcVar<Calculator>) -> impl Widget {
    button! {
        on_click = hn!(|ctx, _| calc.modify(ctx.vars, |mut c|c.eval()));
        click_shortcut = vec![shortcut!(Enter), shortcut!(NumpadEnter), shortcut!(Equals)];
        content = text("=");
    }
}

#[derive(Default, Clone, Debug)]
struct Calculator {
    buffer: Text,
    error: bool,
}
impl Calculator {
    pub fn text(&self) -> &Text {
        if self.buffer.is_empty() {
            static ZERO: Text = Text::from_static("0");
            &ZERO
        } else {
            &self.buffer
        }
    }

    pub fn color(&self) -> &Rgba {
        if self.error {
            &colors::RED
        } else {
            &colors::WHITE
        }
    }

    fn char_is_valid(c: char) -> bool {
        c.is_digit(10) || ['.', '+', '-', '*', '/'].contains(&c)
    }

    pub fn push(&mut self, c: char) {
        if !Self::char_is_valid(c) {
            return;
        }

        if self.error {
            self.buffer.clear();
            self.error = false;
        }

        if self.buffer.is_empty() && !c.is_digit(10) && c != '-' {
            let b = self.buffer.to_mut();
            b.push('0');
            b.push(c);
        } else {
            if !c.is_digit(10) && self.trailing_op() {
                self.buffer.to_mut().pop();
            }

            self.buffer.to_mut().push(c);
        }
    }

    fn trailing_op(&self) -> bool {
        self.buffer.chars().last().map(|c| !c.is_digit(10) && c != ')').unwrap_or(false)
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.error = false;
    }

    pub fn backspace(&mut self) {
        self.buffer.pop();
        self.error = false;
    }

    pub fn square(&mut self) {
        if self.error {
            self.clear()
        } else if !self.buffer.is_empty() {
            self.buffer = formatx!("({})^2", self.buffer)
        }
    }

    pub fn square_root(&mut self) {
        if self.error {
            self.clear()
        } else if !self.buffer.is_empty() {
            self.buffer = formatx!("sqrt({})", self.buffer)
        }
    }

    pub fn eval(&mut self) {
        use std::fmt::Write;

        let expr = if self.trailing_op() {
            &self.buffer[..self.buffer.len() - 1]
        } else {
            &self.buffer
        };

        if expr.is_empty() {
            self.buffer.clear();
            self.error = false;
        } else {
            match eval_str(expr) {
                Ok(new) => {
                    if new.is_finite() {
                        self.buffer.clear();
                        let _ = write!(&mut self.buffer.to_mut(), "{new}");
                        self.error = false;
                    } else {
                        eprintln!("Result not finite: {new}");
                        self.error = true;
                    }
                }
                Err(e) => {
                    eprintln!("{e}");
                    self.error = true;
                }
            }
        }
    }
}

/// set custom fallback font for the ⌫ symbol.
fn set_fallback_font(ctx: &mut WindowContext) {
    use zero_ui::core::text::*;

    let fonts = ctx.services.fonts();
    let und = lang!(und);
    if fonts
        .get_list(
            &FontNames::system_ui(&und),
            FontStyle::Normal,
            FontWeight::NORMAL,
            FontStretch::NORMAL,
            &und,
        )
        .iter()
        .all(|f| f.glyph_for_char('⌫').is_none())
    {
        // OS UI and fallback fonts do not support `⌫`, load custom font that does.

        static FALLBACK: &[u8] = include_bytes!("res/calculator/notosanssymbols2-regular-subset.ttf");
        let fallback = zero_ui::core::text::CustomFont::from_bytes("fallback", FontDataRef::from_static(FALLBACK), 0);

        fonts.register(fallback).unwrap();
        fonts.generics_mut().set_fallback(und, "fallback");
    }
}
