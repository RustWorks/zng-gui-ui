//! Zero-Ui is the pure Rust GUI framework with batteries included.
//!
//! It provides all that you need to create a beautiful, fast and responsive multi-platform GUI apps, it includes many features
//! that allow you to get started quickly, without sacrificing customization or performance. With features like gesture events,
//! common widgets, layouts, data binding, async tasks, accessibility and localization
//! you can focus on what makes your app unique, not the boilerplate required to get modern apps up to standard.
//!
//! When you do need to customize, Zero-Ui is rightly flexible, you can create new widgets or customize existing ones, not just
//! new looks but new behavior, at a lower level you can introduce new event types or new event sources, making custom hardware seamless
//! integrate into the framework.
//!
//! # Usage
//!
//! First add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! zero-ui = "0.1"
//! ```
//!
//! Then create your first window:
//!
//! ```rust
//! # fn main() { }
//! use zero_ui::prelude::*;
//!
//! fn run() {
//!     APP.defaults().run_window(async {
//!         let size = var_from((800, 600));
//!         Window! {
//!             title = size.map(|s: &Size| formatx!("Button Example - {}", s));
//!             size;
//!             child_align = Align::CENTER;
//!             child = Button! {
//!                 on_click = hn!(|_| {
//!                     println!("Button clicked!");
//!                 });
//!                 text::font_size = 28;
//!                 child = Text!("Click Me!");
//!             }
//!         }
//!     })
//! }
//! ```

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

// manually expanded enable_widget_macros to avoid error running doc tests:
//  macro-expanded `extern crate` items cannot shadow names passed with `--extern`
#[doc(hidden)]
#[allow(unused_extern_crates)]
extern crate self as zero_ui;
#[doc(hidden)]
pub use zero_ui_app::__proc_macro_util;

/// Types for general app development.
pub mod prelude {
    pub use crate::APP;
    pub use crate::{gesture, keyboard, layout, mouse, touch, widget};

    pub use zero_ui_task::rayon::prelude::{
        FromParallelIterator as _, IndexedParallelIterator as _, IntoParallelIterator as _, IntoParallelRefIterator as _,
        IntoParallelRefMutIterator as _, ParallelBridge as _, ParallelDrainFull as _, ParallelDrainRange as _, ParallelExtend as _,
        ParallelIterator as _, ParallelSlice as _, ParallelSliceMut as _, ParallelString as _,
    };

    pub use zero_ui_app::{
        event::{AnyEventArgs as _, Command, CommandHandle, CommandInfoExt as _, CommandNameExt as _, EventArgs as _},
        handler::{app_hn, app_hn_once, async_app_hn, async_app_hn_once, async_hn, async_hn_once, hn, hn_once, AppHandler, WidgetHandler},
        shortcut::{shortcut, CommandShortcutExt as _, Shortcut, ShortcutFilter, Shortcuts},
        timer::{DeadlineHandle, DeadlineVar, TimerHandle, TimerVar, TIMERS},
        update::{UpdateOp, UPDATES},
        widget::{
            border::{BorderSides, BorderStyle, CornerRadius, CornerRadiusFit, LineOrientation, LineStyle},
            easing,
            info::{InteractionPath, Interactivity, Visibility, WidgetInfo, WidgetPath},
            instance::{
                ui_vec, ArcNode, ArcNodeList, BoxedUiNode, BoxedUiNodeList, EditableUiNodeList, EditableUiNodeListRef, SortingList, UiNode,
                UiNodeList, UiNodeListChain as _, UiNodeListObserver, UiNodeOp, UiNodeVec, ZIndex,
            },
            AnyVarSubscribe as _, StaticWidgetId, VarLayout as _, VarSubscribe as _, WidgetId, WIDGET,
        },
        window::{MonitorId, StaticWindowId, WindowId, WINDOW},
    };

    pub use zero_ui_app::widget::inspector::WidgetInfoInspectorExt as _;

    pub use zero_ui_var::{
        context_var, expr_var, impl_from_and_into_var, merge_var, response_done_var, response_var, state_var, var, var_from, when_var,
        AnyVar as _, ArcVar, BoxedVar, ContextVar, IntoValue, IntoVar, LocalVar, ObservableVec, ReadOnlyArcVar, ResponderVar, ResponseVar,
        Var, VarCapabilities, VarHandle, VarHandles, VarValue,
    };

    pub use crate::var::animation::easing;

    pub use zero_ui_layout::units::{
        Align, AngleDegree, AngleGradian, AngleRadian, AngleUnits as _, ByteUnits as _, Deadline, Dip, DipBox, DipPoint, DipRect,
        DipSideOffsets, DipSize, DipToPx as _, DipVector, Factor, Factor2d, FactorPercent, FactorSideOffsets, FactorUnits as _,
        Layout1d as _, Layout2d as _, LayoutAxis, Length, LengthUnits as _, Line, LineFromTuplesBuilder as _, Point, Px, PxBox,
        PxConstraints, PxConstraints2d, PxCornerRadius, PxLine, PxPoint, PxRect, PxSideOffsets, PxSize, PxToDip as _, PxTransform,
        PxVector, Rect, RectFromTuplesBuilder as _, ResolutionUnits as _, SideOffsets, Size, TimeUnits as _, Transform, Vector,
    };

    pub use zero_ui_txt::{formatx, ToText as _, Txt};

    pub use zero_ui_clone_move::{async_clmv, async_clmv_fn, async_clmv_fn_once, clmv};

    pub use crate::task;

    pub use zero_ui_app_context::{app_local, context_local, RunOnDrop};

    pub use zero_ui_state_map::{state_map, OwnedStateMap, StateId, StateMapMut, StateMapRef, StaticStateId};

    pub use zero_ui_color::{
        color_scheme_highlight, color_scheme_map, color_scheme_pair, colors, gradient, hex, hsl, hsla, hsv, hsva, rgb, rgba, web_colors,
        ColorPair, ColorScheme, Hsla, Hsva, MixBlendMode, Rgba,
    };

    pub use zero_ui_ext_clipboard::CLIPBOARD;

    pub use zero_ui_ext_config::CONFIG;

    pub use zero_ui_ext_font::{
        font_features, FontSize, FontStretch, FontStyle, FontWeight, Hyphens, Justify, TextTransformFn, WhiteSpace, WordBreak, WordSpacing,
    };

    pub use zero_ui_ext_fs_watcher::WATCHER;

    pub use zero_ui_ext_image::{ImageSource, IMAGES};

    pub use zero_ui_wgt_image::Image;

    pub use zero_ui_ext_input::{
        focus::{commands::CommandFocusExt as _, iter::IterFocusableExt as _, WidgetInfoFocusExt as _, FOCUS},
        gesture::{ClickArgs, CommandShortcutMatchesExt as _, HeadlessAppGestureExt as _},
        keyboard::{HeadlessAppKeyboardExt as _, KeyInputArgs},
        mouse::{ClickMode, ClickTrigger, WidgetInfoMouseExt as _},
        pointer_capture::CaptureMode,
    };

    pub use zero_ui_ext_l10n::{l10n, lang, Lang, L10N};

    pub use zero_ui_wgt_text::lang;

    pub use zero_ui_ext_undo::{CommandUndoExt as _, REDO_CMD, UNDO, UNDO_CMD};

    pub use zero_ui_ext_window::{
        AppRunWindowExt as _, AutoSize, HeadlessAppWindowExt as _, RenderMode, StartPosition, WINDOW_Ext as _, WidgetInfoImeArea as _,
        WindowChrome, WindowCloseRequestedArgs, WindowIcon, WindowRoot, WINDOWS,
    };

    pub use zero_ui_wgt::Wgt;

    pub use crate::text;
    pub use zero_ui_wgt_text::Text;

    pub use zero_ui_wgt_text_input::TextInput;

    pub use zero_ui_wgt_window::Window;

    pub use zero_ui_wgt_container::Container;

    pub use zero_ui_wgt_button::Button;

    pub use zero_ui_wgt_data::{data, DATA};

    pub use crate::grid;
    pub use zero_ui_wgt_grid::Grid;

    pub use crate::layer;
    pub use zero_ui_wgt_layer::{AnchorMode, LayerIndex, LAYERS};

    pub use zero_ui_wgt_text::icon::CommandIconExt as _;

    pub use crate::popup;
    pub use zero_ui_wgt_layer::popup::POPUP;

    pub use crate::menu;
    pub use zero_ui_wgt_menu::{
        context::{context_menu, context_menu_fn, ContextMenu},
        sub::SubMenu,
        Menu,
    };

    pub use zero_ui_wgt_rule_line::hr::Hr;

    pub use zero_ui_wgt_scroll::{Scroll, SCROLL};

    pub use crate::toggle;
    pub use zero_ui_wgt_toggle::Toggle;

    pub use crate::tip;
    pub use zero_ui_wgt_tooltip::{tooltip, tooltip_fn, Tip};

    pub use zero_ui_wgt::{wgt_fn, WidgetFn};

    pub use zero_ui_wgt_style::{style_fn, Style};

    pub use zero_ui_wgt_stack::{Stack, StackDirection};

