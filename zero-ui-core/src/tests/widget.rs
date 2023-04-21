//! Tests for `#[widget(..)]`  macro.
//!
//! Note: Compile error tests are in the integration tests folder: `tests/build/widget` and `tests/build/widget_new`

use self::util::Position;
use crate::{
    app::App,
    context::{WIDGET, WINDOW},
    var::{IntoValue, Var},
    widget,
    widget_builder::WidgetBuilder,
    widget_instance::{UiNode, WidgetId},
    widget_set,
};

// Used in multiple tests.
#[widget($crate::tests::widget::EmptyWgt)]
pub struct EmptyWgt(crate::widget_base::WidgetBase);

/*
 * Tests the implicitly inherited properties.
 */
#[test]
pub fn implicit_inherited() {
    let _app = App::minimal().run_headless(false);
    let expected = WidgetId::new_unique();
    let wgt = EmptyWgt! {
        id = expected;
    };
    let actual = wgt.with_context(|| WIDGET.id()).expect("expected widget");
    assert_eq!(expected, actual);
}

/*
 * Tests the inherited properties' default values and assigns.
 */
#[widget($crate::tests::widget::BarWgt)]
pub struct BarWgt(crate::widget_base::WidgetBase);
impl BarWgt {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            bar_trace = "bar_wgt";
            foo_trace = "foo_wgt";
        }
    }
}
#[crate::property(CONTEXT)]
pub fn foo_trace(child: impl UiNode, trace: impl crate::var::IntoValue<&'static str>) -> impl UiNode {
    util::trace(child, trace)
}

#[crate::property(CONTEXT, widget_impl(BarWgt))]
pub fn bar_trace(child: impl UiNode, trace: impl crate::var::IntoValue<&'static str>) -> impl UiNode {
    util::trace(child, trace)
}

#[test]
pub fn wgt_default_values() {
    let _app = App::minimal().run_headless(false);

    let mut default = BarWgt!();

    WINDOW.with_test_context(|| {
        default.init();
    });

    // test default values used.
    assert!(util::traced(&default, "foo_wgt"));
    assert!(util::traced(&default, "bar_wgt"));
}
#[test]
pub fn wgt_assign_values() {
    let _app = App::minimal().run_headless(false);

    let foo_trace = "foo!";
    let mut default = BarWgt! {
        foo_trace; // shorthand assign test.
        bar_trace = "bar!";
    };

    WINDOW.with_test_context(|| {
        default.init();
    });

    // test new values used.
    assert!(util::traced(&default, "foo!"));
    assert!(util::traced(&default, "bar!"));

    // test default values not used.
    assert!(!util::traced(&default, "foo_wgt"));
    assert!(!util::traced(&default, "bar_wgt"));
}

/*
 * Tests changing the default value of the inherited property.
 */
#[widget($crate::tests::widget::ResetWgt)]
pub struct ResetWgt(BarWgt);
impl ResetWgt {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            bar_trace = "reset_bar_wgt";
            foo_trace = "reset_wgt";
        }
    }
}

#[test]
pub fn wgt_with_new_value_for_inherited() {
    let _app = App::minimal().run_headless(false);

    let mut default = ResetWgt!();
    WINDOW.with_test_context(|| {
        default.init();
    });

    assert!(util::traced(&default, "reset_wgt"));
    assert!(util::traced(&default, "reset_bar_wgt"));
    assert!(!util::traced(&default, "bar_wgt"));
}

/*
 * Test unsetting default value.
 */
#[widget($crate::tests::widget::DefaultValueWgt)]
pub struct DefaultValueWgt(crate::widget_base::WidgetBase);
impl DefaultValueWgt {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            util::trace = "default_value_wgt";
        }
    }
}
#[test]
pub fn unset_default_value() {
    let _app = App::minimal().run_headless(false);

    WINDOW.with_test_context(|| {
        let mut default = DefaultValueWgt!();
        default.init();

        assert!(util::traced(&default, "default_value_wgt"));

        let mut no_default = DefaultValueWgt! {
            util::trace = unset!;
        };
        no_default.init();

        assert!(!util::traced(&no_default, "default_value_wgt"));
    });
}

/*
 * Tests value initialization order.
 */
#[test]
pub fn value_init_order() {
    let _app = App::minimal().run_headless(false);

    WINDOW.with_test_context(|| {
        Position::reset();
        let mut wgt = EmptyWgt! {
            util::count_border = Position::next("count_border");
            util::count_context = Position::next("count_context");
        };
        wgt.init();

        // values evaluated in typed order.
        assert_eq!(util::sorted_value_init(&wgt), ["count_border", "count_context"]);

        // but properties init in the nest group order.
        assert_eq!(util::sorted_node_init(&wgt), ["count_context", "count_border"]);
    });
}

