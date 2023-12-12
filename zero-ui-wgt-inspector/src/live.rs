use zero_ui_core::{
    handler::clmv,
    window::{CursorIcon, WINDOW_Ext},
};

use crate::core::{
    color::colors,
    context::*,
    handler::async_clmv,
    hn,
    inspector::live::{InspectedTree, InspectedWidget},
    render::{SpatialFrameId, SpatialFrameKey},
    units::*,
    var::*,
    widget_instance::*,
    window::{WindowId, WINDOWS},
};

use super::*;

pub fn inspect_node(child: impl UiNode, can_inspect: impl IntoVar<bool>) -> impl UiNode {
    let mut inspected_tree = None::<InspectedTree>;
    let inspector = WindowId::new_unique();

    let selected_wgt = var(None);
    let hit_select = var(HitSelect::Disabled);
    let show_selected = var(true);

    let can_inspect = can_inspect.into_var();
    let mut cmd_handle = CommandHandle::dummy();

    enum InspectorUpdateOnly {
        Info,
        Render,
    }

    let child = match_node(
        child,
        clmv!(selected_wgt, hit_select, show_selected, |c, op| match op {
            UiNodeOp::Init => {
                WIDGET.sub_var(&can_inspect);
                cmd_handle = INSPECT_CMD.scoped(WINDOW.id()).subscribe_wgt(can_inspect.get(), WIDGET.id());
            }
            UiNodeOp::Update { .. } => {
                if let Some(e) = can_inspect.get_new() {
                    cmd_handle.set_enabled(e);
                }
            }
            UiNodeOp::Info { info } => {
                if inspected_tree.is_some() {
                    if WINDOWS.is_open(inspector) {
                        INSPECT_CMD.scoped(WINDOW.id()).notify_param(InspectorUpdateOnly::Info);
                    } else if !WINDOWS.is_opening(inspector) {
                        inspected_tree = None;
                    }
                }
            }
            UiNodeOp::Event { update } => {
                c.event(update);

                if let Some(args) = INSPECT_CMD.scoped(WINDOW.id()).on_unhandled(update) {
                    args.propagation().stop();

                    if let Some(u) = args.param::<InspectorUpdateOnly>() {
                        if let Some(i) = &inspected_tree {
                            match u {
                                InspectorUpdateOnly::Info => i.update(WINDOW.info()),
                                InspectorUpdateOnly::Render => i.update_render(),
                            }
                        }
                    } else if let Some(inspected) = inspector_window::inspected() {
                        // can't inspect inspector window, redirect command to inspected
                        INSPECT_CMD.scoped(inspected).notify();
                    } else {
                        let inspected_tree = match &inspected_tree {
                            Some(i) => {
                                i.update(WINDOW.info());
                                i.clone()
                            }
                            None => {
                                let i = InspectedTree::new(WINDOW.info());
                                inspected_tree = Some(i.clone());
                                i
                            }
                        };

                        let inspected = WINDOW.id();
                        WINDOWS.focus_or_open(
                            inspector,
                            async_clmv!(inspected_tree, selected_wgt, hit_select, show_selected, {
                                inspector_window::new(inspected, inspected_tree, selected_wgt, hit_select, show_selected)
                            }),
                        );
                    }
                }
            }
            UiNodeOp::Render { .. } | UiNodeOp::RenderUpdate { .. } => {
                INSPECT_CMD.scoped(WINDOW.id()).notify_param(InspectorUpdateOnly::Render);
            }
            _ => {}
        }),
    );

    let child = adorn_selected(child, selected_wgt, show_selected);
    select_on_click(child, hit_select)
}

fn adorn_selected(child: impl UiNode, selected_wgt: impl Var<Option<InspectedWidget>>, enabled: impl Var<bool>) -> impl UiNode {
    use inspector_window::SELECTED_BORDER_VAR;

    let selected_info = selected_wgt.flat_map(|s| {
        if let Some(s) = s {
            s.info().map(|i| Some(i.clone())).boxed()
        } else {
            var(None).boxed()
        }
    });
    let transform_id = SpatialFrameId::new_unique();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_render(&selected_info)
                .sub_var_render(&enabled)
                .sub_var_render(&SELECTED_BORDER_VAR);
        }
        UiNodeOp::Render { frame } => {
            c.render(frame);

            if !enabled.get() {
                return;
            }
            selected_info.with(|w| {
                if let Some(w) = w {
                    let bounds = w.bounds_info();
                    let transform = bounds.inner_transform();
                    let size = bounds.inner_size();

                    frame.push_reference_frame(transform_id.into(), transform.into(), false, false, |frame| {
                        let widths = Dip::new(3).to_px(frame.scale_factor());
                        frame.push_border(
                            PxRect::from_size(size).inflate(widths, widths),
                            PxSideOffsets::new_all_same(widths),
                            SELECTED_BORDER_VAR.get().into(),
                            PxCornerRadius::default(),
                        );
                    });
                }
            });
        }
        _ => {}
    })
}