    pub use zero_ui_wgt_wrap::Wrap;
}

/// Prelude for declaring properties and widgets.
pub mod wgt_prelude {
    pub use zero_ui_app::{
        event::{
            command, event, event_args, AnyEventArgs as _, Command, CommandHandle, CommandInfoExt as _, CommandNameExt as _, Event,
            EventArgs as _, EventHandle, EventHandles, EventPropagationHandle,
        },
        handler::{app_hn, app_hn_once, async_app_hn, async_app_hn_once, async_hn, async_hn_once, hn, hn_once, AppHandler, WidgetHandler},
        render::{FrameBuilder, FrameUpdate, FrameValue, FrameValueKey, FrameValueUpdate, SpatialFrameId, TransformStyle},
        shortcut::{shortcut, CommandShortcutExt as _, Shortcut, ShortcutFilter, Shortcuts},
        timer::{DeadlineHandle, DeadlineVar, TimerHandle, TimerVar, TIMERS},
        update::{EventUpdate, UpdateDeliveryList, UpdateOp, WidgetUpdates, UPDATES},
        widget::{
            base::{WidgetBase, WidgetImpl},
            border::{BorderSides, BorderStyle, CornerRadius, CornerRadiusFit, LineOrientation, LineStyle, BORDER},
            builder::{property_id, NestGroup, WidgetBuilder, WidgetBuilding},
            easing,
            info::{
                InteractionPath, Interactivity, Visibility, WidgetBorderInfo, WidgetBoundsInfo, WidgetInfo, WidgetInfoBuilder,
                WidgetLayout, WidgetMeasure, WidgetPath,
            },
            instance::{
                match_node, match_node_leaf, match_node_list, match_node_typed, match_widget, ui_vec, ArcNode, ArcNodeList, BoxedUiNode,
                BoxedUiNodeList, EditableUiNodeList, EditableUiNodeListRef, FillUiNode, NilUiNode, PanelList, SortingList, UiNode,
                UiNodeList, UiNodeListChain as _, UiNodeListObserver, UiNodeOp, UiNodeVec, ZIndex, SORTING_LIST,
            },
            property, ui_node, widget, widget_impl, widget_mixin, widget_set, AnyVarSubscribe as _, VarLayout as _, VarSubscribe as _,
            WidgetId, WidgetUpdateMode, WIDGET,
        },
        window::{MonitorId, WindowId, WINDOW},
    };

    pub use zero_ui_var::{
        context_var, expr_var, impl_from_and_into_var, merge_var, response_done_var, response_var, state_var, var, when_var, AnyVar as _,
        ArcVar, BoxedVar, ContextVar, IntoValue, IntoVar, LocalVar, ReadOnlyArcVar, ResponderVar, ResponseVar, Var, VarCapabilities,
        VarHandle, VarHandles, VarValue,
    };

    pub use zero_ui_layout::{
        context::{LayoutDirection, LayoutMetrics, DIRECTION_VAR, LAYOUT},
        units::{
            Align, AngleDegree, AngleGradian, AngleRadian, AngleUnits as _, ByteUnits as _, Deadline, Dip, DipBox, DipPoint, DipRect,
            DipSideOffsets, DipSize, DipToPx as _, DipVector, Factor, Factor2d, FactorPercent, FactorSideOffsets, FactorUnits as _,
            Layout1d as _, Layout2d as _, LayoutAxis, Length, LengthUnits as _, Line, LineFromTuplesBuilder as _, Point, Px, PxBox,
            PxConstraints, PxConstraints2d, PxCornerRadius, PxLine, PxPoint, PxRect, PxSideOffsets, PxSize, PxToDip as _, PxTransform,
            PxVector, Rect, RectFromTuplesBuilder as _, ResolutionUnits as _, SideOffsets, Size, TimeUnits as _, Transform, Vector,
        },
    };

    pub use zero_ui_txt::{formatx, ToText as _, Txt};

    pub use zero_ui_clone_move::{async_clmv, async_clmv_fn, async_clmv_fn_once, clmv};

    pub use crate::task;

    pub use zero_ui_app_context::{
        app_local, context_local, CaptureFilter, ContextLocal, ContextValueSet, FullLocalContext, LocalContext, RunOnDrop,
    };

    pub use zero_ui_state_map::{state_map, OwnedStateMap, StateId, StateMapMut, StateMapRef, StaticStateId};

    pub use zero_ui_wgt::prelude::{IdEntry, IdMap, IdSet};

    pub use zero_ui_wgt::{wgt_fn, WidgetFn};

    pub use zero_ui_color::{
        color_scheme_highlight, color_scheme_map, color_scheme_pair, colors, gradient, hex, hsl, hsla, hsv, hsva, rgb, rgba, web_colors,
        ColorPair, ColorScheme, Hsla, Hsva, MixBlendMode, Rgba,
    };

    pub use zero_ui_wgt::nodes::{
        bind_is_state, border_node, command_property, event_is_state, event_is_state2, event_is_state3, event_is_state4, event_property,
        fill_node, list_presenter, presenter, presenter_opt, widget_state_get_state, widget_state_is_state, with_context_blend,
        with_context_local, with_context_local_init, with_context_var, with_context_var_init, with_widget_state, with_widget_state_modify,
    };

    pub use zero_ui_ext_window::WidgetInfoBuilderImeArea as _;
}

pub use zero_ui_state_map as state_map;

pub use zero_ui_clone_move::{async_clmv, async_clmv_fn, async_clmv_fn_once, clmv};

/// Parallel async tasks and async task runners.
///
/// This module fully re-exports [`zero_ui_task`], it provides common async utilities, all contextualized
/// in the running [`app::LocalContext`]. See the [`zero_ui_task`] crate level documentation for more details.
pub mod task {
    pub use zero_ui_task::*;

    pub use zero_ui_app::widget::UiTaskWidget;
}

/// Color and gradient types, functions, properties and macros.
///
/// See [`zero_ui_color`], [`zero_ui_wgt_filter`] and [`zero_ui_wgt_fill`] for the full API.
pub mod color {
    pub use zero_ui_color::{
        color_scheme_highlight, color_scheme_map, color_scheme_pair, colors, hex, hsl, hsla, hsla_sampler, hsv, hsva, lerp_space,
        linear_hsla_sampler, rgb, rgba, rgba_sampler, web_colors, with_lerp_space, ColorPair, ColorScheme, Hsla, Hsva, LerpSpace,
        MixBlendMode, PreMulRgba, RenderColor, RenderMixBlendMode, Rgba, COLOR_SCHEME_VAR,
    };

    pub use zero_ui_wgt::color_scheme;

    pub use zero_ui_wgt_fill::nodes::flood;

    /// Color filters.
    pub mod filter {
        pub use zero_ui_color::filter::{ColorMatrix, Filter, RenderFilter};

        pub use zero_ui_wgt_filter::{
            backdrop_blur, backdrop_brightness, backdrop_color_matrix, backdrop_contrast, backdrop_filter, backdrop_grayscale,
            backdrop_hue_rotate, backdrop_invert, backdrop_saturate, backdrop_sepia, blur, brightness, child_filter, child_mix_blend,
            child_opacity, color_matrix, contrast, drop_shadow, filter, grayscale, hue_rotate, invert_color, mix_blend, opacity, saturate,
            sepia,
        };
    }

    /// Color gradient.
    pub mod gradient {
        pub use zero_ui_color::gradient::{
            stops, ColorStop, ExtendMode, GradientRadius, GradientRadiusBase, GradientStop, GradientStops, LinearGradientAxis,
            RenderExtendMode, RenderGradientStop,
        };

        pub use zero_ui_wgt_fill::nodes::{
            conic_gradient, gradient, linear_gradient, radial_gradient, ConicGradient, GradientBuilder, LinearGradient, RadialGradient,
            TiledConicGradient, TiledLinearGradient, TiledRadialGradient,
        };
    }
}

/// Layout service, units and other types.
///
/// See [`zero_ui_layout`], [`zero_ui_wgt_transform`] and [`zero_ui_wgt_size_offset`] for the full API.
pub mod layout {
    pub use zero_ui_layout::units::{
        slerp_enabled, slerp_sampler, Align, AngleDegree, AngleGradian, AngleRadian, AngleTurn, AngleUnits, BoolVector2D, ByteLength,
        ByteUnits, CornerRadius2D, Deadline, Dip, DipBox, DipCornerRadius, DipPoint, DipRect, DipSideOffsets, DipSize, DipToPx, DipVector,
        DistanceKey, Factor, Factor2d, FactorPercent, FactorSideOffsets, FactorUnits, GridSpacing, Layout1d, Layout2d, LayoutAxis,
        LayoutMask, Length, LengthExpr, LengthUnits, Line, LineFromTuplesBuilder, Orientation2D, Point, Ppi, Ppm, Px, PxBox, PxConstraints,
        PxConstraints2d, PxCornerRadius, PxGridSpacing, PxLine, PxPoint, PxRect, PxSideOffsets, PxSize, PxToDip, PxTransform, PxVector,
        Rect, RectFromTuplesBuilder, RenderAngle, ResolutionUnits, SideOffsets, SideOffsets2D, Size, TimeUnits, Transform, Vector,
    };