#[test]
pub fn wgt_child_property_init_order() {
    let _app = App::minimal().run_headless(false);

    WINDOW.with_test_context(|| {
        Position::reset();
        let mut wgt = EmptyWgt! {
            util::count_border = Position::next("count_border");
            util::count_child_layout = Position::next("count_child_layout");
            util::count_context = Position::next("count_context");
        };
        wgt.init();

        // values evaluated in typed order.
        assert_eq!(
            util::sorted_value_init(&wgt),
            ["count_border", "count_child_layout", "count_context"]
        );

        // but properties init in the nest group order (child first).
        assert_eq!(
            util::sorted_node_init(&wgt),
            ["count_context", "count_border", "count_child_layout"]
        );
    });
}

/*
 * Tests the ordering of properties of the same nest group.
 */
#[widget($crate::tests::widget::SameNestGroupOrderWgt)]
pub struct SameNestGroupOrderWgt(crate::widget_base::WidgetBase);

#[crate::property(BORDER)]
pub fn border_a(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
    util::count_border(child, count)
}

#[crate::property(BORDER)]
pub fn border_b(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
    util::count_border(child, count)
}

#[test]
pub fn wgt_same_nest_group_order() {
    let _app = App::minimal().run_headless(false);

    WINDOW.with_test_context(|| {
        Position::reset();
        let mut wgt = SameNestGroupOrderWgt! {
            border_a = Position::next("border_a");
            border_b = Position::next("border_b");
        };
        wgt.init();

        // values evaluated in typed order.
        assert_eq!(util::sorted_value_init(&wgt), ["border_a", "border_b"]);

        // properties with the same nest group are set in reversed typed order.
        // inner_a is set after inner_b so it will contain inner_b:
        // let node = border_b(child, ..);
        // let node = border_a(node, ..);
        assert_eq!(util::sorted_node_init(&wgt), ["border_a", "border_b"]);

        Position::reset();
        // order of declaration(in the widget) doesn't impact the order of evaluation,
        // only the order of use does (in here).
        let mut wgt = SameNestGroupOrderWgt! {
            border_b = Position::next("border_b");
            border_a = Position::next("border_a");
        };
        wgt.init();

        assert_eq!(util::sorted_value_init(&wgt), ["border_b", "border_a"]);
        assert_eq!(util::sorted_node_init(&wgt), ["border_b", "border_a"]);
    });
}

/*
 *  Tests widget when.
 */
#[widget($crate::tests::widget::WhenWgt)]
pub struct WhenWgt(crate::widget_base::WidgetBase);
impl WhenWgt {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            util::live_trace = "boo!";

            when *#util::is_state {
                util::live_trace = "ok.";
            }
        }
    }
}
#[test]
pub fn wgt_when() {
    let _app = App::minimal().run_headless(false);

    WINDOW.with_test_context(|| {
        let mut wgt = WhenWgt!();
        WINDOW.test_init(&mut wgt);

        assert!(util::traced(&wgt, "boo!"));

        util::set_state(&mut wgt, true);
        WINDOW.test_update(&mut wgt, None); // state
        WINDOW.test_update(&mut wgt, None); // when
        assert!(util::traced(&wgt, "ok."));

        util::set_state(&mut wgt, false);
        WINDOW.test_update(&mut wgt, None); // state
        WINDOW.test_update(&mut wgt, None); // when

        assert!(util::traced(&wgt, "boo!"));
    });
}
#[test]
pub fn widget_user_when() {
    let _app = App::minimal().run_headless(false);

    WINDOW.with_test_context(|| {
        let mut wgt = EmptyWgt! {
            util::live_trace = "A";

            when *#util::is_state {
                util::live_trace = "B";
            }
        };
        wgt.init();

        assert!(util::traced(&wgt, "A"));

        util::set_state(&mut wgt, true);
        WINDOW.test_update(&mut wgt, None); // state
        WINDOW.test_update(&mut wgt, None); // when

        assert!(util::traced(&wgt, "B"));

        util::set_state(&mut wgt, false); // state
        WINDOW.test_update(&mut wgt, None); // when

        assert!(util::traced(&wgt, "A"));
    });
}

/*
 * Tests multiple widget whens
 */
#[widget($crate::tests::widget::MultiWhenWgt)]
pub struct MultiWhenWgt(crate::widget_base::WidgetBase);

impl MultiWhenWgt {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            util::live_trace = "default";
            when *#util::is_state {
                util::live_trace = "state_0";
            }
            when *#util::is_state {
                util::live_trace = "state_1";
            }
        }
    }
}
#[test]
pub fn wgt_multi_when() {
    let _app = App::minimal().run_headless(false);

    WINDOW.with_test_context(|| {
        let mut wgt = MultiWhenWgt!();
        wgt.init();

        assert!(util::traced(&wgt, "default"));

        util::set_state(&mut wgt, true);
        WINDOW.test_update(&mut wgt, None); // state
        WINDOW.test_update(&mut wgt, None); // when

        assert!(util::traced(&wgt, "state_1"));

        util::set_state(&mut wgt, false);
        WINDOW.test_update(&mut wgt, None);

        assert!(util::traced(&wgt, "default"));
    });
}

