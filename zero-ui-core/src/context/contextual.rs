use std::{
    ptr,
    sync::{
        atomic::{AtomicPtr, Ordering},
        Arc,
    },
};

use parking_lot::Mutex;

use crate::{crate_util::RunOnDrop, task, widget_instance::WidgetId, window::WindowId};

use super::WidgetContext;

macro_rules! contextual_ctx {
    ($($Context:ident),+ $(,)?) => {$(paste::paste! {

#[doc = " Represents a *contextual* reference to [`"$Context "`]."]
///
#[doc = "This type exist to provide access to a [`"$Context "`] inside [Ui bound](crate::task::ui) futures."]
#[doc = "Every time the task updates the executor loads a exclusive reference to the context using the paired [`"$Context "Scope`]"]
/// to provide the context for that update. Inside the future you can then call [`with`](Self::with) to get the exclusive
/// reference to the context.
pub struct [<$Context Mut>] {
    ctx: Arc<Mutex<[<$Context ScopeData>]>>,
}
impl Clone for [<$Context Mut>] {
    fn clone(&self) -> Self {
        Self {
            ctx: Arc::clone(&self.ctx)
        }
    }
}
impl [<$Context Mut>] {
    #[doc = "Runs an action with the *contextual* exclusive borrow to a [`"$Context "`]."]
    ///
    /// ## Panics
    ///
    /// Panics if `with` is called again inside `action`, also panics if not called inside the paired
    #[doc = "[`"$Context "Scope::with`]. You should assume that if you have access to a [`"$Context "Mut`] it is in a valid"]
    /// state, the onus of safety is on the caller.
    pub fn with<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&mut $Context) -> R,
    {
        let ctx = self.ctx.try_lock().unwrap_or_else(|| {
            panic!("already in `{0}Mut::with`, cannot borrow `&mut {0}` twice", stringify!($Context));
        });

        let ptr = ctx.ptr.load(Ordering::Relaxed);

        if ptr.is_null() {
            panic!("no `&mut {0}` loaded for `{0}Mut`", stringify!($Context));
        }

        let ctx = unsafe { &mut *(ptr as *mut $Context) };
        action(ctx)
    }
}

#[doc = "Pair of [`"$Context "Mut`] that can setup its reference."]
pub struct [<$Context Scope>] {
    ctx: Arc<Mutex<[<$Context ScopeData>]>>,
}
struct [<$Context ScopeData>] {
    ptr: AtomicPtr<()>,
}
impl [<$Context Scope>] {
    #[doc = "Create a new [`"$Context "Scope`], [`"$Context "Mut`] pair."]
    pub fn new() -> (Self, [<$Context Mut>]) {
        let ctx = Arc::new(Mutex::new([<$Context ScopeData>] {
            ptr: AtomicPtr::new(ptr::null_mut()),
        }));

        (Self { ctx: Arc::clone(&ctx) }, [<$Context Mut>] { ctx })
    }

    #[doc = "Runs `action` while the paired [`"$Context "Mut`] points to `ctx`."]
    pub fn with<R, F>(&self, ctx: &mut $Context, action: F) -> R
    where
        F: FnOnce() -> R,
    {
        let prev = self.ctx.lock().ptr.swap(ctx as *mut $Context as *mut (), Ordering::Relaxed);
        let _r = RunOnDrop::new(|| {
            self.ctx.lock().ptr.store(prev, Ordering::Relaxed);
        });
        action()
    }
}

    })+};
}
contextual_ctx!(WidgetContext);

impl WidgetContextMut {
    /// Yield for one update.
    ///
    /// Async event handlers run in widget updates, the code each `.await` runs in a different update, but only if
    /// the `.await` does not return immediately. This future always awaits once for each new update, so the
    /// code after awaiting is guaranteed to run in a different update.
    ///
    /// Note that this does not cause an immediate update, if no update was requested it will *wait* until one is.
    /// To force an update and then yield use [`update`](Self::update) instead.
    ///
    /// You can reuse this future but it is very cheap to just make a new one.
    ///
    /// ```
    /// # use zero_ui_core::{context::*, handler::*};
    /// # TestWidgetContext::doc_test((),
    /// async_hn!(|ctx, _| {
    ///     println!("First update");
    ///     ctx.yield_one().await;
    ///     println!("Second update");
    /// })
    /// # );
    /// ```
    pub async fn yield_one(&self) {
        task::yield_one().await
    }

    /// Requests one update and returns a future that *yields* one update.
    ///
    /// This is like [`yield_one`](Self::yield_one) but also requests the next update, causing the code after
    /// the `.await` to run immediately after one update is processed.
    ///
    /// ```
    /// # use zero_ui_core::context::*;
    /// # use zero_ui_core::handler::*;
    /// # use zero_ui_core::var::*;
    /// # TestWidgetContext::doc_test((),
    /// async_hn!(|ctx, _| {
    ///     let foo_var = var(false);
    ///     // variable assign will cause an update.
    ///     foo_var.set(true);
    ///
    ///     ctx.yield_one().await;// wait next update.
    ///
    ///     // we are in the next update now, the variable value is new.
    ///     assert_eq!(Some(true), foo_var.get_new());
    ///
    ///     ctx.update().await;// force next update and wait.
    ///
    ///     // we are in the requested update, variable value is no longer new.
    ///     assert_eq!(None, foo_var.get_new());
    /// })
    /// # );
    /// ```
    ///
    /// In the example above, the variable assign causes an app update so `yield_one` processes it immediately,
    /// but the second `.await` needs to cause an update if we don't want to depend on another part of the app
    /// to awake.
    pub async fn update(&self) {
        todo!("remove");
        // self.with(|c| c.updates.update(c.path.widget_id()));
        self.yield_one().await
    }

    /// Id of the window that owns the context widget.
    pub fn window_id(&self) -> WindowId {
        self.with(|ctx| ctx.path.window_id())
    }

    /// Id of the context widget.
    pub fn widget_id(&self) -> WidgetId {
        self.with(|ctx| ctx.path.widget_id())
    }
}