    pub use zero_ui_layout::context::{
        InlineConstraints, InlineConstraintsLayout, InlineConstraintsMeasure, InlineSegment, InlineSegmentPos, LayoutDirection,
        LayoutMetrics, LayoutMetricsSnapshot, LayoutPassId, TextSegmentKind, DIRECTION_VAR, LAYOUT,
    };

    pub use zero_ui_app::widget::info::{WidgetLayout, WidgetMeasure};

    pub use zero_ui_wgt_transform::{
        backface_visibility, perspective, perspective_origin, rotate, rotate_x, rotate_y, rotate_z, scale, scale_x, scale_xy, scale_y,
        skew, skew_x, skew_y, transform, transform_origin, transform_style, translate, translate_x, translate_y, translate_z,
    };

    pub use zero_ui_wgt_size_offset::{
        actual_bounds, actual_height, actual_height_px, actual_size, actual_size_px, actual_transform, actual_width, actual_width_px,
        baseline, height, max_height, max_size, max_width, min_height, min_size, min_width, offset, size, sticky_height, sticky_size,
        sticky_width, width, x, y, WidgetLength, WIDGET_SIZE,
    };

    pub use zero_ui_wgt::{align, inline, is_ltr, is_rtl, margin, InlineMode};

    pub use zero_ui_wgt_container::{child_align, padding};

    pub use zero_ui_app::render::TransformStyle;
}

/// Frame builder and other types.
///
/// See [`zero_ui_app::render`] for the full API.
pub mod render {
    pub use zero_ui_app::render::{
        ClipBuilder, Font, FontSynthesis, FrameBuilder, FrameUpdate, FrameValue, FrameValueKey, FrameValueUpdate, HitTestBuilder,
        HitTestClipBuilder, ImageRendering, RepeatMode, SpatialFrameId, SpatialFrameKey, StaticSpatialFrameId,
    };
    pub use zero_ui_view_api::window::FrameId;
}

/// Variables API.
///
/// See [`zero_ui_var`] for the full var API.
pub mod var {
    pub use zero_ui_var::types::{
        AnyWhenVarBuilder, ArcCowVar, ArcWhenVar, ContextualizedVar, ReadOnlyVar, Response, VecChange, WeakArcVar, WeakContextInitHandle,
        WeakContextualizedVar, WeakReadOnlyVar, WeakWhenVar,
    };
    pub use zero_ui_var::{
        context_var, expr_var, getter_var, merge_var, response_done_var, response_var, state_var, var, var_default, var_from, when_var,
        AnyVar, AnyVarValue, AnyWeakVar, ArcEq, ArcVar, BoxedAnyVar, BoxedAnyWeakVar, BoxedVar, BoxedWeakVar, ContextInitHandle,
        ContextVar, IntoValue, IntoVar, LocalVar, MergeVarBuilder, ObservableVec, ReadOnlyArcVar, ReadOnlyContextVar, ResponderVar,
        ResponseVar, TraceValueArgs, Var, VarCapabilities, VarHandle, VarHandles, VarHookArgs, VarModify, VarPtr, VarUpdateId, VarValue,
        WeakVar, VARS,
    };

    pub use zero_ui_app::widget::{AnyVarSubscribe, VarLayout, VarSubscribe};

    /// Var animation types and functions.
    pub mod animation {
        pub use zero_ui_var::animation::{
            Animation, AnimationController, AnimationHandle, AnimationTimer, ChaseAnimation, ModifyInfo, NilAnimationObserver, Transition,
            TransitionKeyed, Transitionable, WeakAnimationHandle,
        };

        /// Common easing functions.
        pub mod easing {
            pub use zero_ui_var::animation::easing::{
                back, bounce, circ, cubic, cubic_bezier, ease_in, ease_in_out, ease_out, ease_out_in, elastic, expo, linear, none, quad,
                quart, quint, reverse, reverse_out, sine, step_ceil, step_floor, Bezier, EasingFn, EasingModifierFn, EasingStep,
                EasingTime,
            };
        }
    }
}

/// App extensions, context, events and commands API.
///
/// See [`zero_ui_app`] and [`zero_ui_app_context`] for the full API.
pub mod app {
    pub use zero_ui_app::{
        AppEventObserver, AppExtended, AppExtension, AppExtensionBoxed, AppExtensionInfo, ControlFlow, ExitRequestedArgs, HeadlessApp,
        EXIT_CMD, EXIT_REQUESTED_EVENT,
    };
    pub use zero_ui_app_context::{
        app_local, context_local, AppId, AppLocal, AppScope, CaptureFilter, ContextLocal, ContextValueSet, FullLocalContext, LocalContext,
        RunOnDrop, StaticAppId,
    };
    pub use zero_ui_wgt_input::commands::{
        on_new, on_open, on_pre_new, on_pre_open, on_pre_save, on_pre_save_as, on_save, on_save_as, NEW_CMD, OPEN_CMD, SAVE_AS_CMD,
        SAVE_CMD,
    };
}

/// Event and command API.
///
/// See [`zero_ui_app::event`] for the full event API.
pub mod event {
    pub use zero_ui_app::event::{
        command, event, event_args, AnyEvent, AnyEventArgs, Command, CommandArgs, CommandHandle, CommandInfoExt, CommandMeta,
        CommandMetaVar, CommandMetaVarId, CommandNameExt, CommandParam, CommandScope, Event, EventArgs, EventHandle, EventHandles,
        EventPropagationHandle, EventReceiver, EVENTS,
    };
    pub use zero_ui_wgt::nodes::{command_property, event_property, on_command, on_event, on_pre_command, on_pre_event};
}

/// App update service and types.
///
/// See [`zero_ui_app::update`] for the full update API.
pub mod update {
    pub use zero_ui_app::update::{
        ContextUpdates, EventUpdate, InfoUpdates, LayoutUpdates, OnUpdateHandle, RenderUpdates, UpdateArgs, UpdateDeliveryList, UpdateOp,
        UpdateSubscribers, UpdatesTraceUiNodeExt, WeakOnUpdateHandle, WidgetUpdates, UPDATES,
    };
}

/// App timers service and types.
///
/// See [`zero_ui_app::timer`] for the full time API. Also see [`task::deadline`] for a timer decoupled from the app loop.
pub mod timer {
    pub use zero_ui_app::timer::{
        DeadlineArgs, DeadlineHandle, DeadlineVar, Timer, TimerArgs, TimerHandle, TimerVar, WeakDeadlineHandle, WeakTimerHandle, TIMERS,
    };
}

/// Widget info, builder and base, UI node and list.
///
/// See [`zero_ui_app::widget`] for the full API.
pub mod widget {
    pub use zero_ui_app::widget::base::{HitTestMode, Parallel, WidgetBase, WidgetExt, WidgetImpl, PARALLEL_VAR};

    pub use zero_ui_app::widget::{
        easing, property, ui_node, widget, widget_mixin, widget_set, StaticWidgetId, WidgetId, WidgetUpdateMode, WIDGET,
    };

    pub use zero_ui_app::widget::border::{
        BorderSide, BorderSides, BorderStyle, CornerRadius, CornerRadiusFit, LineOrientation, LineStyle, BORDER,
    };

    pub use zero_ui_wgt::{
        border, border_align, border_over, can_auto_hide, clip_to_bounds, corner_radius, corner_radius_fit, enabled, hit_test_mode, inline,
        interactive, is_collapsed, is_disabled, is_enabled, is_hidden, is_hit_testable, is_inited, is_visible, modal, on_block,
        on_blocked_changed, on_deinit, on_disable, on_enable, on_enabled_changed, on_info_init, on_init, on_interactivity_changed, on_move,
        on_node_op, on_pre_block, on_pre_blocked_changed, on_pre_deinit, on_pre_disable, on_pre_enable, on_pre_enabled_changed,
        on_pre_init, on_pre_interactivity_changed, on_pre_move, on_pre_node_op, on_pre_transform_changed, on_pre_unblock, on_pre_update,
        on_pre_vis_disable, on_pre_vis_enable, on_pre_vis_enabled_changed, on_transform_changed, on_unblock, on_update, on_vis_disable,
        on_vis_enable, on_vis_enabled_changed, parallel, visibility, wgt_fn, z_index, OnDeinitArgs, OnNodeOpArgs, Wgt, WidgetFn,
    };

    pub use zero_ui_wgt_fill::{
        background, background_color, background_conic, background_fn, background_gradient, background_radial, foreground,
        foreground_color, foreground_fn, foreground_gradient, foreground_highlight,
    };

