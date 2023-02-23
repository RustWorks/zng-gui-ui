#![cfg(test)]

use super::*;

mod any {
    use super::*;

    #[test]
    fn downcast_ref_rc() {
        let any_var = var(true).boxed_any();
        assert!(any_var.as_any().downcast_ref::<ArcVar<bool>>().is_some())
    }

    #[test]
    fn downcast_ref_boxed() {
        let any_var = var(true).boxed().boxed_any();
        assert!(any_var.as_any().downcast_ref::<ArcVar<bool>>().is_some())
    }

    #[test]
    fn downcast_ref_context_var() {
        context_var! {
            static FOO_VAR: bool = true;
        }
        let any_var = FOO_VAR.boxed_any();
        assert!(any_var.as_any().downcast_ref::<ContextVar<bool>>().is_some());
    }

    #[test]
    fn downcast_double_boxed() {
        let any_var = var(true).boxed_any().double_boxed_any();
        assert!(any_var.downcast_ref::<BoxedVar<bool>>().is_some())
    }

    #[test]
    fn downcast_rc() {
        let any_var = var(true).boxed_any();
        let any_box = any_var.as_any();
        assert!(any_box.downcast_ref::<ArcVar<bool>>().is_some());
    }

    #[test]
    fn downcast_boxed() {
        let any_var = var(true).boxed().boxed_any();
        let any_box = any_var.as_any();
        assert!(any_box.downcast_ref::<ArcVar<bool>>().is_some());
    }
}

mod bindings {
    use super::*;
    use crate::app::App;
    use crate::text::ToText;

    #[test]
    fn one_way_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::minimal().run_headless(false);

        a.bind_map(&b, |a| a.to_text()).perm();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(20);

        update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
                assert_eq!(Some(20i32), a.get_new());
                assert_eq!(Some("20".to_text()), b.get_new());
            },
            false,
        );
        assert_eq!(1, update_count);

        a.set(13);

        update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
                assert_eq!(Some(13i32), a.get_new());
                assert_eq!(Some("13".to_text()), b.get_new());
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn two_way_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::minimal().run_headless(false);

        a.bind_map_bidi(&b, |a| a.to_text(), |b| b.parse().unwrap()).perm();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(20);

        update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
                assert_eq!(Some(20i32), a.get_new());
                assert_eq!(Some("20".to_text()), b.get_new());
            },
            false,
        );
        assert_eq!(1, update_count);

        b.set("55");

        update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
                assert_eq!(Some("55".to_text()), b.get_new());
                assert_eq!(Some(55i32), a.get_new());
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn one_way_filtered_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::minimal().run_headless(false);

        a.bind_filter_map(&b, |a| if *a == 13 { None } else { Some(a.to_text()) }).perm();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(20);

        update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
                assert_eq!(Some(20i32), a.get_new());
                assert_eq!(Some("20".to_text()), b.get_new());
            },
            false,
        );
        assert_eq!(1, update_count);

        a.set(13);

        update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
                assert_eq!(Some(13i32), a.get_new());
                assert_eq!("20".to_text(), b.get());
                assert!(!b.is_new());
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn two_way_filtered_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::minimal().run_headless(false);

        a.bind_filter_map_bidi(&b, |a| Some(a.to_text()), |b| b.parse().ok()).perm();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(20);

        update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
                assert_eq!(Some(20i32), a.get_new());
                assert_eq!(Some("20".to_text()), b.get_new());
            },
            false,
        );
        assert_eq!(1, update_count);

        b.set("55");

        update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
                assert_eq!(Some("55".to_text()), b.get_new());
                assert_eq!(Some(55i32), a.get_new());
            },
            false,
        );
        assert_eq!(1, update_count);

        b.set("not a i32");

        update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
                assert_eq!(Some("not a i32".to_text()), b.get_new());
                assert_eq!(55i32, a.get());
                assert!(!a.is_new());
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn binding_chain() {
        let a = var(0);
        let b = var(0);
        let c = var(0);
        let d = var(0);

        let mut app = App::minimal().run_headless(false);

        a.bind_map(&b, |a| *a + 1).perm();
        b.bind_map(&c, |b| *b + 1).perm();
        c.bind_map(&d, |c| *c + 1).perm();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(20);

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;

                assert_eq!(Some(20), a.get_new());
                assert_eq!(Some(21), b.get_new());
                assert_eq!(Some(22), c.get_new());
                assert_eq!(Some(23), d.get_new());
            },
            false,
        );
        assert_eq!(1, update_count);

        a.set(30);

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;

                assert_eq!(Some(30), a.get_new());
                assert_eq!(Some(31), b.get_new());
                assert_eq!(Some(32), c.get_new());
                assert_eq!(Some(33), d.get_new());
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn binding_bidi_chain() {
        let a = var(0);
        let b = var(0);
        let c = var(0);
        let d = var(0);

        let mut app = App::minimal().run_headless(false);

        a.bind_bidi(&b).perm();
        b.bind_bidi(&c).perm();
        c.bind_bidi(&d).perm();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(20);

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;

                assert_eq!(Some(20), a.get_new());
                assert_eq!(Some(20), b.get_new());
                assert_eq!(Some(20), c.get_new());
                assert_eq!(Some(20), d.get_new());
            },
            false,
        );
        assert_eq!(1, update_count);

        d.set(30);

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;

                assert_eq!(Some(30), a.get_new());
                assert_eq!(Some(30), b.get_new());
                assert_eq!(Some(30), c.get_new());
                assert_eq!(Some(30), d.get_new());
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn binding_drop() {
        let a = var(1);
        let b = var(1);

        let mut app = App::minimal().run_headless(false);

        let handle = a.bind_map(&b, |i| *i + 1);

        a.set(10);

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;

                assert_eq!(Some(10), a.get_new());
                assert_eq!(Some(11), b.get_new());
            },
            false,
        );
        assert_eq!(1, update_count);

        drop(handle);

        a.set(100);

        update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;

                assert_eq!(Some(100), a.get_new());
                assert!(!b.is_new());
                assert_eq!(11, b.get());
            },
            false,
        );
        assert_eq!(1, update_count);

        assert_eq!(1, a.strong_count());
        assert_eq!(1, b.strong_count());
    }
}