/*
 * Tests widget property attributes.
 */
#[widget($crate::tests::widget::CfgPropertyWgt)]
pub struct CfgPropertyWgt(crate::widget_base::WidgetBase);
impl CfgPropertyWgt {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            // property not included in widget.
            #[cfg(never)]
            never_trace = "never-trace";

            // suppress warning.
            #[allow(non_snake_case)]
            always_trace = {
                #[allow(clippy::needless_late_init)]
                let weird___name;
                weird___name = "always-trace";
                weird___name
            };
        }
    }
}
#[cfg(never)]
#[crate::property(CONTEXT)]
pub fn never_trace(child: impl UiNode, trace: impl IntoValue<&'static str>) -> impl UiNode {
    util::trace(child, trace)
}
#[crate::property(CONTEXT)]
pub fn always_trace(child: impl UiNode, trace: impl IntoValue<&'static str>) -> impl UiNode {
    util::trace(child, trace)
}

#[test]
pub fn wgt_cfg_property() {
    let _app = App::minimal().run_headless(false);

    WINDOW.with_test_context(|| {
        let mut wgt = CfgPropertyWgt!();
        wgt.init();

        assert!(util::traced(&wgt, "always-trace"));
        assert!(!util::traced(&wgt, "never-trace"));
    });
}
#[test]
pub fn user_cfg_property() {
    let _app = App::minimal().run_headless(false);

    WINDOW.with_test_context(|| {
        let mut wgt = EmptyWgt! {
            // property not set.
            #[cfg(never)]
            never_trace = "never-trace";

            // suppress warning.
            #[allow(non_snake_case)]
            always_trace = {
                #[allow(clippy::needless_late_init)]
                let weird___name;
                weird___name = "always-trace";
                weird___name
            };
        };

        wgt.init();

        assert!(util::traced(&wgt, "always-trace"));
        assert!(!util::traced(&wgt, "never-trace"));
    });
}

/*
 * Tests widget when attributes.
 */
#[widget($crate::tests::widget::CfgWhenWgt)]
pub struct CfgWhenWgt(crate::widget_base::WidgetBase);
impl CfgWhenWgt {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            util::live_trace = "trace";

            // suppress warning in all assigns.
            #[allow(non_snake_case)]
            when *#util::is_state {
                util::live_trace = {
                    #[allow(clippy::needless_late_init)]
                    let weird___name;
                    weird___name = "is_state";
                    weird___name
                };
            }

            // when not applied.
            #[cfg(never)]
            when *#util::is_state {
                util::live_trace = "is_never_state";
            }
        }
    }
}
#[test]
pub fn wgt_cfg_when() {
    let _app = App::minimal().run_headless(false);

    WINDOW.with_test_context(|| {
        let mut wgt = CfgWhenWgt!();

        wgt.init();

        assert!(util::traced(&wgt, "trace"));

        util::set_state(&mut wgt, true);
        WINDOW.test_update(&mut wgt, None); // state
        WINDOW.test_update(&mut wgt, None); // when

        assert!(util::traced(&wgt, "is_state"));

        util::set_state(&mut wgt, false);
        WINDOW.test_update(&mut wgt, None);

        assert!(util::traced(&wgt, "trace"));
    });
}

#[test]
pub fn user_cfg_when() {
    let _app = App::minimal().run_headless(false);

    WINDOW.with_test_context(|| {
        let mut wgt = EmptyWgt! {
            util::live_trace = "trace";

            when *#util::is_state {
                util::live_trace = {
                    #[allow(non_snake_case)]
                    #[allow(clippy::needless_late_init)]
                    let weird___name;
                    weird___name = "is_state";
                    weird___name
                };
            }

            #[cfg(never)]
            when *#util::is_state {
                util::live_trace = "is_never_state";
            }
        };

        wgt.init();

        assert!(util::traced(&wgt, "trace"));

        util::set_state(&mut wgt, true);
        WINDOW.test_update(&mut wgt, None); // state
        WINDOW.test_update(&mut wgt, None); // when

        assert!(util::traced(&wgt, "is_state"));

        util::set_state(&mut wgt, false);
        WINDOW.test_update(&mut wgt, None);

        assert!(util::traced(&wgt, "trace"));
    });
}

/*
 * Tests order properties are inited and applied.
 */