    /// Widget and property builder types.
    ///
    /// See [`zero_ui_app::widget::builder`] for the full API.
    pub mod builder {
        pub use zero_ui_app::widget::builder::{
            property_args, property_id, property_info, property_input_types, source_location, widget_type, AnyWhenArcWidgetHandlerBuilder,
            ArcWidgetHandler, BuilderProperty, BuilderPropertyMut, BuilderPropertyRef, Importance, InputKind, NestGroup, NestPosition,
            PropertyArgs, PropertyBuildAction, PropertyBuildActionArgs, PropertyBuildActions, PropertyBuildActionsWhenData, PropertyId,
            PropertyInfo, PropertyInput, PropertyInputTypes, PropertyNewArgs, SourceLocation, WhenBuildAction, WhenInfo, WhenInput,
            WhenInputMember, WhenInputVar, WidgetBuilder, WidgetBuilderProperties, WidgetBuilding, WidgetType,
        };
    }

    /// Widget info tree and info builder.
    pub mod info {
        pub use zero_ui_app::widget::info::{
            iter, HitInfo, HitTestInfo, InlineSegmentInfo, InteractionPath, Interactivity, InteractivityChangedArgs,
            InteractivityFilterArgs, ParallelBuilder, RelativeHitZ, TransformChangedArgs, TreeFilter, Visibility, VisibilityChangedArgs,
            WidgetBorderInfo, WidgetBoundsInfo, WidgetDescendantsRange, WidgetInfo, WidgetInfoBuilder, WidgetInfoChangedArgs,
            WidgetInfoMeta, WidgetInfoTree, WidgetInfoTreeStats, WidgetInlineInfo, WidgetInlineMeasure, WidgetPath,
            INTERACTIVITY_CHANGED_EVENT, TRANSFORM_CHANGED_EVENT, VISIBILITY_CHANGED_EVENT, WIDGET_INFO_CHANGED_EVENT,
        };

        /// Accessibility metadata types.
        pub mod access {
            pub use zero_ui_app::widget::info::access::{AccessBuildArgs, WidgetAccessInfo, WidgetAccessInfoBuilder};
        }

        /// Helper types for inspecting an UI tree.
        pub mod inspector {
            pub use zero_ui_app::widget::inspector::{
                InspectPropertyPattern, InspectWidgetPattern, InspectorContext, InspectorInfo, InstanceItem, WidgetInfoInspectorExt,
            };
        }
    }

    /// Widget instance types, [`UiNode`], [`UiNodeList`] and others.
    ///
    /// [`UiNode`]: crate::prelude::UiNode
    /// [`UiNodeList`]: crate::prelude::UiNodeList
    pub mod instance {
        // !!: TODO, rename to node?
        pub use zero_ui_app::widget::instance::{
            extend_widget, match_node, match_node_leaf, match_node_list, match_node_typed, match_widget, ui_vec, AdoptiveChildNode,
            AdoptiveNode, ArcNode, ArcNodeList, BoxedUiNode, BoxedUiNodeList, DefaultPanelListData, EditableUiNodeList,
            EditableUiNodeListRef, FillUiNode, MatchNodeChild, MatchNodeChildren, MatchWidgetChild, NilUiNode, OffsetUiListObserver,
            PanelList, PanelListData, PanelListRange, PanelListRangeHandle, SortingList, UiNode, UiNodeList, UiNodeListChain,
            UiNodeListChainImpl, UiNodeListObserver, UiNodeOp, UiNodeOpMethod, UiNodeVec, WeakNode, WeakNodeList, WhenUiNodeBuilder,
            WhenUiNodeListBuilder, ZIndex, SORTING_LIST, Z_INDEX,
        };

        pub use zero_ui_wgt::nodes::{
            bind_is_state, border_node, event_is_state, event_is_state2, event_is_state3, event_is_state4, fill_node, interactive_node,
            list_presenter, presenter, presenter_opt, validate_getter_var, widget_state_get_state, widget_state_is_state,
            with_context_blend, with_context_local, with_context_local_init, with_context_var, with_context_var_init, with_index_len_node,
            with_index_node, with_rev_index_node, with_widget_state, with_widget_state_modify,
        };
    }
}

/// Event handler API.
///
/// See [`zero_ui_app::handler`] for the full handler API.
pub mod handler {
    pub use zero_ui_app::handler::{
        app_hn, app_hn_once, async_app_hn, async_app_hn_once, async_hn, async_hn_once, hn, hn_once, AppHandler, AppHandlerArgs,
        AppWeakHandle, WidgetHandler,
    };
}

/// Clipboard service, commands and types.
///
/// See [`zero_ui_ext_clipboard`] for the full clipboard API.
pub mod clipboard {
    pub use zero_ui_ext_clipboard::{ClipboardError, CLIPBOARD, COPY_CMD, CUT_CMD, PASTE_CMD};
    pub use zero_ui_wgt_input::commands::{on_copy, on_cut, on_paste, on_pre_copy, on_pre_cut, on_pre_paste};
}

/// Config service, sources and types.
///
/// See [`zero_ui_ext_config`] for the full config API.
pub mod config {
    pub use zero_ui_ext_config::{
        AnyConfig, Config, ConfigKey, ConfigMap, ConfigStatus, ConfigValue, ConfigVars, FallbackConfig, FallbackConfigReset, JsonConfig,
        MemoryConfig, RawConfigValue, ReadOnlyConfig, RonConfig, SwapConfig, SwitchConfig, SyncConfig, TomlConfig, YamlConfig, CONFIG,
    };
}

/// Fonts service and text shaping.
///
/// See [`zero_ui_ext_font`] for the full font and shaping API.
pub mod font {
    pub use zero_ui_ext_font::{
        font_features, unicode_bidi_levels, unicode_bidi_sort, BidiLevel, CaretIndex, ColorGlyph, ColorGlyphs, ColorPalette,
        ColorPaletteType, ColorPalettes, CustomFont, Font, FontChange, FontChangedArgs, FontColorPalette, FontDataRef, FontFace,
        FontFaceList, FontFaceMetrics, FontList, FontMetrics, FontName, FontNames, FontSize, FontStretch, FontStyle, FontWeight,
        Hyphenation, HyphenationDataDir, HyphenationDataSource, Hyphens, Justify, LayoutDirections, LetterSpacing, LineBreak, LineHeight,
        LineSpacing, OutlineHintingOptions, OutlineSink, ParagraphSpacing, SegmentedText, SegmentedTextIter, ShapedColoredGlyphs,
        ShapedLine, ShapedSegment, ShapedText, TabLength, TextLineThickness, TextOverflowInfo, TextSegment, TextSegmentKind,
        TextShapingArgs, TextTransformFn, UnderlineThickness, WhiteSpace, WordBreak, WordSpacing, FONTS, FONT_CHANGED_EVENT,
    };
}

/// File system watcher service and types.
///
/// See [`zero_ui_ext_fs_watcher`] for the full watcher API.
pub mod fs_watcher {
    pub use zero_ui_ext_fs_watcher::{
        FsChange, FsChangeNote, FsChangeNoteHandle, FsChangesArgs, WatchFile, WatcherHandle, WatcherReadStatus, WatcherSyncStatus,
        WatcherSyncWriteNote, WriteFile, FS_CHANGES_EVENT, WATCHER,
    };
}

/// Images service, widget and types.
///
/// See [`zero_ui_ext_image`] for the full image API and [`zero_ui_wgt_image`] for the full widget API.
pub mod image {
    pub use zero_ui_ext_image::{
        render_retain, ImageCacheMode, ImageDataFormat, ImageDownscale, ImageHash, ImageHasher, ImageLimits, ImageMaskMode, ImagePpi,
        ImageRenderArgs, ImageSource, ImageSourceFilter, ImageVar, Img, PathFilter, IMAGES, IMAGE_RENDER,
    };

    #[cfg(http)]
    pub use zero_ui_ext_image::UriFilter;

    pub use zero_ui_wgt_image::{
        img_align, img_cache, img_crop, img_downscale, img_error_fn, img_fit, img_limits, img_loading_fn, img_offset, img_rendering,
        img_repeat, img_repeat_spacing, img_scale, img_scale_factor, img_scale_ppi, is_error, is_loaded, on_error, on_load, Image,
        ImageFit, ImageRepeat, ImgErrorArgs, ImgLoadArgs, ImgLoadingArgs,
    };

    /// Mask image properties.
    ///
    /// See [`zero_ui_wgt_image::mask`] for the full API.
    pub mod mask {
        pub use zero_ui_wgt_image::mask::{
            mask_align, mask_fit, mask_image, mask_image_cache, mask_image_downscale, mask_image_limits, mask_mode, mask_offset,
        };
    }
}

