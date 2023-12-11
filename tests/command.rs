use zero_ui::{
    keyboard::{Key, KeyCode},
    prelude::*,
    wgt_prelude::*,
};

#[test]
fn notify() {
    let mut app = APP.defaults().run_headless(false);
    app.open_window(listener_window(false));

    let cmd = FOO_CMD;
    cmd.notify();

    let _ = app.update(false);

    assert_eq!(&*TEST_TRACE.read(), &vec!["no-scope / App".to_owned()]);
}

#[test]
fn notify_scoped() {
    let mut app = APP.defaults().run_headless(false);
    let window_id = app.open_window(listener_window(false));

    let cmd = FOO_CMD;
    let cmd_scoped = cmd.scoped(window_id);

    cmd_scoped.notify();

    let _ = app.update(false);

    assert_eq!(&*TEST_TRACE.read(), &vec![format!("scoped-win / Window({window_id:?})")]);
}

#[test]
fn shortcut() {
    let mut app = APP.defaults().run_headless(false);
    let window_id = app.open_window(listener_window(false));

    FOO_CMD.shortcut().set(shortcut!('F')).unwrap();

    app.press_key(window_id, KeyCode::KeyF, Key::Char('F'));

    let widget_id = WidgetId::named("test-widget");
    // because we target the scoped first.
    assert_eq!(&*TEST_TRACE.read(), &vec![format!("scoped-wgt / Widget({widget_id:?})")]);
}

#[test]
fn shortcut_with_focused_scope() {
    let mut app = APP.defaults().run_headless(false);
    let window_id = app.open_window(listener_window(true));

    FOO_CMD.shortcut().set(shortcut!('F')).unwrap();

    app.press_key(window_id, KeyCode::KeyF, Key::Char('F'));

    let trace = TEST_TRACE.read();
    let widget_id = WidgetId::named("other-widget");
    assert_eq!(1, trace.len()); // because we target the focused first.
    assert_eq!(&trace[0], &format!("scoped-wgt / Widget({widget_id:?})"));
}

#[test]
fn shortcut_scoped() {
    let mut app = APP.defaults().run_headless(false);
    let window_id = app.open_window(listener_window(false));

    FOO_CMD.shortcut().set(shortcut!('F')).unwrap();
    FOO_CMD.scoped(window_id).shortcut().set(shortcut!('G')).unwrap();

    app.press_key(window_id, KeyCode::KeyG, Key::Char('G'));

    {
        let mut trace = TEST_TRACE.write();
        assert_eq!(&*trace, &vec![format!("scoped-win / Window({window_id:?})")]);
        trace.clear();
    }

    app.press_key(window_id, KeyCode::KeyF, Key::Char('F'));

    let widget_id = WidgetId::named("test-widget");
    assert_eq!(&*TEST_TRACE.read(), &vec![format!("scoped-wgt / Widget({widget_id:?})")]);
}

async fn listener_window(focused_wgt: bool) -> WindowRoot {
    fn foo_handler() -> impl UiNode {
        let mut _handle = None;
        let mut _handle_scoped = None;
        let mut _handle_scoped_wgt = None;
        match_node_leaf(move |op| match op {
            UiNodeOp::Init => {
                _handle = Some(FOO_CMD.subscribe(true));
                _handle_scoped = Some(FOO_CMD.scoped(WINDOW.id()).subscribe(true));
                _handle_scoped_wgt = Some(FOO_CMD.scoped(WIDGET.id()).subscribe(true));
            }
            UiNodeOp::Deinit => {
                _handle = None;
                _handle_scoped = None;
                _handle_scoped_wgt = None;
            }
            UiNodeOp::Event { update } => {
                if let Some(args) = FOO_CMD.on(update) {
                    args.handle(|args| {
                        TEST_TRACE.write().push(format!("no-scope / {:?}", args.scope));
                    });
                }

                if let Some(args) = FOO_CMD.scoped(WINDOW.id()).on(update) {
                    args.handle(|args| {
                        TEST_TRACE.write().push(format!("scoped-win / {:?}", args.scope));
                    });
                }

                if let Some(args) = FOO_CMD.scoped(WIDGET.id()).on(update) {
                    args.handle(|args| {
                        TEST_TRACE.write().push(format!("scoped-wgt / {:?}", args.scope));
                    });
                }
            }
            _ => {}
        })
    }

    Window! {
        zero_ui::core::widget_base::parallel = false;
        child = Stack! {
            direction = StackDirection::top_to_bottom();
            children = ui_vec![
                Container! {
                    id = "test-widget";
                    size = (100, 100);
                    child = foo_handler();
                },
                Container! {
                    id = "other-widget";
                    size = (100, 100);
                    focusable = focused_wgt;
                    child = foo_handler();
                }
            ];
        }
    }
}

command! {
    pub static FOO_CMD;
}

app_local! {
    static TEST_TRACE: Vec<String> = const { vec![] };
}