mod context {
    use when::AnyWhenVarBuilder;

    use crate::{app::*, context::*, text::*, var::*, widget_instance::*, *};

    context_var! {
        static TEST_VAR: Text = "";
    }

    static PROBE_ID: StaticStateId<Text> = StaticStateId::new_unique();

    #[property(CONTEXT, default(TEST_VAR))]
    fn test_prop(child: impl UiNode, value: impl IntoVar<Text>) -> impl UiNode {
        with_context_var(child, TEST_VAR, value)
    }

    #[property(CONTEXT)]
    fn probe(child: impl UiNode, var: impl IntoVar<Text>) -> impl UiNode {
        #[ui_node(struct ProbeNode {
            child: impl UiNode,
            var: impl Var<Text>,
        })]
        impl UiNode for ProbeNode {
            fn init(&mut self, ctx: &mut WidgetContext) {
                ctx.app_state.set(&PROBE_ID, self.var.get());
                self.child.init(ctx);
            }
        }
        ProbeNode {
            child,
            var: var.into_var(),
        }
    }

    #[property(EVENT)]
    fn on_init(child: impl UiNode, handler: impl handler::WidgetHandler<()>) -> impl UiNode {
        #[ui_node(struct OnInitNode {
            child: impl UiNode,
            handler: impl handler::WidgetHandler<()>,
        })]
        impl UiNode for OnInitNode {
            fn init(&mut self, ctx: &mut WidgetContext) {
                self.child.init(ctx);
                self.handler.event(ctx, &());
            }

            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                self.child.update(ctx, updates);
                self.handler.update(ctx);
            }
        }
        OnInitNode { child, handler }
    }

    #[widget($crate::var::tests::context::test_wgt)]
    mod test_wgt {
        use super::*;

        inherit!(crate::widget_base::base);

        properties! {
            pub crate::widget_base::child;
        }

        fn include(wgt: &mut widget_builder::WidgetBuilder) {
            wgt.push_build_action(|wgt| {
                if let Some(child) = wgt.capture_ui_node(property_id!(self::child)) {
                    wgt.set_child(child);
                }
            });
        }
    }

    fn test_app(app: AppExtended<impl AppExtension>, root: impl UiNode) -> HeadlessApp {
        test_log();

        use crate::window::*;
        let mut app = app.run_headless(false);
        WINDOWS.open(move |_| crate::window::Window::new_test(root));
        let _ = app.update(false);
        app
    }

    #[test]
    fn context_var_basic() {
        let mut test = test_app(
            App::default(),
            test_wgt! {
                test_prop = "test!";

                child = test_wgt! {
                    probe = TEST_VAR;
                }
            },
        );

        assert_eq!(test.ctx().app_state.get(&PROBE_ID), Some(&Text::from("test!")));
    }

    #[test]
    fn context_var_map() {
        let mut test = test_app(
            App::default(),
            test_wgt! {
                test_prop = "test!";

                child = test_wgt! {
                    probe = TEST_VAR.map(|t| formatx!("map {t}"));
                }
            },
        );

        assert_eq!(test.ctx().app_state.get(&PROBE_ID), Some(&Text::from("map test!")));
    }

    #[test]
    fn context_var_map_cloned() {
        let app = App::default();

        // mapped context var should depend on the context.

        let mapped = TEST_VAR.map(|t| formatx!("map {t}"));
        use self::test_prop as test_prop_a;
        use self::test_prop as test_prop_b;

        let mut test = test_app(
            app,
            test_wgt! {
                test_prop_a = "A!";

                child = test_wgt! {
                    probe = mapped.clone();
                    test_prop_b = "B!";

                    child = test_wgt! {
                        probe = mapped;
                    }
                }
            },
        );

        assert_eq!(test.ctx().app_state.get(&PROBE_ID), Some(&Text::from("map B!")));
    }

    #[test]
    fn context_var_map_cloned3() {
        let app = App::default();
        // mapped context var should depend on the context.

        let mapped = TEST_VAR.map(|t| formatx!("map {t}"));
        let mut test = test_app(
            app,
            test_wgt! {
                test_prop = "A!";

                child = test_wgt! {
                    probe = mapped.clone();
                    test_prop = "B!";

                    child = test_wgt! {
                        probe = mapped.clone();
                        test_prop = "C!";

                        child = test_wgt! {
                            probe = mapped;
                            test_prop = "D!";
                        }
                    }
                }
            },
        );

        assert_eq!(test.ctx().app_state.get(&PROBE_ID), Some(&Text::from("map C!")));
    }

    #[test]
    fn context_var_map_not_cloned() {
        let app = App::default();

        // sanity check for `context_var_map_cloned`

        use self::test_prop as test_prop_a;
        use self::test_prop as test_prop_b;

        let mut test = test_app(
            app,
            test_wgt! {
                test_prop_a = "A!";

                child = test_wgt! {
                    probe = TEST_VAR.map(|t| formatx!("map {t}"));
                    test_prop_b = "B!";

                    child = test_wgt! {
                        probe = TEST_VAR.map(|t| formatx!("map {t}"));
                    }
                }
            },
        );

        assert_eq!(test.ctx().app_state.get(&PROBE_ID), Some(&Text::from("map B!")));
    }

    #[test]
    fn context_var_map_moved_app_ctx() {
        let _app = App::minimal();

        let mapped = TEST_VAR.map(|t| formatx!("map {t}"));
        let (_, a) = TEST_VAR.with_context(ContextInitHandle::new(), "A", || mapped.get());

        let (_, b) = TEST_VAR.with_context(ContextInitHandle::new(), "B", || mapped.get());

        assert_ne!(a, b);
    }

    #[test]
    fn context_var_cloned_same_widget() {
        let app = App::default();

        let mapped = TEST_VAR.map(|t| formatx!("map {t}"));
        use self::probe as probe_a;
        use self::probe as probe_b;
        use self::test_prop as test_prop_a;
        use self::test_prop as test_prop_b;

        let mut test = test_app(
            app,
            test_wgt! {
                test_prop_a = "A!";
                probe_a = mapped.clone();
                test_prop_b = "B!";
                probe_b = mapped;
            },
        );

        assert_eq!(test.ctx().app_state.get(&PROBE_ID), Some(&Text::from("map B!")));
    }

    #[test]
    fn context_var_set() {
        let mut app = test_app(App::default(), NilUiNode);

        let backing_var = var(Text::from(""));

        TEST_VAR.with_context(ContextInitHandle::new(), backing_var.clone(), || {
            let t = TEST_VAR;
            assert!(t.capabilities().contains(VarCapabilities::MODIFY));
            t.set("set!").unwrap();
        });

        let _ = app.update(false);
        assert_eq!(backing_var.get(), "set!");
    }

    #[test]
    fn context_var_binding() {
        let app = App::default();

        let input_var = var("Input!".to_text());
        let other_var = var(".".to_text());

        let mut test = test_app(
            app,
            test_wgt! {
                test_prop = input_var.clone();
                on_init = hn_once!(other_var, |_, _| {
                    TEST_VAR.bind(&other_var).perm();
                });
                child = NilUiNode;
            },
        );

        test.update(false).assert_wait();

        assert_eq!(".", other_var.get());

        input_var.set("Update!");

        test.update(false).assert_wait();

        assert_eq!("Update!", input_var.get());
        assert_eq!("Update!", other_var.get());
    }

    #[test]
    fn context_var_recursion_when1() {
        let _scope = App::minimal();

        let var = when_var! {
            false => var("hello".to_text()),
            _ => TEST_VAR,
        };

        let (_, r) = TEST_VAR.with_context(ContextInitHandle::new(), var.clone(), || var.get());

        assert_eq!("", r);
    }

    #[test]
    fn context_var_recursion_when2() {
        let _scope = App::minimal();

        let var = when_var! {
            true => TEST_VAR,
            _ => var("hello".to_text()),
        };

        let (_, r) = TEST_VAR.with_context(ContextInitHandle::new(), var.clone(), || var.get());

        assert_eq!("", r);
    }

    #[test]
    fn context_var_recursion_issue_when_any() {
        let _scope = App::minimal();

        let mut var = AnyWhenVarBuilder::new(TEST_VAR);
        var.push(self::var(false), self::var("hello".to_text()));
        let var = var.contextualized_build().unwrap();

        let (_, r) = TEST_VAR.with_context(ContextInitHandle::new(), var.clone(), || var.get());

        assert_eq!("", r);
    }

    #[test]
    fn context_var_recursion_merge() {
        let _scope = App::minimal();

        let var = merge_var!(TEST_VAR, var(true), |t, _| t.clone());

        let (_, r) = TEST_VAR.with_context(ContextInitHandle::new(), var.clone(), || var.get());

        assert_eq!("", r);
    }
}