/// Accessibility service, events and properties.
///
/// See [`zero_ui_app::access`] and [`zero_ui_wgt_access`] for the full API.
pub mod access {
    pub use zero_ui_app::access::{
        AccessClickArgs, AccessExpanderArgs, AccessIncrementArgs, AccessInitedArgs, AccessNumberArgs, AccessScrollArgs,
        AccessSelectionArgs, AccessTextArgs, AccessToolTipArgs, ScrollCmd, ACCESS, ACCESS_CLICK_EVENT, ACCESS_EXPANDER_EVENT,
        ACCESS_INCREMENT_EVENT, ACCESS_INITED_EVENT, ACCESS_NUMBER_EVENT, ACCESS_SCROLL_EVENT, ACCESS_SELECTION_EVENT, ACCESS_TEXT_EVENT,
        ACCESS_TOOLTIP_EVENT,
    };
    pub use zero_ui_wgt_access::{
        access_commands, access_role, accessible, active_descendant, auto_complete, checked, col_count, col_index, col_span, controls,
        current, described_by, details, error_message, expanded, flows_to, invalid, item_count, item_index, label, labelled_by,
        labelled_by_child, level, live, modal, multi_selectable, on_access_click, on_access_expander, on_access_increment,
        on_access_number, on_access_scroll, on_access_selection, on_access_text, on_access_tooltip, on_pre_access_click,
        on_pre_access_expander, on_pre_access_increment, on_pre_access_number, on_pre_access_scroll, on_pre_access_selection,
        on_pre_access_text, on_pre_access_tooltip, orientation, owns, placeholder, popup, read_only, required, row_count, row_index,
        row_span, scroll_horizontal, scroll_vertical, selected, sort, value, value_max, value_min, AccessCmdName, AccessRole, AutoComplete,
        CurrentKind, Invalid, LiveIndicator, Orientation, Popup, SortDirection,
    };
}

/// Keyboard service, properties, events and types.
///
/// See [`zero_ui_ext_input::keyboard`] and [`zero_ui_wgt_input::keyboard`] for the full keyboard API.
pub mod keyboard {
    pub use zero_ui_app::shortcut::ModifiersState;

    pub use zero_ui_ext_input::keyboard::{
        HeadlessAppKeyboardExt, Key, KeyCode, KeyInputArgs, KeyRepeatConfig, KeyState, ModifiersChangedArgs, NativeKeyCode, KEYBOARD,
        KEY_INPUT_EVENT, MODIFIERS_CHANGED_EVENT,
    };

    pub use zero_ui_wgt_input::keyboard::{
        on_disabled_key_input, on_key_down, on_key_input, on_key_up, on_pre_disabled_key_input, on_pre_key_down, on_pre_key_input,
        on_pre_key_up,
    };
}

/// Mouse service, properties, events and types.
///
/// See [`zero_ui_ext_input::mouse`] and [`zero_ui_wgt_input::mouse`] for the full mouse API.
pub mod mouse {
    pub use zero_ui_ext_input::mouse::{
        ButtonRepeatConfig, ButtonState, ClickMode, ClickTrigger, MouseButton, MouseClickArgs, MouseHoverArgs, MouseInputArgs,
        MouseMoveArgs, MousePosition, MouseScrollDelta, MouseWheelArgs, MultiClickConfig, WidgetInfoBuilderMouseExt, WidgetInfoMouseExt,
        MOUSE, MOUSE_CLICK_EVENT, MOUSE_HOVERED_EVENT, MOUSE_INPUT_EVENT, MOUSE_MOVE_EVENT, MOUSE_WHEEL_EVENT,
    };

    pub use zero_ui_wgt_input::mouse::{
        on_disabled_mouse_any_click, on_disabled_mouse_click, on_disabled_mouse_hovered, on_disabled_mouse_input, on_disabled_mouse_wheel,
        on_mouse_any_click, on_mouse_any_double_click, on_mouse_any_single_click, on_mouse_any_triple_click, on_mouse_click,
        on_mouse_double_click, on_mouse_down, on_mouse_enter, on_mouse_hovered, on_mouse_input, on_mouse_leave, on_mouse_move,
        on_mouse_scroll, on_mouse_single_click, on_mouse_triple_click, on_mouse_up, on_mouse_wheel, on_mouse_zoom,
        on_pre_disabled_mouse_any_click, on_pre_disabled_mouse_click, on_pre_disabled_mouse_hovered, on_pre_disabled_mouse_input,
        on_pre_disabled_mouse_wheel, on_pre_mouse_any_click, on_pre_mouse_any_double_click, on_pre_mouse_any_single_click,
        on_pre_mouse_any_triple_click, on_pre_mouse_click, on_pre_mouse_double_click, on_pre_mouse_down, on_pre_mouse_enter,
        on_pre_mouse_hovered, on_pre_mouse_input, on_pre_mouse_leave, on_pre_mouse_move, on_pre_mouse_scroll, on_pre_mouse_single_click,
        on_pre_mouse_triple_click, on_pre_mouse_up, on_pre_mouse_wheel, on_pre_mouse_zoom,
    };

    pub use zero_ui_wgt_input::{click_mode, cursor, is_cap_mouse_pressed, is_mouse_pressed, CursorIcon};
}

/// Touch service, properties, events and types.
///
/// See [`zero_ui_ext_input::touch`] and [`zero_ui_wgt_input::touch`] for the full touch API.
pub mod touch {
    pub use zero_ui_ext_input::touch::{
        TouchConfig, TouchForce, TouchId, TouchInputArgs, TouchLongPressArgs, TouchMove, TouchMoveArgs, TouchPhase, TouchPosition,
        TouchTapArgs, TouchTransformArgs, TouchTransformInfo, TouchTransformMode, TouchUpdate, TouchedArgs, TOUCH, TOUCHED_EVENT,
        TOUCH_INPUT_EVENT, TOUCH_LONG_PRESS_EVENT, TOUCH_MOVE_EVENT, TOUCH_TAP_EVENT, TOUCH_TRANSFORM_EVENT,
    };

    pub use zero_ui_wgt_input::touch::{
        on_disabled_touch_input, on_disabled_touch_long_press, on_disabled_touch_tap, on_pre_disabled_touch_input,
        on_pre_disabled_touch_long_press, on_pre_disabled_touch_tap, on_pre_touch_cancel, on_pre_touch_end, on_pre_touch_enter,
        on_pre_touch_input, on_pre_touch_leave, on_pre_touch_long_press, on_pre_touch_move, on_pre_touch_start, on_pre_touch_tap,
        on_pre_touch_transform, on_pre_touched, on_touch_cancel, on_touch_end, on_touch_enter, on_touch_input, on_touch_leave,
        on_touch_long_press, on_touch_move, on_touch_start, on_touch_tap, on_touch_transform, on_touched,
    };

    pub use zero_ui_wgt_input::{is_cap_touched, is_touched, is_touched_from_start, touch_transform};
}

/// Touch service, properties, events and types.
///
/// See [`zero_ui_ext_input::focus`] and [`zero_ui_wgt_input::focus`] for the full focus API.
pub mod focus {
    pub use zero_ui_ext_input::focus::{
        commands, iter, DirectionalNav, FocusChangedArgs, FocusChangedCause, FocusInfo, FocusInfoBuilder, FocusInfoTree, FocusNavAction,
        FocusRequest, FocusScopeOnFocus, FocusTarget, ReturnFocusChangedArgs, TabIndex, TabNav, WidgetFocusInfo, WidgetInfoFocusExt, FOCUS,
        FOCUS_CHANGED_EVENT, RETURN_FOCUS_CHANGED_EVENT,
    };
    pub use zero_ui_wgt_input::focus::{
        alt_focus_scope, directional_nav, focus_click_behavior, focus_highlight, focus_on_init, focus_scope, focus_scope_behavior,
        focus_shortcut, focusable, is_focus_within, is_focus_within_hgl, is_focused, is_focused_hgl, is_return_focus,
        is_return_focus_within, on_blur, on_focus, on_focus_changed, on_focus_enter, on_focus_leave, on_pre_blur, on_pre_focus,
        on_pre_focus_changed, on_pre_focus_enter, on_pre_focus_leave, skip_directional, tab_index, tab_nav, FocusClickBehavior,
        FocusableMix,
    };
}

/// Pointer capture service, properties, events and types.
///
/// See [`zero_ui_ext_input::pointer_capture`] and [`zero_ui_wgt_input::pointer_capture`] for the full pointer capture API.
pub mod pointer_capture {
    pub use zero_ui_ext_input::pointer_capture::{CaptureInfo, CaptureMode, PointerCaptureArgs, POINTER_CAPTURE, POINTER_CAPTURE_EVENT};

    pub use zero_ui_wgt_input::pointer_capture::{
        capture_pointer, capture_pointer_on_init, on_got_pointer_capture, on_lost_pointer_capture, on_pointer_capture_changed,
        on_pre_got_pointer_capture, on_pre_lost_pointer_capture, on_pre_pointer_capture_changed,
    };
}

/// Gesture service, properties, events, shortcuts and other types.
///
/// See [`zero_ui_ext_input::gesture`] and [`zero_ui_wgt_input::gesture`] for the full gesture API
/// and [`zero_ui_app::shortcut`] for the shortcut API.
///
/// [`zero_ui_app::shortcut`]: mod@zero_ui_app::shortcut
pub mod gesture {
    pub use zero_ui_ext_input::gesture::{
        ClickArgs, ClickArgsSource, CommandShortcutMatchesExt, HeadlessAppGestureExt, ShortcutActions, ShortcutArgs, ShortcutClick,
        ShortcutsHandle, WeakShortcutsHandle, CLICK_EVENT, GESTURES, SHORTCUT_EVENT,
    };

    pub use zero_ui_app::shortcut::{
        shortcut, CommandShortcutExt, GestureKey, KeyChord, KeyGesture, ModifierGesture, Shortcut, ShortcutFilter, Shortcuts,
    };