#[widget($crate::tests::widget::PropertyNestGroupSortingWgt)]
pub struct PropertyNestGroupSortingWgt(crate::widget_base::WidgetBase);
impl PropertyNestGroupSortingWgt {}
fn property_nest_group_sorting_init1() -> impl UiNode {
    PropertyNestGroupSortingWgt! {
        util::count_border = Position::next("count_border");
        util::count_border2 = Position::next("count_border2");
        util::count_size = Position::next("count_size");
        util::count_size2 = Position::next("count_size2");
        util::count_layout = Position::next("count_layout");
        util::count_layout2 = Position::next("count_layout2");
        util::count_event = Position::next("count_event");
        util::count_event2 = Position::next("count_event2");
        util::count_context = Position::next("count_context");
        util::count_context2 = Position::next("count_context2");

        util::count_child_layout = Position::next("count_child_layout");
        util::count_child_layout2 = Position::next("count_child_layout2");
        util::count_child_context = Position::next("count_child_context");
        util::count_child_context2 = Position::next("count_child_context2");
    }
}
#[test]
pub fn property_nest_group_sorting_value_init1() {
    let _app = App::minimal().run_headless(false);

    WINDOW.with_test_context(|| {
        Position::reset();

        let mut wgt = property_nest_group_sorting_init1();
        wgt.init();

        // assert that value init is the same as typed.
        pretty_assertions::assert_eq!(
            util::sorted_value_init(&wgt),
            [
                "count_border",
                "count_border2",
                "count_size",
                "count_size2",
                "count_layout",
                "count_layout2",
                "count_event",
                "count_event2",
                "count_context",
                "count_context2",
                "count_child_layout",
                "count_child_layout2",
                "count_child_context",
                "count_child_context2",
            ]
        );
    });
}
fn property_nest_group_sorting_init2() -> impl UiNode {
    PropertyNestGroupSortingWgt! {
        util::count_child_context = Position::next("count_child_context");
        util::count_child_context2 = Position::next("count_child_context2");
        util::count_child_layout = Position::next("count_child_layout");
        util::count_child_layout2 = Position::next("count_child_layout2");

        util::count_context = Position::next("count_context");
        util::count_context2 = Position::next("count_context2");
        util::count_event = Position::next("count_event");
        util::count_event2 = Position::next("count_event2");
        util::count_layout = Position::next("count_layout");
        util::count_layout2 = Position::next("count_layout2");
        util::count_size = Position::next("count_size");
        util::count_size2 = Position::next("count_size2");
        util::count_border = Position::next("count_border");
        util::count_border2 = Position::next("count_border2");
    }
}
#[test]
pub fn property_nest_group_sorting_value_init2() {
    let _app = App::minimal().run_headless(false);

    WINDOW.with_test_context(|| {
        Position::reset();

        let mut wgt = property_nest_group_sorting_init2();
        wgt.init();

        // assert that value init is the same as typed.
        pretty_assertions::assert_eq!(
            util::sorted_value_init(&wgt),
            [
                "count_child_context",
                "count_child_context2",
                "count_child_layout",
                "count_child_layout2",
                "count_context",
                "count_context2",
                "count_event",
                "count_event2",
                "count_layout",
                "count_layout2",
                "count_size",
                "count_size2",
                "count_border",
                "count_border2",
            ]
        );
    });
}
fn assert_node_order(wgt: &impl UiNode) {
    // assert that `UiNode::init` position is sorted by `child` and
    // property priorities, followed by the typed position.
    pretty_assertions::assert_eq!(
        util::sorted_node_init(wgt),
        [
            // each property wraps the next one and takes a position number before
            // delegating to the next property (child node).
            "count_context",
            "count_context2",
            "count_event",
            "count_event2",
            "count_layout",
            "count_layout2",
            "count_size",
            "count_size2",
            "count_border",
            "count_border2",
            "count_child_context",
            "count_child_context2",
            "count_child_layout",
            "count_child_layout2",
        ]
    );
}
#[test]
pub fn property_nest_group_sorting_node_init1() {
    let _app = App::minimal().run_headless(false);

    WINDOW.with_test_context(|| {
        Position::reset();

        let mut wgt = property_nest_group_sorting_init1();
        wgt.init();

        assert_node_order(&wgt);
    });
}
#[test]
pub fn property_nest_group_sorting_node_init2() {
    let _app = App::minimal().run_headless(false);
    WINDOW.with_test_context(|| {
        Position::reset();

        let mut wgt = property_nest_group_sorting_init2();
        wgt.init();

        assert_node_order(&wgt);
    });
}
#[widget($crate::tests::widget::PropertyNestGroupSortingInheritedWgt)]
pub struct PropertyNestGroupSortingInheritedWgt(PropertyNestGroupSortingWgt);

#[test]
pub fn property_nest_group_sorting_node_inherited_init() {
    let _app = App::minimal().run_headless(false);
    WINDOW.with_test_context(|| {
        Position::reset();

        let mut wgt = PropertyNestGroupSortingInheritedWgt! {
            util::count_child_context = Position::next("count_child_context");
            util::count_child_context2 = Position::next("count_child_context2");
            util::count_child_layout = Position::next("count_child_layout");
            util::count_child_layout2 = Position::next("count_child_layout2");

            util::count_context = Position::next("count_context");
            util::count_context2 = Position::next("count_context2");
            util::count_event = Position::next("count_event");
            util::count_event2 = Position::next("count_event2");
            util::count_layout = Position::next("count_layout");
            util::count_layout2 = Position::next("count_layout2");
            util::count_size = Position::next("count_size");
            util::count_size2 = Position::next("count_size2");
            util::count_border = Position::next("count_border");
            util::count_border2 = Position::next("count_border2");
        };
        wgt.init();

        assert_node_order(&wgt);
    });
}