fn select_on_click(child: impl UiNode, hit_select: impl Var<HitSelect>) -> impl UiNode {
    // when `pending` we need to block interaction with window content, as if a modal
    // overlay was opened, but we can't rebuild info, and we actually want the click target,
    // so we only manually block common pointer events.

    let mut click_handle = EventHandles::dummy();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&hit_select);
        }
        UiNodeOp::Update { .. } => {
            if let Some(h) = hit_select.get_new() {
                if matches!(h, HitSelect::Enabled) {
                    WINDOW.vars().cursor().set(CursorIcon::Crosshair);
                    click_handle.push(crate::core::mouse::MOUSE_INPUT_EVENT.subscribe(WIDGET.id()));
                    click_handle.push(crate::core::touch::TOUCH_INPUT_EVENT.subscribe(WIDGET.id()));
                } else {
                    WINDOW.vars().cursor().set(CursorIcon::Default);
                    click_handle.clear();
                }
            }
        }
        UiNodeOp::Event { update } => {
            if matches!(hit_select.get(), HitSelect::Enabled) {
                let mut select = None;

                if let Some(args) = crate::core::mouse::MOUSE_MOVE_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = crate::core::mouse::MOUSE_INPUT_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                    select = Some(args.target.widget_id());
                } else if let Some(args) = crate::core::mouse::MOUSE_HOVERED_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = crate::core::mouse::MOUSE_WHEEL_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = CLICK_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = crate::core::access::ACCESS_CLICK_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = crate::core::touch::TOUCH_INPUT_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                    select = Some(args.target.widget_id());
                } else if let Some(args) = crate::core::touch::TOUCHED_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = crate::core::touch::TOUCH_MOVE_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = crate::core::touch::TOUCH_TAP_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = crate::core::touch::TOUCH_TRANSFORM_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = crate::core::touch::TOUCH_LONG_PRESS_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                }

                if let Some(id) = select {
                    hit_select.set(HitSelect::Select(id));
                }
            }
        }
        _ => {}
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum HitSelect {
    Disabled,
    Enabled,
    Select(WidgetId),
}

mod inspector_window {
    use std::mem;

    use super::HitSelect;
    use crate::core::inspector::live::*;
    use crate::core::{inspector::*, window::*};
    use crate::prelude::new_widget::*;
    use crate::prelude::*;

    pub(super) fn new(
        inspected: WindowId,
        inspected_tree: InspectedTree,
        selected_wgt: impl Var<Option<InspectedWidget>>,
        hit_select: impl Var<HitSelect>,
        adorn_selected: impl Var<bool>,
    ) -> WindowRoot {
        let parent = WINDOWS.vars(inspected).unwrap().parent().get().unwrap_or(inspected);

        let tree = WINDOWS.widget_tree(inspected).unwrap();
        let title = if let Some(title) = tree.root().inspect_property(property_id!(window::title)) {
            title.downcast_var::<Txt>(0).map(|t| formatx!("{t} - Inspector")).boxed()
        } else {
            var_from("Inspector").boxed()
        };
        let icon = if let Some(icon) = tree.root().inspect_property(property_id!(window::icon)) {
            icon.downcast_var::<WindowIcon>(0).clone().boxed()
        } else {
            var(WindowIcon::Default).boxed()
        };

        let wgt_filter = var(Txt::from_static(""));

        let data_handle = hit_select.on_new(app_hn!(inspected_tree, selected_wgt, |a: &OnVarArgs<HitSelect>, _| {
            if let HitSelect::Select(id) = a.value {
                selected_wgt.set(inspected_tree.inspect(id));
            }
        }));

        Window! {
            parent;
            title;
            icon;
            lang = lang!(en_US);
            width = 1100;
            set_inspected = inspected;
            color_scheme = ColorScheme::Dark;
            on_close = hn!(selected_wgt, |_| {
                selected_wgt.set(None);
            });
            child = Container! {
                child_insert_above = menu(hit_select, adorn_selected, wgt_filter.clone()), 0;
                child = Scroll! {
                    toggle::selector = toggle::Selector::single_opt(selected_wgt.clone());
                    child = tree_view(inspected_tree, wgt_filter.clone());
                    child_align = Align::FILL_TOP;
                    padding = 5;
                };
            };
            child_insert_right = Container! {
                width = 600;
                child = presenter(selected_wgt, wgt_fn!(|w| {
                    selected_view(w).boxed()
                }));
                background_color = SELECTED_BKG_VAR;
            }, 0;

            data = data_handle; // keep alive
        }
    }