    pub use zero_ui_wgt_input::gesture::{
        click_shortcut, context_click_shortcut, on_any_click, on_any_double_click, on_any_single_click, on_any_triple_click, on_click,
        on_context_click, on_disabled_click, on_double_click, on_pre_any_click, on_pre_any_double_click, on_pre_any_single_click,
        on_pre_any_triple_click, on_pre_click, on_pre_context_click, on_pre_disabled_click, on_pre_double_click, on_pre_single_click,
        on_pre_triple_click, on_single_click, on_triple_click,
    };

    pub use zero_ui_wgt_input::{is_cap_hovered, is_cap_pointer_pressed, is_cap_pressed, is_hovered, is_hovered_disabled, is_pressed};
}

/// Localization service, sources and types.
///
/// See [`zero_ui_ext_l10n`] for the full localization API.
pub mod l10n {
    pub use zero_ui_ext_l10n::{
        IntoL10nVar, L10nArgument, L10nDir, L10nMessageBuilder, L10nSource, Lang, LangMap, LangResource, LangResourceStatus, LangResources,
        Langs, NilL10nSource, SwapL10nSource, L10N, LANG_VAR,
    };
}

/// Undo service, commands and types.
///
/// See [`zero_ui_ext_undo`] for the full undo API.
pub mod undo {
    pub use zero_ui_ext_undo::{
        CommandUndoExt, RedoAction, UndoAction, UndoActionMergeArgs, UndoFullOp, UndoInfo, UndoOp, UndoSelect, UndoSelectInterval,
        UndoSelectLtEq, UndoSelector, UndoStackInfo, UndoTransaction, UndoVarModifyTag, WidgetInfoUndoExt, WidgetUndoScope,
        CLEAR_HISTORY_CMD, REDO_CMD, UNDO, UNDO_CMD, UNDO_INTERVAL_VAR, UNDO_LIMIT_VAR,
    };

    pub use zero_ui_wgt_undo::{undo_enabled, undo_interval, undo_limit, undo_scope, UndoMix};

    /// Undo history widget.
    ///
    /// See [`zero_ui_wgt_undo_history`] for the full undo API.
    pub mod history {
        pub use zero_ui_wgt_undo_history::{
            extend_undo_button_style, group_by_undo_interval, is_cap_hovered_timestamp, replace_undo_button_style, UndoEntryArgs,
            UndoHistory, UndoPanelArgs, UndoRedoButtonStyle, UndoStackArgs,
        };
    }
}

/// Data context types.
///
/// See [`zero_ui_wgt_data`] for the full API.
pub mod data_context {
    pub use zero_ui_wgt_data::{
        data, data_error, data_error_color, data_info, data_info_color, data_note, data_warn, data_warn_color, extend_data_note_colors,
        get_data_error, get_data_error_txt, get_data_info, get_data_info_txt, get_data_notes, get_data_notes_top, get_data_warn,
        get_data_warn_txt, has_data_error, has_data_info, has_data_notes, has_data_warn, replace_data_note_colors, with_data_note_color,
        DataNote, DataNoteHandle, DataNoteLevel, DataNoteValue, DataNotes, DATA, DATA_NOTE_COLORS_VAR,
    };
}

/// Window service, widget, events, commands and types.
///
/// See [`zero_ui_ext_window`], [`zero_ui_app::window`] and [`zero_ui_wgt_window`] for the full window API.
pub mod window {
    pub use zero_ui_app::window::{MonitorId, StaticMonitorId, StaticWindowId, WindowId, WindowMode, WINDOW};

    pub use zero_ui_ext_window::{
        AppRunWindowExt, AutoSize, CloseWindowResult, CursorImage, FocusIndicator, FrameCaptureMode, FrameImageReadyArgs,
        HeadlessAppWindowExt, HeadlessMonitor, ImeArgs, MonitorInfo, MonitorQuery, MonitorsChangedArgs, ParallelWin, RenderMode,
        RendererDebug, StartPosition, VideoMode, WINDOW_Ext, WidgetInfoBuilderImeArea, WidgetInfoImeArea, WindowChangedArgs, WindowChrome,
        WindowCloseArgs, WindowCloseRequestedArgs, WindowIcon, WindowLoadingHandle, WindowOpenArgs, WindowRoot, WindowRootExtenderArgs,
        WindowState, WindowStateAllowed, WindowVars, FRAME_IMAGE_READY_EVENT, IME_EVENT, MONITORS, MONITORS_CHANGED_EVENT, WINDOWS,
        WINDOW_CHANGED_EVENT, WINDOW_CLOSE_EVENT, WINDOW_CLOSE_REQUESTED_EVENT, WINDOW_LOAD_EVENT, WINDOW_OPEN_EVENT,
    };

    pub use zero_ui_view_api::webrender_api::DebugFlags;

    /// Window commands.
    pub mod commands {
        pub use zero_ui_ext_window::commands::*;

        #[cfg(inspector)]
        pub use zero_ui_wgt_inspector::INSPECT_CMD;
    }

    pub use zero_ui_wgt_window::{SaveState, Window};

    /// Native dialog types.
    pub mod native_dialog {
        pub use zero_ui_view_api::dialog::{
            FileDialog, FileDialogKind, FileDialogResponse, MsgDialog, MsgDialogButtons, MsgDialogIcon, MsgDialogResponse,
        };
    }
}

/// Debug inspection helpers.
pub mod inspector {
    pub use zero_ui_wgt_inspector::debug::{
        show_bounds, show_center_points, show_directional_query, show_hit_test, show_rows, InspectMode,
    };
}

/// Text widget, properties and types.
///
/// See [`zero_ui_wgt_text`] for the full widget API.
pub mod text {
    pub use zero_ui_txt::*;

    pub use zero_ui_wgt_text::{
        accepts_enter, accepts_tab, auto_selection, caret_color, caret_touch_shape, change_stop_delay, commands, direction, font_aa,
        font_annotation, font_caps, font_char_variant, font_cn_variant, font_color, font_common_lig, font_contextual_alt,
        font_discretionary_lig, font_ea_width, font_family, font_features, font_historical_forms, font_historical_lig, font_jp_variant,
        font_kerning, font_num_fraction, font_num_spacing, font_numeric, font_ornaments, font_palette, font_palette_colors, font_position,
        font_size, font_stretch, font_style, font_style_set, font_stylistic, font_swash, font_synthesis, font_variations, font_weight,
        get_caret_index, get_caret_status, get_chars_count, get_lines_len, get_lines_wrap_count, get_overflow, hyphen_char, hyphens,
        ime_underline, is_line_overflown, is_overflown, is_parse_pending, justify, lang, letter_spacing, line_break, line_height,
        line_spacing, max_chars_count, obscure_txt, obscuring_char, on_change_stop, overline, overline_color, paragraph_spacing,
        selection_color, selection_toolbar, selection_toolbar_anchor, selection_toolbar_fn, strikethrough, strikethrough_color, tab_length,
        txt_align, txt_editable, txt_highlight, txt_overflow, txt_overflow_align, underline, white_space, word_break, word_spacing,
        AutoSelection, CaretShape, CaretStatus, ChangeStopArgs, ChangeStopCause, Em, FontFeaturesMix, FontMix, LangMix, LinesWrapCount,
        ParagraphMix, SelectionToolbarArgs, Strong, Text, TextAlignMix, TextDecorationMix, TextEditMix, TextFillMix, TextOverflow,
        TextSpacingMix, TextTransformMix, TextWrapMix, TxtParseValue, UnderlinePosition, UnderlineSkip, FONT_COLOR_VAR,
    };
}

/// Icon widget and types.
///
/// See [`zero_ui_wgt_text::icon`] for the full widget API.
pub mod icon {
    pub use zero_ui_wgt_text::icon::{ico_color, ico_size, CommandIconExt, GlyphIcon, GlyphSource, Icon};

    #[cfg(feature = "material_icons")]
    pub use zero_ui_wgt_material_icons::{filled, outlined, rounded, sharp, MaterialFonts, MaterialIcon};
}

/// Container widget.
///
/// See [`zero_ui_wgt_container`] for the full widget API.
pub mod container {
    pub use zero_ui_wgt_container::{
        child_insert, child_insert_above, child_insert_below, child_insert_end, child_insert_left, child_insert_right, child_insert_start,
        child_out_insert, ChildInsertPlace, Container,
    };
}

/// Button widget, style and properties.
///
/// See [`zero_ui_wgt_button`] for the full widget API.
pub mod button {
    pub use zero_ui_wgt_button::{base_colors, extend_style, replace_style, Button, DefaultStyle};

    pub use zero_ui_wgt_link::LinkStyle;
}

/// ANSI text widget.
///
/// See [`zero_ui_wgt_ansi_text`] for the full widget API.
pub mod ansi_text {
    pub use zero_ui_wgt_ansi_text::{
        AnsiColor, AnsiStyle, AnsiText, AnsiTextParser, AnsiTxt, AnsiWeight, LineFnArgs, PageFnArgs, PanelFnArgs, TextFnArgs,
    };
}