#[widget($crate::tests::widget::PropertyNestGroupSortingDefaultsWgt)]
pub struct PropertyNestGroupSortingDefaultsWgt(PropertyNestGroupSortingWgt);
impl PropertyNestGroupSortingDefaultsWgt {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            util::count_context = Position::next("count_context");
            util::count_context2 = Position::next("count_context2");
            util::count_event = Position::next("count_event");
            util::count_event2 = Position::next("count_event2");
            util::count_layout = Position::next("count_layout");
            util::count_layout2 = Position::next("count_layout2");
            util::count_size = Position::next("count_size");
            util::count_size2 = Position::next("count_size2");
            util::count_border = Position::next("count_border");
            util::count_border2 = Position::next("count_border2");

            util::count_child_context = Position::next("count_child_context");
            util::count_child_context2 = Position::next("count_child_context2");
            util::count_child_layout = Position::next("count_child_layout");
            util::count_child_layout2 = Position::next("count_child_layout2");
        }
    }
}
#[test]
pub fn property_nest_group_sorting_defaults() {
    let _app = App::minimal().run_headless(false);
    WINDOW.with_test_context(|| {
        Position::reset();

        let mut wgt = PropertyNestGroupSortingDefaultsWgt!();
        wgt.init();
        assert_node_order(&wgt);
    });
}

/*
 * Tests property member access in when
 */

