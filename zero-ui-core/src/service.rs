//! Services API.

use std::{any::*, cell::Cell, fmt, ptr, rc::Rc, thread::LocalKey};

/// Auto implement [`Service`](type@Service) trait and generates an extension method for requiring the service.
pub use zero_ui_proc_macros::Service;

/// Error when an service of the same type is registered twice.
///
/// The associated value is the instance that could not be registered.
pub struct AlreadyRegistered<S: Service>(pub S);
impl<S: Service> fmt::Debug for AlreadyRegistered<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AlreadyRegistered<{}>", type_name::<S>())
    }
}
impl<S: Service> fmt::Display for AlreadyRegistered<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` is already registered", type_name::<S>())
    }
}
impl<S: Service> std::error::Error for AlreadyRegistered<S> {}

struct ServiceInstanceEntry {
    _instance: Box<dyn Service>,
    deiniter: Box<dyn Fn()>,
}
impl Drop for ServiceInstanceEntry {
    fn drop(&mut self) {
        (self.deiniter)();
    }
}

/// Access to application services.
///
/// An instance of this struct is available in [`AppContext`] and derived contexts.
///
/// [`AppContext`]: crate::context::AppContext::services
pub struct Services {
    services: Vec<ServiceInstanceEntry>,
}
impl Services {
    pub(crate) fn default() -> Self {
        Services {
            services: Vec::with_capacity(20),
        }
    }

    /// Register a new service for the duration of the application context.
    pub fn try_register<S: Service + Sized>(&mut self, service: S) -> Result<(), AlreadyRegistered<S>> {
        let mut service = Box::new(service);
        let prev = S::thread_local_entry().init(service.as_mut() as _);
        if prev.is_null() {
            let deiniter = Box::new(|| S::thread_local_entry().deinit());
            self.services.push(ServiceInstanceEntry {
                _instance: service,
                deiniter,
            });
            Ok(())
        } else {
            S::thread_local_entry().init(prev);
            Err(AlreadyRegistered(*service))
        }
    }

    /// Register a new service for the duration of the application context.
    ///
    /// # Panics
    ///
    /// Panics if another instance of the service is already registered.
    #[track_caller]
    pub fn register<S: Service + Sized>(&mut self, service: S) {
        self.try_register(service).expect("service already registered")
    }

    /// Gets a service reference if the service is registered in the application.
    #[inline]
    pub fn get<S: Service>(&mut self) -> Option<&mut S> {
        let ptr = S::thread_local_entry().get();
        if ptr.is_null() {
            None
        } else {
            // SAFETY: This is safe as long as only Services calls thread_local_entry
            // with a &mut self reference.
            Some(unsafe { &mut *ptr })
        }
    }

    /// Requires a service reference.
    ///
    /// # Extension Methods
    ///
    /// Every service implemented using `derive` has a `ServiceNameExt` trait that implements a method for [`Services`]
    /// that requires the service. So instead of using this method to request `services.req::<FooBar>()` you can use
    /// `services.foo_bar()` if you have imported `FooBarExt`.
    ///
    /// # Panics
    ///
    /// If  the service is not registered in the application.
    #[inline]
    #[track_caller]
    pub fn req<S: Service>(&mut self) -> &mut S {
        self.get::<S>()
            .unwrap_or_else(|| panic!("app service `{}` is required", type_name::<S>()))
    }

    /// Gets multiple service references if all services are registered in the application.
    ///
    /// # Service Types
    ///
    /// The type argument must be a tuple (2..=16) of [`Service`] implementers. No type must repeat.
    /// The return type is a tuple with each service type borrowed mutable (`&mut S`).
    ///
    /// # Panics
    ///
    /// If the same service type is requested more then once.
    #[inline]
    pub fn get_multi<'m, M: ServiceTuple<'m>>(&'m mut self) -> Option<M::Borrowed> {
        M::get().ok()
    }

    /// Requires multiple service references.
    ///
    /// # Service Types
    ///
    /// The type argument must be a tuple (2..=16) of [`Service`] implementers. No type must repeat.
    /// The return type is a tuple with each service type borrowed mutable (`&mut S`).
    ///
    /// # Panics
    ///
    /// If any of the services is not registered in the application.
    ///
    /// If the same service type is required more then once.
    #[inline]
    #[track_caller]
    pub fn req_multi<'m, M: ServiceTuple<'m>>(&'m mut self) -> M::Borrowed {
        M::get().unwrap_or_else(|e| panic!("service `{e}` is required"))
    }
}

/// Identifies an application service type.
///
/// # Derive
///
/// Implement this trait using `#[derive(Service)]`. It also generates an extension method for [`Services`] using the service name.
///
/// # Examples
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::{service::*, context::WidgetContext};
/// /// Foo-bar service.
/// #[derive(Service)]
/// pub struct FooBar { }
///
/// mod elsewhere {
/// #   use super::*;
///     use crate::FooBarExt;// generated extension method.
///     fn update(ctx: &mut WidgetContext) {
///         let service = ctx.services.foo_bar();
///     }
/// }
/// ```
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait Service: 'static {
    /// Use `#[derive ..]` to implement this trait.
    ///
    /// If that is not possible copy the `thread_local` implementation generated
    /// by the macro as close as possible.
    #[doc(hidden)]
    fn thread_local_entry() -> ServiceEntry<Self>
    where
        Self: Sized;
}