/// Checkerboard visual widget.
///
/// See [`zero_ui_wgt_checkerboard`] for the full widget API.
pub mod checkerboard {
    pub use zero_ui_wgt_checkerboard::{cb_offset, cb_size, colors, node, Checkerboard};
}

/// Grid layout widgets.
///
/// See [`zero_ui_wgt_grid`] for the full widget API.
pub mod grid {
    pub use zero_ui_wgt_grid::{node, AutoGrowFnArgs, AutoGrowMode, Cell, Column, Grid, Row};

    /// Cell widget and properties.
    pub mod cell {
        pub use zero_ui_wgt_grid::cell::{at, column, column_span, row, row_span, span, Cell, CellInfo, AT_AUTO};
    }

    /// Column widget and properties.
    pub mod column {
        pub use zero_ui_wgt_grid::column::{
            get_index, get_index_fct, get_index_len, get_rev_index, is_even, is_first, is_last, is_odd, Column,
        };
    }

    /// Row widget and properties.
    pub mod row {
        pub use zero_ui_wgt_grid::row::{get_index, get_index_fct, get_index_len, get_rev_index, is_even, is_first, is_last, is_odd, Row};
    }
}

/// Window layers.
///
/// See [`zero_ui_wgt_layer`] for the full layers API.
pub mod layer {
    pub use zero_ui_wgt_layer::{adorner, adorner_fn, AnchorMode, AnchorOffset, AnchorSize, AnchorTransform, LayerIndex, LAYERS};
}

/// Popup widget and properties.
///
/// See [`zero_ui_wgt_layer::popup`] for the full widget API.
pub mod popup {
    pub use zero_ui_wgt_layer::popup::{
        anchor_mode, close_delay, close_on_focus_leave, context_capture, extend_style, is_close_delaying, on_popup_close_requested,
        on_pre_popup_close_requested, replace_style, ContextCapture, DefaultStyle, Popup, PopupCloseMode, PopupCloseRequestedArgs,
        PopupState, POPUP, POPUP_CLOSE_CMD, POPUP_CLOSE_REQUESTED_EVENT,
    };
}

/// Markdown widget, properties and types.
///
/// See [`zero_ui_wgt_markdown`] for the full widget API.
pub mod markdown {
    pub use zero_ui_wgt_markdown::{
        anchor, block_quote_fn, code_block_fn, code_inline_fn, footnote_def_fn, footnote_ref_fn, heading_anchor, heading_fn, image_fn,
        image_resolver, link_fn, link_resolver, link_scroll_mode, list_fn, list_item_bullet_fn, list_item_fn, on_link, on_pre_link,
        panel_fn, paragraph_fn, rule_fn, table_fn, text_fn, BlockQuoteFnArgs, CodeBlockFnArgs, CodeInlineFnArgs, FootnoteDefFnArgs,
        FootnoteRefFnArgs, HeadingFnArgs, HeadingLevel, ImageFnArgs, ImageResolver, LinkArgs, LinkFnArgs, LinkResolver, ListFnArgs,
        ListItemBulletFnArgs, ListItemFnArgs, Markdown, MarkdownStyle, PanelFnArgs, ParagraphFnArgs, RuleFnArgs, TableCellFnArgs,
        TableFnArgs, TextFnArgs, WidgetInfoExt, LINK_EVENT,
    };
}

/// Menu widgets, properties and types.
///
/// See [`zero_ui_wgt_menu`] for the full widget API.
pub mod menu {
    pub use zero_ui_wgt_menu::{
        extend_style, icon, icon_fn, panel_fn, replace_style, shortcut_spacing, shortcut_txt, ButtonStyle, CmdButton, DefaultStyle, Menu,
        ToggleStyle, TouchCmdButton,
    };

    /// Context menu widget and properties.
    ///
    /// See [`zero_ui_wgt_menu::context`] for the full widget API.
    pub mod context {
        pub use zero_ui_wgt_menu::context::{
            context_menu, context_menu_anchor, context_menu_fn, disabled_context_menu, disabled_context_menu_fn, extend_style, panel_fn,
            replace_style, ContextMenu, ContextMenuArgs, DefaultStyle, TouchStyle,
        };
    }

    /// Sub-menu popup widget and properties.
    ///
    /// See [`zero_ui_wgt_menu::popup`] for the full widget API.
    pub mod popup {
        pub use zero_ui_wgt_menu::popup::{extend_style, panel_fn, replace_style, DefaultStyle, SubMenuPopup};
    }
}

/// Panel layout widget.
///
/// See [`zero_ui_wgt_panel`] for the full widget API.
pub mod panel {
    pub use zero_ui_wgt_panel::{node, panel_fn, Panel, PanelArgs};
}

/// Rule line widgets and properties.
///
/// See [`zero_ui_wgt_rule_line`] for the full widget API.
pub mod rule_line {
    pub use zero_ui_wgt_rule_line::RuleLine;

    /// Horizontal rule line widget and properties.
    pub mod hr {
        pub use zero_ui_wgt_rule_line::hr::{color, line_style, margin, stroke_thickness, Hr};
    }

    /// Vertical rule line widget and properties.
    pub mod vr {
        pub use zero_ui_wgt_rule_line::vr::{color, line_style, margin, stroke_thickness, Vr};
    }
}

/// Scroll widgets, commands and properties.
///
/// See [`zero_ui_wgt_scroll`] for the full widget API.
pub mod scroll {
    pub use zero_ui_wgt_scroll::{
        alt_factor, auto_hide_extra, clip_to_viewport, define_viewport_unit, h_line_unit, h_page_unit, h_scrollbar_fn, h_wheel_unit, lazy,
        line_units, max_zoom, min_zoom, mode, mouse_pan, overscroll_color, page_units, scroll_to_focused_mode, scrollbar_fn,
        scrollbar_joiner_fn, smooth_scrolling, v_line_unit, v_page_unit, v_scrollbar_fn, v_wheel_unit, wheel_units, zoom_origin,
        zoom_touch_origin, zoom_wheel_origin, zoom_wheel_unit, LazyMode, Scroll, ScrollBarArgs, ScrollFrom, ScrollInfo, ScrollMode,
        ScrollUnitsMix, Scrollbar, ScrollbarFnMix, SmoothScrolling, Thumb, WidgetInfoExt, SCROLL,
    };

    /// Scrollbar thumb widget.
    pub mod thumb {
        pub use zero_ui_wgt_scroll::thumb::{cross_length, offset, viewport_ratio, Thumb};
    }

    /// Scroll widget.
    pub mod scrollbar {
        pub use zero_ui_wgt_scroll::scrollbar::{orientation, Orientation, Scrollbar, SCROLLBAR};
    }

    /// Scroll commands.
    pub mod commands {
        pub use zero_ui_wgt_scroll::commands::{
            scroll_to, scroll_to_zoom, ScrollRequest, ScrollToMode, ScrollToRequest, ScrollToTarget, PAGE_DOWN_CMD, PAGE_LEFT_CMD,
            PAGE_RIGHT_CMD, PAGE_UP_CMD, SCROLL_DOWN_CMD, SCROLL_LEFT_CMD, SCROLL_RIGHT_CMD, SCROLL_TO_BOTTOM_CMD, SCROLL_TO_CMD,
            SCROLL_TO_LEFTMOST_CMD, SCROLL_TO_RIGHTMOST_CMD, SCROLL_TO_TOP_CMD, SCROLL_UP_CMD, ZOOM_IN_CMD, ZOOM_OUT_CMD, ZOOM_RESET_CMD,
        };
    }
}

/// Stack layout widget, nodes and properties.
///
/// See [`zero_ui_wgt_stack`] for the full widget API.
pub mod stack {
    pub use zero_ui_wgt_stack::{
        get_index, get_index_fct, get_index_len, get_rev_index, h_stack, is_even, is_first, is_last, is_odd, lazy_sample, lazy_size, node,
        stack_nodes, stack_nodes_layout_by, v_stack, z_stack, Stack, StackDirection, WidgetInfoStackExt,
    };
}

/// Text input widget and properties.
///
/// See [`zero_ui_wgt_text_input`] for the full widget API.
pub mod text_input {
    pub use zero_ui_wgt_text_input::{
        base_colors, data_notes_adorner_fn, extend_style, field_help, max_chars_count_adorner_fn, replace_style, DefaultStyle, FieldStyle,
        TextInput,
    };
}

/// Label widget and properties.
///
/// See [`zero_ui_wgt_text_input::label`] for the full widget API.
pub mod label {
    pub use zero_ui_wgt_text_input::label::{extend_style, replace_style, DefaultStyle, Label};
}

/// Toggle button widget and styles for check box, combo box, radio button and switch button.
///
/// See [`zero_ui_wgt_toggle`] for the full widget API.
pub mod toggle {
    pub use zero_ui_wgt_toggle::{
        check_spacing, combo_spacing, deselect_on_deinit, deselect_on_new, extend_style, is_checked, radio_spacing, replace_style,
        scroll_on_select, select_on_init, select_on_new, selector, switch_spacing, tristate, CheckStyle, ComboStyle, DefaultStyle,
        RadioStyle, Selector, SelectorError, SelectorImpl, SwitchStyle, Toggle, IS_CHECKED_VAR,
    };