#[test]
pub fn when_property_member_default() {
    let _app = App::minimal().run_headless(false);
    WINDOW.with_test_context(|| {
        let mut wgt = EmptyWgt! {
           util::duo_members = "a", "b";
           util::live_trace = "";
           when {
               assert_eq!(*#util::duo_members, "a");
               true
           } {
               util::live_trace = "true";
           }
        };
        wgt.init();

        assert!(util::traced(&wgt, "true"));
    });
}

#[test]
pub fn when_property_member_index() {
    let _app = App::minimal().run_headless(false);
    WINDOW.with_test_context(|| {
        let mut wgt = EmptyWgt! {
           util::duo_members = "a", "b";
           util::live_trace = "";
           when {
               assert_eq!(*#util::duo_members.0, "a");
               assert_eq!(*#util::duo_members.1, "b");
               true
           } {
               util::live_trace = "true";
           }
        };

        wgt.init();
        assert!(util::traced(&wgt, "true"));
    });
}

#[test]
pub fn when_property_member_named() {
    let _app = App::minimal().run_headless(false);
    WINDOW.with_test_context(|| {
        let mut wgt = EmptyWgt! {
           util::duo_members = "a", "b";
           util::live_trace = "";
           when {
               assert_eq!(*#util::duo_members.member_a, "a");
               assert_eq!(*#util::duo_members.member_b, "b");
               true
           } {
               util::live_trace = "true";
           }
        };

        wgt.init();
        assert!(util::traced(&wgt, "true"));
    });
}

#[test]
pub fn when_property_member_default_method() {
    let _app = App::minimal().run_headless(false);
    WINDOW.with_test_context(|| {
        let mut wgt = EmptyWgt! {
           util::duo_members = "a", "b";
           util::live_trace = "";
           when {
               assert_eq!(#util::duo_members.len(), 1);
               true
           } {
               util::live_trace = "true";
           }
        };
        wgt.init();
        assert!(util::traced(&wgt, "true"));
    });
}

#[test]
pub fn when_property_member_indexed_method() {
    let _app = App::minimal().run_headless(false);
    WINDOW.with_test_context(|| {
        let mut wgt = EmptyWgt! {
           util::duo_members = "a", "b";
           util::live_trace = "";
           when {
               assert_eq!(#util::duo_members.0.len(), 1);
               true
           } {
               util::live_trace = "true";
           }
        };
        wgt.init();

        assert!(util::traced(&wgt, "true"));
    });
}

#[widget($crate::tests::widget::GetBuilder)]
pub struct GetBuilder(crate::widget_base::WidgetBase);
impl GetBuilder {
    pub fn widget_build(&mut self) -> WidgetBuilder {
        let mut wgt = self.widget_take();
        wgt.set_custom_build(crate::widget_base::nodes::build);
        wgt
    }
}

#[test]
pub fn when_reuse() {
    let test = |pass: &str| {
        let _app = App::minimal().run_headless(false);
        WINDOW.with_test_context(|| {
            let builder = GetBuilder! {
                util::live_trace = "false";

                when *#util::is_state {
                    util::live_trace = "true";
                }
            };
            let mut wgt = builder.build();

            wgt.init();
            assert!(!util::traced(&wgt, "true"), "traced `true` in {pass} pass");
            assert!(util::traced(&wgt, "false"), "did not trace `false` in {pass} pass");

            util::set_state(&mut wgt, true);
            WINDOW.test_update(&mut wgt, None); // state
            WINDOW.test_update(&mut wgt, None); // when
            assert!(util::traced(&wgt, "true"), "did not trace `true` after when in {pass} pass");

            util::set_state(&mut wgt, false);
            WINDOW.test_update(&mut wgt, None);
        });
    };

    test("first");
    test("reuse");
}

/*
* Property Default Value
*/

#[test]
pub fn allowed_in_when_without_wgt_assign1() {
    let _app = App::minimal().run_headless(false);
    WINDOW.with_test_context(|| {
        let mut wgt = EmptyWgt! {
            // util::live_trace_default = "default-trace";
            when *#util::is_state {
                util::live_trace_default = "when-trace";
            }
        };

        wgt.init();
        assert!(util::traced(&wgt, "default-trace"));
        assert!(!util::traced(&wgt, "when-trace"));

        util::set_state(&mut wgt, true);
        WINDOW.test_update(&mut wgt, None); // state update
        WINDOW.test_update(&mut wgt, None); // when update

        assert!(util::traced(&wgt, "when-trace"));
    });
}

#[test]
pub fn allowed_in_when_without_wgt_assign2() {
    let _app = App::minimal().run_headless(false);
    WINDOW.with_test_context(|| {
        let mut wgt = EmptyWgt! {
            // util::live_trace_default = "default-trace";
            when *#util::is_state {
                util::live_trace_default = "when-trace";
            }
        };

        wgt.init();
        assert!(util::traced(&wgt, "default-trace"));
        assert!(!util::traced(&wgt, "when-trace"));

        util::set_state(&mut wgt, true);
        WINDOW.test_update(&mut wgt, None); // state update
        WINDOW.test_update(&mut wgt, None); // when update
        assert!(util::traced(&wgt, "when-trace"));
    });
}

/*
* Generated Names Don't Shadow Each Other
*/
#[crate::property(CONTEXT)]
pub fn util_live_trace(
    child: impl crate::widget_instance::UiNode,
    not_str: impl crate::var::IntoVar<bool>,
) -> impl crate::widget_instance::UiNode {
    let var = not_str.into_var().map(|&b| if b { "true" } else { "false" });
    util::live_trace(child, var)
}

#[test]
pub fn generated_name_collision() {
    let _app = App::minimal().run_headless(false);
    WINDOW.with_test_context(|| {
        let mut wgt = EmptyWgt! {
            util::live_trace = "!";
            util_live_trace = false;
        };

        wgt.init();

        assert!(util::traced(&wgt, "!"));
        assert!(util::traced(&wgt, "false"));
    });
}

#[test]
pub fn generated_name_collision_in_when() {
    let _app = App::minimal().run_headless(false);
    WINDOW.with_test_context(|| {
        let mut wgt = EmptyWgt! {
            util::live_trace = "1";
            when *#util::is_state {
                util::live_trace = "2";
            }
            when *#util::is_state {
                util::live_trace = "3";
            }
        };

        wgt.init();
        util::set_state(&mut wgt, true);
        WINDOW.test_update(&mut wgt, None); // state
        WINDOW.test_update(&mut wgt, None); // when

        assert!(util::traced(&wgt, "3"));
        assert!(!util::traced(&wgt, "2"));
    });
}

#[test]
pub fn generated_name_collision_in_when_assign() {
    let _app = App::minimal().run_headless(false);
    WINDOW.with_test_context(|| {
        let mut wgt = EmptyWgt! {
            util::live_trace = "0";
            util_live_trace = false;

            when *#util::is_state {
                util::live_trace = "1";
                util_live_trace = true;
            }
        };

        wgt.init();
        util::set_state(&mut wgt, true);
        WINDOW.test_update(&mut wgt, None); // state update
        WINDOW.test_update(&mut wgt, None); // when update

        assert!(util::traced(&wgt, "1"));
        assert!(util::traced(&wgt, "true"));
    });
}

#[widget($crate::tests::widget::NameCollisionWgtWhen)]
pub struct NameCollisionWgtWhen(crate::widget_base::WidgetBase);
impl NameCollisionWgtWhen {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            util::live_trace = "1";

            when *#util::is_state {
                util::live_trace = "2";
            }
            when *#util::is_state {
                util::live_trace = "3";
            }
        }
    }
}
#[test]
pub fn name_collision_wgt_when() {
    let _app = App::minimal().run_headless(false);
    WINDOW.with_test_context(|| {
        let mut wgt = NameCollisionWgtWhen!();

        wgt.init();
        util::set_state(&mut wgt, true);
        WINDOW.test_update(&mut wgt, None); // state update
        WINDOW.test_update(&mut wgt, None); // when update

        assert!(util::traced(&wgt, "3"));
        assert!(!util::traced(&wgt, "2"));
    });
}