mod flat_map {
    use crate::{context::TestWidgetContext, var::*};
    use std::fmt;

    #[derive(Clone)]
    pub struct Foo {
        pub bar: bool,
        pub var: ArcVar<usize>,
    }
    impl fmt::Debug for Foo {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Foo").field("bar", &self.bar).finish_non_exhaustive()
        }
    }

    #[test]
    pub fn flat_map() {
        let source = var(Foo { bar: true, var: var(32) });

        let test = source.flat_map(|f| f.var.clone());

        let mut ctx = TestWidgetContext::new();

        assert_eq!(32, test.get());

        source.get().var.set(42usize);

        let (_, ctx_updates) = ctx.apply_updates();

        assert!(ctx_updates.update);
        assert!(test.is_new());
        assert_eq!(42, test.get());

        let (_, ctx_updates) = ctx.apply_updates();
        assert!(!ctx_updates.update);

        let old_var = source.get().var;
        source.set(Foo { bar: false, var: var(192) });
        let (_, ctx_updates) = ctx.apply_updates();

        assert!(ctx_updates.update);
        assert!(test.is_new());
        assert_eq!(192, test.get());

        let (_, ctx_updates) = ctx.apply_updates();
        assert!(!ctx_updates.update);

        old_var.set(220usize);
        let (_, ctx_updates) = ctx.apply_updates();
        assert!(ctx_updates.update);
        assert!(!test.is_new());
        assert_eq!(192, test.get());
    }
}