    /// Toggle commands.
    pub mod commands {
        pub use zero_ui_wgt_toggle::commands::{SelectOp, SELECT_CMD, TOGGLE_CMD};
    }
}

/// Tooltip properties and widget.
///
/// See [`zero_ui_wgt_tooltip`] for the full tooltip API.
pub mod tip {
    pub use zero_ui_wgt_tooltip::{
        access_tooltip_anchor, access_tooltip_duration, base_colors, disabled_tooltip, disabled_tooltip_fn, extend_style, replace_style,
        tooltip, tooltip_anchor, tooltip_context_capture, tooltip_delay, tooltip_duration, tooltip_fn, tooltip_interval, DefaultStyle, Tip,
        TooltipArgs,
    };
}

/// View widgets and nodes.
///
/// See [`zero_ui_wgt_view`] for the full view API.
pub mod view {
    pub use zero_ui_wgt_view::{View, ViewArgs};
}

/// Switch widget and node.
///
/// See [`zero_ui_wgt_switch`] for the full widget API.
pub mod switch {
    pub use zero_ui_wgt_switch::{switch_node, Switch};
}

/// Wrap layout widget and properties.
///
/// See [`zero_ui_wgt_wrap`] for the full view API.
pub mod wrap {
    pub use zero_ui_wgt_wrap::{
        get_index, get_index_fct, get_index_len, get_rev_index, is_even, is_first, is_last, is_odd, lazy_sample, lazy_size, node,
        WidgetInfoWrapExt, Wrap,
    };
}

/// Style mix-in and types.
pub mod style {
    pub use zero_ui_wgt_style::{style_fn, with_style_extension, Style, StyleArgs, StyleBuilder, StyleFn, StyleMix};
}

/// Start and manage an app process.
///
/// # View Process
///
/// A view-process must be initialized before starting an app. Panics on `run` if there is
/// no view-process, also panics if the current process is already executing as a view-process.
pub struct APP;
impl std::ops::Deref for APP {
    type Target = zero_ui_app::APP;

    fn deref(&self) -> &Self::Target {
        &zero_ui_app::APP
    }
}

mod defaults {
    use zero_ui_app::{AppExtended, AppExtension, AppExtensionBoxed};
    use zero_ui_ext_clipboard::ClipboardManager;
    use zero_ui_ext_config::ConfigManager;
    use zero_ui_ext_font::FontManager;
    use zero_ui_ext_fs_watcher::FsWatcherManager;
    use zero_ui_ext_image::ImageManager;
    use zero_ui_ext_input::{
        focus::FocusManager, gesture::GestureManager, keyboard::KeyboardManager, mouse::MouseManager,
        pointer_capture::PointerCaptureManager, touch::TouchManager,
    };
    use zero_ui_ext_l10n::L10nManager;
    use zero_ui_ext_undo::UndoManager;
    use zero_ui_ext_window::WindowManager;

    #[cfg(dyn_app_extension)]
    macro_rules! DefaultsAppExtended {
        () => {
            AppExtended<Vec<Box<dyn AppExtensionBoxed>>>
        }
    }
    #[cfg(not(dyn_app_extension))]
    macro_rules! DefaultsAppExtended {
        () => {
            AppExtended<impl AppExtension>
        }
    }

    impl super::APP {
        /// App with default extensions.
        ///     
        /// # Extensions
        ///
        /// Extensions included.
        ///
        /// * [`FsWatcherManager`]
        /// * [`ConfigManager`]
        /// * [`L10nManager`]
        /// * [`PointerCaptureManager`]
        /// * [`MouseManager`]
        /// * [`TouchManager`]
        /// * [`KeyboardManager`]
        /// * [`GestureManager`]
        /// * [`WindowManager`]
        /// * [`FontManager`]
        /// * [`FocusManager`]
        /// * [`ImageManager`]
        /// * [`ClipboardManager`]
        /// * [`UndoManager`]
        /// * [`MaterialFonts`] if `cfg(feature = "material_icons")`.
        ///
        /// [`MaterialFonts`]: zero_ui_wgt_material_icons::MaterialFonts
        pub fn defaults(&self) -> DefaultsAppExtended![] {
            let r = self
                .minimal()
                .extend(FsWatcherManager::default())
                .extend(ConfigManager::default())
                .extend(L10nManager::default())
                .extend(PointerCaptureManager::default())
                .extend(MouseManager::default())
                .extend(TouchManager::default())
                .extend(KeyboardManager::default())
                .extend(GestureManager::default())
                .extend(WindowManager::default())
                .extend(FontManager::default())
                .extend(FocusManager::default())
                .extend(ImageManager::default())
                .extend(ClipboardManager::default())
                .extend(UndoManager::default());

            #[cfg(feature = "material_icons")]
            let r = r.extend(zero_ui_wgt_material_icons::MaterialFonts);

            r.extend(DefaultsInit {})
        }
    }

    struct DefaultsInit {}
    impl AppExtension for DefaultsInit {
        fn init(&mut self) {
            use zero_ui_app::widget::instance::ui_vec;
            use zero_ui_ext_clipboard::COPY_CMD;
            use zero_ui_ext_window::WINDOWS;
            use zero_ui_wgt::wgt_fn;
            use zero_ui_wgt_text::icon::CommandIconExt as _;
            use zero_ui_wgt_text::{commands::SELECT_ALL_CMD, icon::Icon, SelectionToolbarArgs};

            WINDOWS.register_root_extender(|a| {
                let child = a.root;

                #[cfg(inspector)]
                let child = zero_ui_wgt_inspector::inspector(child, zero_ui_wgt_inspector::live_inspector(true));

                // setup COLOR_SCHEME_VAR for all windows, this is not done in `Window!` because
                // WindowRoot is used directly by some headless renderers.
                let child = zero_ui_wgt::nodes::with_context_var_init(child, zero_ui_color::COLOR_SCHEME_VAR, || {
                    use zero_ui_ext_window::WINDOW_Ext as _;
                    use zero_ui_var::Var as _;

                    zero_ui_app::window::WINDOW.vars().actual_color_scheme().boxed()
                });

                // `zero_ui_wgt_menu` depends on `zero_ui_wgt_text` so we can't set this in the text crate.
                zero_ui_wgt_text::selection_toolbar_fn(
                    child,
                    wgt_fn!(|args: SelectionToolbarArgs| {
                        use zero_ui_wgt_menu as menu;
                        menu::context::ContextMenu! {
                            style_fn = menu::context::TouchStyle!();
                            children = ui_vec![
                                menu::TouchCmdButton!(COPY_CMD.scoped(args.anchor_id)),
                                menu::TouchCmdButton!(SELECT_ALL_CMD.scoped(args.anchor_id)),
                            ];
                        }
                    }),
                )
            });

            #[cfg(feature = "material_icons")]
            {
                use zero_ui_ext_clipboard::*;
                use zero_ui_ext_undo::*;
                use zero_ui_ext_window::commands::*;
                use zero_ui_wgt_input::commands::*;
                use zero_ui_wgt_material_icons::outlined as icons;
                use zero_ui_wgt_scroll::commands::*;

                CUT_CMD.init_icon(wgt_fn!(|_| Icon!(icons::CUT)));
                COPY_CMD.init_icon(wgt_fn!(|_| Icon!(icons::COPY)));
                PASTE_CMD.init_icon(wgt_fn!(|_| Icon!(icons::PASTE)));

                UNDO_CMD.init_icon(wgt_fn!(|_| Icon!(icons::UNDO)));
                REDO_CMD.init_icon(wgt_fn!(|_| Icon!(icons::REDO)));

                CLOSE_CMD.init_icon(wgt_fn!(|_| Icon!(icons::CLOSE)));
                MINIMIZE_CMD.init_icon(wgt_fn!(|_| Icon!(icons::MINIMIZE)));
                MAXIMIZE_CMD.init_icon(wgt_fn!(|_| Icon!(icons::MAXIMIZE)));
                FULLSCREEN_CMD.init_icon(wgt_fn!(|_| Icon!(icons::FULLSCREEN)));

                CONTEXT_MENU_CMD.init_icon(wgt_fn!(|_| Icon!(icons::MENU_OPEN)));

                #[cfg(feature = "inspector")]
                zero_ui_wgt_inspector::INSPECT_CMD.init_icon(wgt_fn!(|_| Icon!(icons::SCREEN_SEARCH_DESKTOP)));

                SCROLL_TO_TOP_CMD.init_icon(wgt_fn!(|_| Icon!(icons::VERTICAL_ALIGN_TOP)));
                SCROLL_TO_BOTTOM_CMD.init_icon(wgt_fn!(|_| Icon!(icons::VERTICAL_ALIGN_BOTTOM)));

                ZOOM_IN_CMD.init_icon(wgt_fn!(|_| Icon!(icons::ZOOM_IN)));
                ZOOM_OUT_CMD.init_icon(wgt_fn!(|_| Icon!(icons::ZOOM_OUT)));
            }
        }
    }
}