/*
* macro_rules! generated widget
*/

mod macro_rules_generated {
    use crate::widget;

    macro_rules! test {
        ($name:ident) => {
           test! {
               [$] $name
           }
        };
        ([$dollar:tt] $name:ident) => {
            #[widget($dollar crate::tests::widget::macro_rules_generated::$name)]
            pub struct $name($crate::widget_base::WidgetBase);

            #[$crate::property(CONTEXT, widget_impl($name))]
            pub fn margin(
                child: impl $crate::widget_instance::UiNode,
                margin: impl $crate::var::IntoVar<$crate::units::SideOffsets>
            ) -> impl $crate::widget_instance::UiNode {
                let _ = margin;
                child
            }
        }
    }

    test! {
        Bar
    }
}

#[test]
fn macro_rules_generated() {
    let _ = macro_rules_generated::Bar! {
        margin = 10;
    };
}

pub mod util {
    use std::{
        cell::Cell,
        collections::{HashMap, HashSet},
    };

    use crate::{
        context::{StaticStateId, WIDGET},
        property,
        var::{IntoValue, IntoVar, Var},
        widget_instance::{match_node, UiNode, UiNodeOp},
    };

    /// Insert `trace` in the widget state. Can be probed using [`traced`].
    #[property(CONTEXT)]
    pub fn trace(child: impl UiNode, trace: impl IntoValue<&'static str>) -> impl UiNode {
        let trace = trace.into();
        match_node(child, move |child, op| {
            if let UiNodeOp::Init = op {
                child.init();
                WIDGET.with_state_mut(|mut s| {
                    s.entry(&TRACE_ID).or_default().insert(trace);
                });
            }
        })
    }

    /// Probe for a [`trace`] in the widget state.
    pub fn traced(wgt: &impl UiNode, trace: &'static str) -> bool {
        wgt.with_context(|| WIDGET.with_state(|s| s.get(&TRACE_ID).map(|t| t.contains(trace)).unwrap_or_default()))
            .expect("expected widget")
    }

    static TRACE_ID: StaticStateId<HashSet<&'static str>> = StaticStateId::new_unique();

    /// Insert `count` in the widget state. Can get using [`Count::get`].
    #[property(CONTEXT)]
    pub fn count(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        count_node(child, count)
    }

    /// Same as [`count`] but in `CHILD_CONTEXT` group.
    #[property(CHILD_CONTEXT)]
    pub fn count_child_context(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        count_node(child, count)
    }
    /// Same as [`count`] but in `CHILD_CONTEXT` group.
    #[property(CHILD_CONTEXT)]
    pub fn count_child_context2(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        count_node(child, count)
    }

    /// Same as [`count`] but in `CHILD_LAYOUT` group.
    #[property(CHILD_LAYOUT)]
    pub fn count_child_layout(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        count_node(child, count)
    }
    /// Same as [`count`] but in `CHILD_LAYOUT` group.
    #[property(CHILD_LAYOUT)]
    pub fn count_child_layout2(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        count_node(child, count)
    }

    /// Same as [`count`] but in `BORDER` group.
    #[property(BORDER)]
    pub fn count_border(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        count_node(child, count)
    }
    /// Same as [`count`] but in `BORDER` group.
    #[property(BORDER)]
    pub fn count_border2(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        count_node(child, count)
    }

    /// Same as [`count`] but in `LAYOUT` group.
    #[property(LAYOUT)]
    pub fn count_layout(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        count_node(child, count)
    }
    /// Same as [`count`] but in `LAYOUT` group.
    #[property(LAYOUT)]
    pub fn count_layout2(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        count_node(child, count)
    }

    /// Same as [`count`] but in `CONTEXT` group.
    #[property(CONTEXT)]
    pub fn count_context(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        count_node(child, count)
    }
    /// Same as [`count`] but in `CONTEXT` group.
    #[property(CONTEXT)]
    pub fn count_context2(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        count_node(child, count)
    }

    /// Same as [`count`] but in `SIZE` group.
    #[property(SIZE)]
    pub fn count_size(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        count_node(child, count)
    }
    /// Same as [`count`] but in `SIZE` group.
    #[property(SIZE)]
    pub fn count_size2(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        count_node(child, count)
    }

    /// Same as [`count`] but in `EVENT` group.
    #[property(EVENT)]
    pub fn count_event(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        count_node(child, count)
    }
    /// Same as [`count`] but in `EVENT` group.
    #[property(EVENT)]
    pub fn count_event2(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        count_node(child, count)
    }