    #[property(CONTEXT)]
    fn set_inspected(child: impl UiNode, inspected: impl IntoValue<WindowId>) -> impl UiNode {
        let inspected = inspected.into();
        match_node(child, move |_, op| {
            if let UiNodeOp::Info { info } = op {
                assert!(WIDGET.parent_id().is_none());
                info.set_meta(&INSPECTED_ID, inspected);
            }
        })
    }

    /// Gets the window that is inspected by the current inspector window.
    pub fn inspected() -> Option<WindowId> {
        WINDOW.info().root().meta().get(&INSPECTED_ID).copied()
    }

    pub(super) static INSPECTED_ID: StaticStateId<WindowId> = StaticStateId::new_unique();

    context_var! {
        static TREE_ITEM_BKG_HOVERED_VAR: Rgba = rgb(0.21, 0.21, 0.21);
        static TREE_ITEM_BKG_CHECKED_VAR: Rgba = rgb(0.29, 0.29, 0.29);
        static TREE_ITEM_LINE_VAR: Rgba = rgb(0.21, 0.21, 0.21);
        static WIDGET_ID_COLOR_VAR: Rgba = colors::GRAY;
        static WIDGET_MACRO_COLOR_VAR: Rgba = colors::AZURE;
        static PROPERTY_COLOR_VAR: Rgba = colors::YELLOW;
        static PROPERTY_VALUE_COLOR_VAR: Rgba = colors::ROSE.lighten(50.pct());
        static NEST_GROUP_COLOR_VAR: Rgba = colors::GRAY;
        static SELECTED_BKG_VAR: Rgba = rgb(0.15, 0.15, 0.15);
        static MENU_BKG_VAR: Rgba = rgb(0.13, 0.13, 0.13);
        pub static SELECTED_BORDER_VAR: Rgba = colors::AZURE;
    }

    fn menu(hit_test_select: impl Var<HitSelect>, adorn_selected: impl Var<bool>, search: impl Var<Txt>) -> impl UiNode {
        Container! {
            background_color = MENU_BKG_VAR;
            child_insert_left = Stack! {
                padding = 4;
                spacing = 2;
                direction = StackDirection::left_to_right();
                toggle::extend_style = Style! {
                    padding = 2;
                    corner_radius = 2;
                };
                child_align = Align::CENTER;
                children = ui_vec![
                    Toggle! {
                        child = crosshair_16x16();
                        tooltip = Tip!(Text!("select widget (Ctrl+Shift+C)"));
                        click_shortcut = shortcut!(CTRL|SHIFT+'C');
                        checked = hit_test_select.map_bidi(
                            |c| matches!(c, HitSelect::Enabled),
                            |b| if *b { HitSelect::Enabled } else { HitSelect::Disabled }
                        );
                    },
                    Toggle! {
                        child = Wgt! {
                            size = 16;
                            border = {
                                widths: 3,
                                sides: SELECTED_BORDER_VAR.map_into(),
                            }
                        };
                        tooltip = Tip!(Text!("highlight selected widget"));
                        checked = adorn_selected;
                    },

                ]
            }, 0;
            child = TextInput! {
                access_role = AccessRole::SearchBox;
                margin = (0, 0, 0, 50);
                padding = (3, 5);
                focus_shortcut = [shortcut!['S'], shortcut![CTRL+'F'], shortcut![Find]];
                background = Text! {
                    padding = (4, 6); // +1 border
                    txt = "search widgets (S)";
                    opacity = 50.pct();
                    visibility = search.map(|t| t.is_empty().into());
                };
                txt = search;
            }
        }
    }