#[doc(hidden)]
pub struct ServiceValue<S: Service> {
    value: Cell<*mut S>,
    assert_count: Rc<()>,
}
#[allow(missing_docs)] // this is hidden
impl<S: Service> ServiceValue<S> {
    pub fn init() -> Self {
        Self {
            value: Cell::new(ptr::null_mut()),
            assert_count: Rc::new(()),
        }
    }
}

#[doc(hidden)]
pub struct ServiceEntry<S: Service> {
    local: &'static LocalKey<ServiceValue<S>>,
}
#[allow(missing_docs)] // this is hidden.
impl<S: Service> ServiceEntry<S> {
    pub fn new(local: &'static LocalKey<ServiceValue<S>>) -> Self {
        Self { local }
    }

    fn init(&self, service: *mut S) -> *mut S {
        self.local.with(move |l| l.value.replace(service))
    }

    fn deinit(&self) {
        self.init(ptr::null_mut());
    }

    fn get(&self) -> *mut S {
        self.local.with(|l| l.value.get())
    }

    fn assert_no_dup(&self) -> Rc<()> {
        let count = self.local.with(|l| Rc::clone(&l.assert_count));
        if Rc::strong_count(&count) == 2 {
            count
        } else {
            panic!("service `{}` already in query", type_name::<S>())
        }
    }
}

mod protected {
    pub trait ServiceTuple<'s> {
        type Borrowed;
        fn assert_no_dup();
        fn get() -> Result<Self::Borrowed, &'static str>;
    }
}
macro_rules! impl_multi_tuple {
    ($( ( $($n:tt),+ ) ),+  $(,)?) => {$($crate::paste!{
        impl_multi_tuple! {
            impl $([<_borrowed $n>], [<ptr $n>] = [<S $n>]),+
        }
    })+};

    (impl $($assert:tt, $ptr:tt = $S:tt),+ ) => {

        impl<'s, $($S: Service),+> protected::ServiceTuple<'s> for ( $($S),+ ) {
            type Borrowed = ( $(&'s mut $S),+ );

            fn assert_no_dup() {
                $(
                    let $assert = $S::thread_local_entry().assert_no_dup();
                )+
            }

            fn get() -> Result<Self::Borrowed, &'static str> {
                Self::assert_no_dup();

                $(
                    let $ptr = $S::thread_local_entry().get();
                    if $ptr.is_null() {
                        return Err(type_name::<$S>());
                    }
                )+

                // SAFETY: assert_no_dup validated that all pointers are unique.
                // The cast to &mut is safe as long as it's only called in Services::get_multi().
                Ok(unsafe {($(
                    &mut *$ptr,
                )+)})
            }
        }

        impl<'s, $($S: Service),+> ServiceTuple<'s> for ( $($S),+ ) { }
    }
}

#[doc(hidden)]
pub trait ServiceTuple<'s>: protected::ServiceTuple<'s> {}

impl_multi_tuple! {
    (0, 1),
    (0, 1, 2),
    (0, 1, 2, 3),
    (0, 1, 2, 3, 4),
    (0, 1, 2, 3, 4, 5),
    (0, 1, 2, 3, 4, 5, 6),
    (0, 1, 2, 3, 4, 5, 6, 7),

    (0, 1, 2, 3, 4, 5, 6, 7, 8),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15),

    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23),

    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31),
}