    /// Count adds one every [`Self::next`] call.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct Position {
        pub pos: u32,
        pub tag: &'static str,
    }
    thread_local! {
        static COUNT: Cell<u32> = const { Cell::new(0) };
        static COUNT_INIT: Cell<u32> = const { Cell::new(0) };
    }
    impl Position {
        pub fn next(tag: &'static str) -> Self {
            Position {
                pos: COUNT.with(|c| {
                    let r = c.get();
                    c.set(r + 1);
                    r
                }),
                tag,
            }
        }

        fn next_init() -> u32 {
            COUNT_INIT.with(|c| {
                let r = c.get();
                c.set(r + 1);
                r
            })
        }

        pub fn reset() {
            COUNT.with(|c| c.set(0));
            COUNT_INIT.with(|c| c.set(0));
        }
    }

    /// Gets the [`Position`] tags sorted by call to [`Position::next`].
    pub fn sorted_value_init(wgt: &impl UiNode) -> Vec<&'static str> {
        let mut vec = vec![];
        wgt.with_context(|| {
            if let Some(m) = WIDGET.get_state(&VALUE_POSITION_ID) {
                for (key, value) in m {
                    vec.push((key, value));
                }
            }
        });
        vec.sort_by_key(|(_, i)| *i);
        vec.into_iter().map(|(t, _)| t).collect()
    }

    /// Gets the [`Position`] tags sorted by the [`UiNode::init` call.
    pub fn sorted_node_init(wgt: &impl UiNode) -> Vec<&'static str> {
        let mut vec = vec![];
        wgt.with_context(|| {
            if let Some(m) = WIDGET.get_state(&NODE_POSITION_ID) {
                for (key, value) in m {
                    vec.push((key, value));
                }
            }
        });
        vec.sort_by_key(|(_, i)| *i);
        vec.into_iter().map(|(t, _)| t).collect()
    }

    static VALUE_POSITION_ID: StaticStateId<HashMap<&'static str, u32>> = StaticStateId::new_unique();
    static NODE_POSITION_ID: StaticStateId<HashMap<&'static str, u32>> = StaticStateId::new_unique();

    fn count_node(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        let value_pos = count.into();
        match_node(child, move |_, op| {
            if let UiNodeOp::Init = op {
                WIDGET.with_state_mut(|mut s| {
                    s.entry(&VALUE_POSITION_ID).or_default().insert(value_pos.tag, value_pos.pos);

                    s.entry(&NODE_POSITION_ID).or_default().insert(value_pos.tag, Position::next_init());
                });
            }
        })
    }

    /// Test state property, state can be set using [`set_state`] followed by updating.
    #[property(CONTEXT)]
    pub fn is_state(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
        let state = state.into_var();
        match_node(child, move |child, op| {
            let update = match op {
                UiNodeOp::Init => {
                    child.init();
                    true
                }
                UiNodeOp::Update { updates } => {
                    child.update(updates);
                    true
                }
                _ => false,
            };
            if update {
                let wgt_state = WIDGET.get_state(&IS_STATE_ID).unwrap_or_default();
                if wgt_state != state.get() {
                    let _ = state.set(wgt_state);
                }
            }
        })
    }
    /// Sets the [`is_state`] of a widget.
    ///
    /// Note only applies after update.
    pub fn set_state(wgt: &mut impl UiNode, state: bool) {
        wgt.with_context(|| {
            WIDGET.with_state_mut(|mut s| {
                *s.entry(&IS_STATE_ID).or_default() = state;
            });
            WIDGET.update();
        })
        .expect("expected widget");
    }

    static IS_STATE_ID: StaticStateId<bool> = StaticStateId::new_unique();

    /// A [trace] that can update.
    #[property(CONTEXT)]
    pub fn live_trace(child: impl UiNode, trace: impl IntoVar<&'static str>) -> impl UiNode {
        let trace = trace.into_var();
        match_node(child, move |child, op| match op {
            UiNodeOp::Init => {
                child.init();
                WIDGET.sub_var(&trace);
                WIDGET.with_state_mut(|mut s| {
                    s.entry(&TRACE_ID).or_default().insert(trace.get());
                });
            }
            UiNodeOp::Update { updates } => {
                child.update(updates);
                if let Some(trace) = trace.get_new() {
                    WIDGET.with_state_mut(|mut s| {
                        s.entry(&TRACE_ID).or_default().insert(trace);
                    })
                }
            }
            _ => {}
        })
    }
    /// A [trace] that can update and has a default value of `"default-trace"`.
    #[property(CONTEXT, default("default-trace"))]
    pub fn live_trace_default(child: impl UiNode, trace: impl IntoVar<&'static str>) -> impl UiNode {
        live_trace(child, trace)
    }

    /// A capture_only property.
    #[property(CONTEXT)]
    #[allow(unreachable_code)]
    pub fn capture_only_trace(_child: impl UiNode, trace: impl IntoValue<&'static str>) -> impl UiNode {
        let _ = trace;
        panic!("capture-only property");
        _child
    }

    #[property(CONTEXT)]
    pub fn duo_members(child: impl UiNode, member_a: impl IntoVar<&'static str>, member_b: impl IntoVar<&'static str>) -> impl UiNode {
        let _ = member_a;
        let _ = member_b;
        child
    }
}