    fn crosshair_16x16() -> impl UiNode {
        match_node_leaf(|op| match op {
            UiNodeOp::Layout { final_size, .. } => {
                *final_size = DipSize::splat(Dip::new(16)).to_px(LAYOUT.scale_factor());
            }
            UiNodeOp::Render { frame } => {
                let factor = frame.scale_factor();
                let a = Dip::new(2).to_px(factor);
                let b = Dip::new(16).to_px(factor);
                let m = b / Px(2) - a / Px(2);

                let color = FrameValue::Value(RenderColor::from(colors::WHITE));

                frame.push_color(PxRect::new(PxPoint::new(m, Px(0)), PxSize::new(a, b)), color);
                frame.push_color(PxRect::new(PxPoint::new(Px(0), m), PxSize::new(b, a)), color);
            }
            _ => {}
        })
    }

    fn tree_view(tree: InspectedTree, filter: impl Var<Txt>) -> impl UiNode {
        Container! {
            text::font_family = ["JetBrains Mono", "Consolas", "monospace"];
            child = tree_item_view(tree.inspect_root(), filter, crate::core::var::LocalVar(0u32).boxed());
        }
    }

    fn tree_item_view(wgt: InspectedWidget, filter: impl Var<Txt>, parent_desc_filter: BoxedVar<u32>) -> impl UiNode {
        let wgt_type = wgt.wgt_type();
        let wgt_id = wgt.id();

        let mut pass = false;
        let pass_filter = merge_var!(
            filter.clone(),
            wgt_type,
            clmv!(parent_desc_filter, |f, t| {
                let p = wgt_filter(f, *t, wgt_id);
                if p != pass {
                    pass = p;
                    let _ = parent_desc_filter.modify(move |c| {
                        if pass {
                            *c.to_mut() += 1;
                        } else {
                            *c.to_mut() -= 1;
                        }
                    });
                }
                p
            })
        );

        let descendants_pass_filter = var(0u32).boxed();

        let prev_any_desc = std::sync::atomic::AtomicBool::new(false);
        descendants_pass_filter
            .hook(Box::new(move |a| {
                let any_desc = 0 < *a.value().as_any().downcast_ref::<u32>().unwrap();
                if any_desc != prev_any_desc.swap(any_desc, atomic::Ordering::Relaxed) {
                    let _ = parent_desc_filter.modify(move |c| {
                        if any_desc {
                            *c.to_mut() += 1;
                        } else {
                            *c.to_mut() -= 1;
                        }
                    });
                }
                true
            }))
            .perm();

        Container! {
            when !*#{pass_filter.clone()} && *#{descendants_pass_filter.clone()} == 0 {
                visibility = Visibility::Collapsed;
            }

            child = Toggle! {
                toggle::value = wgt.clone();

                style_fn = StyleFn::nil();
                padding = 2;
                when *#is_hovered {
                    background_color = TREE_ITEM_BKG_HOVERED_VAR;
                }
                when *#toggle::is_checked {
                    background_color = TREE_ITEM_BKG_CHECKED_VAR;
                }

