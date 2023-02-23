use std::{mem, time::Duration};

use zero_ui_view_api::AnimationsConfig;

use crate::{
    app::{AppEventSender, LoopTimer},
    context::Updates,
    crate_util,
    units::Factor,
};

use super::{
    animation::{Animations, ModifyInfo},
    *,
};

/// Represents the last time a variable was mutated or the current update cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VarUpdateId(u32);
impl VarUpdateId {
    /// ID that is never new.
    pub const fn never() -> Self {
        VarUpdateId(0)
    }

    fn next(&mut self) {
        if self.0 == u32::MAX {
            self.0 = 1;
        } else {
            self.0 += 1;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct VarApplyUpdateId(u32);
impl VarApplyUpdateId {
    /// ID that is never returned in `Vars`.
    pub(super) const fn initial() -> Self {
        VarApplyUpdateId(0)
    }

    fn next(&mut self) {
        if self.0 == u32::MAX {
            self.0 = 1;
        } else {
            self.0 += 1;
        }
    }
}

pub(super) type VarUpdateFn = Box<dyn FnOnce(&Vars, &mut Updates)>;

/// Enables write access for [`Var<T>`].
pub struct Vars {
    app_event_sender: AppEventSender,
    pub(super) ans: Animations,

    update_id: VarUpdateId,
    apply_update_id: VarApplyUpdateId,

    updates: RefCell<Vec<(ModifyInfo, VarUpdateFn)>>,
    spare_updates: Vec<(ModifyInfo, VarUpdateFn)>,

    modify_receivers: RefCell<Vec<Box<dyn Fn(&Vars) -> bool>>>,
}
impl Vars {
    /// Id of the current vars update in the app scope.
    ///
    /// Variable with [`AnyVar::last_update`] equal to this are *new*.
    pub fn update_id(&self) -> VarUpdateId {
        self.update_id
    }

    /// Returns a read-only variable that tracks if animations are enabled in the operating system.
    ///
    /// If `false` all animations must be skipped to the end, users with photo-sensitive epilepsy disable animations system wide.
    pub fn animations_enabled(&self) -> ReadOnlyArcVar<bool> {
        self.ans.animations_enabled.read_only()
    }

    /// Variable that defines the global frame duration, the default is 60fps `(1.0 / 60.0).secs()`.
    pub fn frame_duration(&self) -> &ArcVar<Duration> {
        &self.ans.frame_duration
    }

    /// Variable that defines a global scale for the elapsed time of animations.
    pub fn animation_time_scale(&self) -> &ArcVar<Factor> {
        &self.ans.animation_time_scale
    }

    /// Info about the current context when requesting variable modification.
    ///
    /// If is current inside a [`Vars::animate`] closure, or inside a [`Var::modify`] closure requested by an animation, or inside
    /// an [`AnimationController`], returns the info that was collected at the moment the animation was requested. Outside of animations
    /// gets an info with [`importance`] guaranteed to override the [`modify_importance`].
    ///
    /// [`importance`]: ModifyInfo::importance
    /// [`modify_importance`]: AnyVar::modify_importance
    /// [`AnimationController`]: animation::AnimationController
    pub fn current_modify(&self) -> ModifyInfo {
        self.ans.current_modify.borrow().clone()
    }

    /// Adds an animation handler that is called every frame to update captured variables.
    ///
    /// This is used to implement all [`Var<T>`] animations, it enables any kind of variable animation,
    /// including multiple variables.
    ///
    /// Returns an [`AnimationHandle`] that can be used to monitor the animation status and to [`stop`] or to
    /// make the animation [`perm`].
    ///
    /// # Variable Control
    ///
    /// Animations assume *control* of a variable on the first time they cause its value to be new, after this
    /// moment the [`AnyVar::is_animating`] value is `true` and [`AnyVar::modify_importance`] is the animation's importance,
    /// until the animation stops. Only one animation can control a variable at a time, if an animation loses control of a
    /// variable all attempts to modify it from inside the animation are ignored.
    ///
    /// Later started animations steal control from previous animations, direct touch, modify or set calls also remove the variable
    /// from being affected by a running animation.
    ///
    /// # Nested Animations
    ///
    /// Other animations can be started from inside the animation closure, these *nested* animations have the same importance
    /// as the *parent* animation, the animation handle is different and [`AnyVar::is_animating`] is `false` if the nested animation
    /// is dropped before the *parent* animation. But because the animations share the same importance the parent animation can
    /// set the variable again.
    ///
    /// # Examples
    ///
    /// The example animates a `text` variable from `"Animation at 0%"` to `"Animation at 100%"`, when the animation
    /// stops the `completed` variable is set to `true`.
    ///
    /// ```
    /// # use zero_ui_core::{var::*, *, units::*, text::*, handler::*};
    /// #
    /// fn animate_text(text: &impl Var<Text>, completed: &impl Var<bool>, vars: &Vars) {
    ///     let transition = animation::Transition::new(0u8, 100);
    ///     let mut prev_value = 101;
    ///     vars.animate(clone_move!(text, completed, |vars, animation| {
    ///         let step = easing::expo(animation.elapsed_stop(1.secs()));
    ///         let value = transition.sample(step);
    ///         if value != prev_value {
    ///             if value == 100 {
    ///                 animation.stop();
    ///                 completed.set(vars, true);
    ///             }
    ///             let _ = text.set(vars, formatx!("Animation at {value}%"));
    ///             prev_value = value;
    ///         }
    ///     }))
    ///     .perm()
    /// }
    /// ```
    ///
    /// Note that the animation can be stopped from the inside, the closure second parameter is an [`Animation`]. In
    /// the example this is the only way to stop the animation, because we called [`perm`]. Animations hold a clone
    /// of the variables they affect and exist for the duration of the app if not stopped, causing the app to wake and call the
    /// animation closure for every frame.
    ///
    /// This method is the most basic animation interface, used to build all other animations and *easing*, its rare that you
    /// will need to use it directly, most of the time animation effects can be composted using the [`Var`] easing and mapping
    /// methods.
    ///
    /// ```
    /// # use zero_ui_core::{var::*, *, units::*, text::*, handler::*};
    /// # fn demo(vars: &Vars) {
    /// let value = var(0u8);
    /// let text = value.map(|v| formatx!("Animation at {v}%"));
    /// value.ease_ne(vars, 100, 1.secs(), easing::expo);
    /// # }
    /// ```
    ///
    /// # Optimization Tips
    ///
    /// When no animation is running the app *sleeps* awaiting for an external event, update request or timer elapse, when at least one
    /// animation is running the app awakes every [`Vars::frame_duration`]. You can use [`Animation::sleep`] to *pause* the animation
    /// for a duration, if all animations are sleeping the app is also sleeping.
    ///
    /// Animations have their control over a variable permanently overridden when a newer animation modifies it or
    /// it is modified directly, but even if overridden **the animation keeps running**. This happens because the system has no insight of
    /// all side effects caused by the `animation` closure. You can use the [`Vars::current_modify`] and [`AnyVar::modify_importance`]
    /// to detect when the animation no longer affects any variables and stop it.
    ///
    /// These optimizations are implemented by the animations provided as methods of [`Var<T>`].
    ///
    /// # External Controller
    ///
    /// The animation can be controlled from the inside using the [`Animation`] reference, it can be stopped using the returned
    /// [`AnimationHandle`], and it can also be controlled by a registered [`AnimationController`] that can manage multiple
    /// animations at the same time, see [`with_animation_controller`] for more details.
    ///
    /// [`AnimationHandle`]: animation::AnimationHandle
    /// [`AnimationController`]: animation::AnimationController
    /// [`Animation`]: animation::Animation
    /// [`Animation::sleep`]: animation::Animation::sleep
    /// [`stop`]: animation::AnimationHandle::stop
    /// [`perm`]: animation::AnimationHandle::perm
    /// [`with_animation_controller`]: Self::with_animation_controller
    pub fn animate<A>(&self, animation: A) -> animation::AnimationHandle
    where
        A: FnMut(&Vars, &animation::Animation) + 'static,
    {
        Animations::animate(self, animation)
    }

    /// Calls `animate` while `controller` is registered as the animation controller.
    ///
    /// The `controller` is notified of animation events for each animation spawned by `animate` and can affect then with the same
    /// level of access as [`Vars::animate`]. Only one controller can affect animations at a time.
    ///
    /// This can be used to manage multiple animations at the same time, or to get [`Vars::animate`] level of access to an animation
    /// that is not implemented to allow such access. Note that animation implementers are not required to support the full
    /// [`Animation`] API, for example, there is no guarantee that a restart requested by the controller will repeat the same animation.
    ///
    /// The controller can start new animations, these animations will have the same controller if not overridden, you can
    /// use this method and the [`NilAnimationObserver`] to avoid this behavior.
    ///
    /// [`Animation`]: animation::Animation
    /// [`NilAnimationObserver`]: animation::NilAnimationObserver
    pub fn with_animation_controller<R>(&self, controller: impl animation::AnimationController, animate: impl FnOnce() -> R) -> R {
        Animations::with_animation_controller(self, controller, animate)
    }

    pub(crate) fn instance(app_event_sender: AppEventSender) -> Vars {
        Vars {
            app_event_sender,
            ans: Animations::new(),
            update_id: VarUpdateId(1),
            apply_update_id: VarApplyUpdateId(1),
            updates: RefCell::new(Vec::with_capacity(128)),
            spare_updates: Vec::with_capacity(128),
            modify_receivers: RefCell::new(vec![]),
        }
    }

    pub(super) fn schedule_update(&self, update: VarUpdateFn) {
        let curr_modify = self.current_modify();
        self.updates.borrow_mut().push((curr_modify, update));
    }

    /// Id of each `schedule_update` cycle during `apply_updates`
    pub(super) fn apply_update_id(&self) -> VarApplyUpdateId {
        self.apply_update_id
    }

    pub(crate) fn apply_updates(&mut self, updates: &mut Updates) {
        debug_assert!(self.spare_updates.is_empty());

        self.update_id.next();
        self.ans.animation_start_time.set(None);

        // if has pending updates, apply all,
        // var updates can generate other updates (bindings), these are applied in the same
        // app update, hence the loop and "spare" vec alloc.
        while !self.updates.get_mut().is_empty() {
            let mut var_updates = mem::replace(self.updates.get_mut(), mem::take(&mut self.spare_updates));

            for (animation_info, update) in var_updates.drain(..) {
                // load animation priority that was current when the update was requested.
                let prev_info = mem::replace(&mut *self.ans.current_modify.borrow_mut(), animation_info);
                let _cleanup = crate_util::RunOnDrop::new(|| *self.ans.current_modify.borrow_mut() = prev_info);

                // apply.
                update(self, updates);
            }
            self.spare_updates = var_updates;

            self.apply_update_id.next();
        }
    }

    pub(crate) fn register_channel_recv(&self, recv_modify: Box<dyn Fn(&Vars) -> bool>) {
        self.modify_receivers.borrow_mut().push(recv_modify);
    }

    pub(crate) fn app_event_sender(&self) -> AppEventSender {
        self.app_event_sender.clone()
    }

    pub(crate) fn receive_sended_modify(&self) {
        let mut rcvs = mem::take(&mut *self.modify_receivers.borrow_mut());
        rcvs.retain(|rcv| rcv(self));

        let mut rcvs_mut = self.modify_receivers.borrow_mut();
        rcvs.extend(rcvs_mut.drain(..));
        *rcvs_mut = rcvs;
    }

    pub(crate) fn update_animations_config(&self, cfg: &AnimationsConfig) {
        self.ans.animations_enabled.set_ne(self, cfg.enabled);
    }

    /// Called in `update_timers`, does one animation frame if the frame duration has elapsed.
    pub(crate) fn update_animations(&mut self, timer: &mut LoopTimer) {
        Animations::update_animations(self, timer)
    }

    /// Returns the next animation frame, if there are any active animations.
    pub(crate) fn next_deadline(&mut self, timer: &mut LoopTimer) {
        Animations::next_deadline(self, timer)
    }

    pub(crate) fn has_pending_updates(&mut self) -> bool {
        !self.updates.get_mut().is_empty()
    }
}

/// Represents temporary access to [`Vars`].
///
/// All contexts that provide [`Vars`] implement this trait to facilitate access to it.
pub trait WithVars {
    /// Visit the [`Vars`] reference.
    fn with_vars<R, F: FnOnce(&Vars) -> R>(&self, visit: F) -> R;
}
impl WithVars for Vars {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(self)
    }
}

impl<'a> WithVars for crate::context::AppContext<'a> {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(self.vars)
    }
}
impl<'a> WithVars for crate::context::WindowContext<'a> {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(self.vars)
    }
}
impl<'a> WithVars for crate::context::WidgetContext<'a> {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(self.vars)
    }
}
impl<'a> WithVars for crate::context::LayoutContext<'a> {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(self.vars)
    }
}
impl WithVars for crate::context::AppContextMut {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        self.with(move |ctx| action(ctx.vars))
    }
}
impl WithVars for crate::context::WidgetContextMut {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        self.with(move |ctx| action(ctx.vars))
    }
}
#[cfg(any(test, doc, feature = "test_util"))]
impl WithVars for crate::context::TestWidgetContext {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(&self.vars)
    }
}
impl WithVars for crate::app::HeadlessApp {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(self.vars())
    }
}