                child = Wrap! {
                    children = ui_vec![
                        Text! {
                            txt = wgt.wgt_macro_name();
                            font_weight = FontWeight::BOLD;
                            font_color = WIDGET_MACRO_COLOR_VAR;

                            when !*#{pass_filter.clone()} {
                                opacity = 50.pct();
                            }
                        },
                        Text!(" {{ "),
                        Text! {
                            txt = formatx!("{:#}", wgt.id());
                            font_color = WIDGET_ID_COLOR_VAR;
                        },
                        Text!(wgt.descendants_len().map(|&l| if l == 0 { Txt::from_static(" }") } else { Txt::from_static("") })),
                    ];
                }
            };

            child_insert_below = presenter(wgt.children(), wgt_fn!(descendants_pass_filter, |children: Vec<InspectedWidget>| {
                let children: UiNodeVec = children.into_iter().map(|c| {
                    tree_item_view(c, filter.clone(), descendants_pass_filter.clone())
                }).collect();
                if children.is_empty() {
                    NilUiNode.boxed()
                } else {
                    Container! {
                        child = Stack! {
                            padding = (0, 0, 0, 2.em());
                            direction = StackDirection::top_to_bottom();
                            children;

                            border = {
                                widths: (0, 0, 0, 1),
                                sides: TREE_ITEM_LINE_VAR.map(|&c| BorderSides::new_left(BorderSide::dashed(c))),
                            };
                        };
                        child_insert_below = Text!("}}"), 0;
                    }.boxed()

                }

            })), 2;
        }
    }

    fn selected_view(wgt: Option<InspectedWidget>) -> impl UiNode {
        if let Some(wgt) = wgt {
            Scroll! {
                mode = ScrollMode::VERTICAL;
                child_align = Align::FILL_TOP;
                padding = 4;
                child = Stack! {
                    direction = StackDirection::top_to_bottom();
                    text::font_family = ["JetBrains Mono", "Consolas", "monospace"];
                    children = ui_vec![
                        Wrap! {
                            children = ui_vec![
                                Text! {
                                    txt = wgt.wgt_macro_name();
                                    font_size = 1.2.em();
                                    font_weight = FontWeight::BOLD;
                                    font_color = WIDGET_MACRO_COLOR_VAR;
                                },
                                Text! {
                                    txt = formatx!(" {:#}", wgt.id());
                                    font_size = 1.2.em();
                                    font_color = WIDGET_ID_COLOR_VAR;
                                },
                                {
                                    let parent_property = wgt.parent_property_name();
                                    Wrap! {
                                        visibility = parent_property.map(|p| (!p.is_empty()).into());
                                        tooltip = Tip!(Text!("parent property"));
                                        children = ui_vec![
                                            Text!(" (in "),
                                            Text! {
                                                txt = parent_property;
                                                font_color = PROPERTY_COLOR_VAR;
                                            },
                                            Text!(")"),
                                        ]
                                    }
                                },
                            ]
                        },
                        presenter(wgt.inspector_info(), wgt_fn!(|i| {
                            if let Some(i) = i {
                                inspector_info_view(i).boxed()
                            } else {
                                NilUiNode.boxed()
                            }
                        })),
                        Hr!(),
                        info_watchers(&wgt),
                    ]
                }
            }
        } else {
            Text! {
                txt_align = Align::TOP;
                padding = 20;
                font_style = FontStyle::Italic;
                txt = formatx!("select a widget to inspect");
            }
        }
    }

    fn inspector_info_view(info: InspectedInfo) -> impl UiNode {
        let mut current_group = None;
        let mut group_items = UiNodeVec::new();
        let mut out = UiNodeVec::new();
        let ctx = &info.context;

        for item in info.items.iter() {
            match item {
                InstanceItem::Property { args, captured } => {
                    let p_info = args.property();
                    let user_assigned = info
                        .builder
                        .property(p_info.id)
                        .map(|p| p.importance == Importance::INSTANCE)
                        .unwrap_or_default();

                    if current_group.as_ref() != Some(&p_info.group) {
                        if let Some(g) = current_group.take() {
                            out.push(nest_group_view(g, mem::take(&mut group_items)));
                        }
                        current_group = Some(p_info.group);
                    }

                    group_items.push(property_view(ctx, &**args, p_info, *captured, user_assigned));
                }
                InstanceItem::Intrinsic { group, name } => {
                    if current_group.as_ref() != Some(group) {
                        if let Some(g) = current_group.take() {
                            out.push(nest_group_view(g, mem::take(&mut group_items)));
                        }
                        current_group = Some(*group);
                    }
                    group_items.push(intrinsic_view(name));
                }
            }
        }

        if !group_items.is_empty() {
            out.push(nest_group_view(current_group.unwrap(), group_items));
        }

        Stack! {
            direction = StackDirection::top_to_bottom();
            children = out;
        }
    }

    fn nest_group_view(group: NestGroup, mut items: UiNodeVec) -> impl UiNode {
        items.insert(
            0,
            Text! {
                txt = formatx!("// {}", group.name());
                tooltip = Tip!(Text!("nest group"));
                margin = (10, 0, 0, 0);
                font_color = NEST_GROUP_COLOR_VAR;
            },
        );

        Stack! {
            direction = StackDirection::top_to_bottom();
            spacing = 3;
            children = items;
        }
    }

    fn value_background(value: &BoxedVar<Txt>) -> impl Var<Rgba> {
        let flash = var(rgba(0, 0, 0, 0));
        let mut flashing = None;
        value
            .on_pre_new(app_hn!(flash, |_, _| {
                let h = flash.set_ease(colors::BLACK, colors::BLACK.transparent(), 500.ms(), easing::linear);
                flashing = Some(h);
            }))
            .perm();
        flash
    }

    fn property_view(
        ctx: &InspectorContext,
        args: &dyn PropertyArgs,
        info: PropertyInfo,
        captured: bool,
        user_assigned: bool,
    ) -> impl UiNode {
        // TODO, indicators for user or widget set properties.
        let mut ctx = ctx.latest_capture();
        let mut children = ui_vec![
            Text! {
                txt = info.name;
                font_color = PROPERTY_COLOR_VAR;
                tooltip = Tip!(Text!(if captured {
                    "captured property"
                } else {
                    "property"
                }));
            },
            Text!(" = "),
        ];
        if info.inputs.len() == 1 {
            let value = ctx.with_context(|| args.live_debug(0));
            let flash = value_background(&value);

            children.push(Text! {
                txt = value;
                font_color = PROPERTY_VALUE_COLOR_VAR;
                background_color = flash;
                tooltip = Tip!(Text!(if user_assigned {
                    "instance value"
                } else {
                    "intrinsic value"
                }))
            });
            children.push(Text!(";"));
        } else {
            children.push(Text!("{{\n"));
            for (i, input) in info.inputs.iter().enumerate() {
                children.push(Text!("    {}: ", input.name));

                let value = ctx.with_context(|| args.live_debug(i));
                let flash = value_background(&value);

                children.push(Text! {
                    txt = value;
                    font_color = PROPERTY_VALUE_COLOR_VAR;
                    background_color = flash;
                });
                children.push(Text!(",\n"));
            }
            children.push(Text!("}};"));
        }

        Wrap! {
            children;
        }
    }

    fn intrinsic_view(name: &'static str) -> impl UiNode {
        Text! {
            txt = name;
            font_style = FontStyle::Italic;
            tooltip = Tip!(Text!("intrinsic node"));
        }
    }

    fn info_watchers(wgt: &InspectedWidget) -> impl UiNode {
        let mut children = UiNodeVec::new();
        children.push(Text! {
            txt = "interactivity: ";
        });

        let value = wgt.info().map(|i| formatx!("{:?}", i.interactivity())).boxed();
        let flash = value_background(&value);
        children.push(Text! {
            txt = value;
            font_color = PROPERTY_VALUE_COLOR_VAR;
            background_color = flash;
        });

        children.push(Text! {
            txt = ",\nvisibility: ";
        });
        let value = wgt.render_watcher(|i| formatx!("{:?}", i.visibility())).boxed();
        let flash = value_background(&value);
        children.push(Text! {
            txt = value;
            font_color = PROPERTY_VALUE_COLOR_VAR;
            background_color = flash;
        });

        children.push(Text! {
            txt = ",\ninner_bounds: ";
        });
        let value = wgt.render_watcher(|i| formatx!("{:?}", i.bounds_info().inner_bounds())).boxed();
        let flash = value_background(&value);
        children.push(Text! {
            txt = value;
            font_color = PROPERTY_VALUE_COLOR_VAR;
            background_color = flash;
        });
        children.push(Text! {
            txt = ",";
        });

        Stack! {
            direction = StackDirection::top_to_bottom();
            spacing = 3;
            children = ui_vec![
                Text! {
                    txt = formatx!("/* INFO */");
                    tooltip = Tip!(Text!("watched widget info"));
                    font_color = NEST_GROUP_COLOR_VAR;
                },
                Wrap!(children),
            ];
        }
    }

    fn wgt_filter(filter: &str, wgt_ty: Option<WidgetType>, wgt_id: WidgetId) -> bool {
        if filter.is_empty() {
            return true;
        }

        if let Some(t) = wgt_ty {
            if let Some(tn) = filter.strip_suffix('!') {
                if t.name() == tn {
                    return true;
                }
            } else if t.name().contains(filter) {
                return true;
            }
        }

        if wgt_id.name().contains(filter) {
            return true;
        }

        if let Some(f) = filter.strip_prefix('#') {
            if let Ok(i) = f.parse::<u64>() {
                if wgt_id.sequential() == i {
                    return true;
                }
            }
        }

        false
    }
}