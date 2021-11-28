//! # crossdylib
//!
//! This library can be used to achieve shared state across shared libraries/modules.
//!
//! ## Support
//!
//! Supported platforms
//!
//! * Windows
//! * Linux
//!
//! # Example
//!
//! ### `a.dll`
//!
//! ```ignore
//! #[macro_use] extern crate crossdylib;
//!
//! crossdylib! {
//! 	static THE_ANSWER: std::sync::Mutex<u32> = std::sync::Mutex::new(39);
//! }
//!
//! #[no_mangle]
//! pub unsafe extern "C" fn increment() {
//! 	THE_ANSWER.sync().unwrap();
//!
//! 	let mut lock = THE_ANSWER.lock().unwrap();
//! 	*lock += 1;
//! 	assert_eq!(*lock, 40);
//! }
//! ```
//!
//! ### `b.dll`
//!
//! ```ignore
//! #[macro_use] extern crate crossdylib;
//!
//! crossdylib! {
//! 	static THE_ANSWER: std::sync::Mutex<u32> = std::sync::Mutex::new(39);
//! }
//!
//! #[no_mangle]
//! pub unsafe extern "C" fn increment() {
//! 	THE_ANSWER.sync().unwrap();
//!
//! 	let mut lock = THE_ANSWER.lock().unwrap();
//! 	*lock += 1;
//! 	assert_eq!(*lock, 41);
//! }
//! ```
//!
//! ### `main.exe`
//!
//! ```ignore
//! fn main() {
//! 	let a = Library::new("a.dll").unwrap();
//! 	a.get::<extern "C" fn()>("increment").unwrap()();
//!
//! 	let b = Library::new("b.dll").unwrap();
//! 	b.get::<extern "C" fn()>("increment").unwrap()();
//!
//! 	println!("Success");
//! }
//! ```

use std::sync::{Arc, atomic::AtomicBool};

#[doc(hidden)]
pub use concat_idents::concat_idents as __concat_idents;

#[doc(hidden)]
#[cfg(debug_assertions)]
pub const fn __new_container<T>() -> atomic_refcell::AtomicRefCell<Option<Arc<T>>> {
	atomic_refcell::AtomicRefCell::new(None)
}

#[doc(hidden)]
#[cfg(not(debug_assertions))]
pub const fn __new_container<T>() -> core::cell::UnsafeCell<Option<Arc<T>>> {
	core::cell::UnsafeCell::new(None)
}

#[doc(hidden)]
pub struct CrossDylib<T> {
	#[doc(hidden)]
	pub syncing: AtomicBool, // This flag prevents us iterating over OURSELF

	#[doc(hidden)]
	#[cfg(not(debug_assertions))]
	pub inner: core::cell::UnsafeCell<Option<Arc<T>>>,

	#[doc(hidden)]
	#[cfg(debug_assertions)]
	pub inner: atomic_refcell::AtomicRefCell<Option<Arc<T>>>,

	#[doc(hidden)]
	pub symbol: &'static [u8],

	#[doc(hidden)]
	pub init: fn() -> T
}

unsafe impl<T> Sync for CrossDylib<T> {}

impl<T> CrossDylib<T> {
	#[doc(hidden)]
	#[inline]
	#[cfg(not(debug_assertions))]
	pub fn new_ref(&self) -> Arc<T> {
		match unsafe { &*self.inner.get() } {
			Some(ref inner) => inner.clone(),
			None => unreachable!()
		}
	}

	#[doc(hidden)]
	#[inline]
	#[cfg(debug_assertions)]
	pub fn new_ref(&self) -> Arc<T> {
		match &*self.inner.borrow() {
			Some(inner) => inner.clone(),
			None => unreachable!()
		}
	}

	/// Synchronizes the value across all loaded shared libraries. You must call this function before using a CrossDylib.
	///
	/// **This function is NOT thread safe.** Do not call this function from more than one thread. Always wait for this function to complete before spawning threads that use the CrossDylib.
	///
	/// # Panics
	///
	/// This function will panic in debug mode if a race condition occurs.
	pub unsafe fn sync(&self) -> Result<(), libloading::Error> {
		use findshlibs::{IterationControl, SharedLibrary};

		assert!(self.symbol.len() > 0 && self.symbol.ends_with(&[0u8]));

		let mut result = Ok(());

		#[cfg(not(debug_assertions))]
		let inner = &mut *self.inner.get();

		#[cfg(debug_assertions)]
		let mut inner = self.inner.borrow_mut();

		self.syncing.store(true, std::sync::atomic::Ordering::Release);

		// Try and find a module that has already created this
		findshlibs::TargetSharedLibrary::each(|shlib| {
			let lib = match libloading::Library::new(shlib.name()) {
				Ok(lib) => lib,
				Err(err) => {
					result = Err(err);
					return IterationControl::Break
				},
			};

			match lib.get::<extern "Rust" fn() -> Option<Arc<T>>>(self.symbol) {
				Err(libloading::Error::DlSym { .. }) | Err(libloading::Error::DlSymUnknown) | Err(libloading::Error::GetProcAddress { .. }) | Err(libloading::Error::GetProcAddressUnknown) => IterationControl::Continue,
				Err(err) => {
					result = Err(err);
					IterationControl::Break
				},
				Ok(sym) => {
					if let Some(init) = sym() {
						*inner = Some(init);
						IterationControl::Break
					} else {
						// We just iterated over ourself
						IterationControl::Continue
					}
				}
			}
		});

		self.syncing.store(false, std::sync::atomic::Ordering::Release);

		// Initialise if we haven't already
		if inner.is_none() {
			*inner = Some(Arc::new((self.init)()));
		}

		result
	}
}

impl<T> std::ops::Deref for CrossDylib<T> {
	type Target = T;

	#[inline]
	fn deref(&self) -> &Self::Target {
		unsafe {
			let inner = {
				#[cfg(not(debug_assertions))] {
					&*self.inner.get()
				}
				#[cfg(debug_assertions)] {
					drop(self.inner.borrow());
					&*self.inner.as_ptr()
				}
			};

			debug_assert!(inner.is_some(), "CrossDylib::sync() must be called before accessing a CrossDylib");

			match &*inner {
				Some(inner) => &*inner,
				None => std::hint::unreachable_unchecked()
			}
		}
	}
}

#[macro_export]
/// Creates CrossDylibs.
///
/// CrossDylibs must be synchronized using the `CrossDylib::sync` function before use.
///
/// # Panics
///
/// CrossDylibs will panic if accessed before `CrossDylib::sync` is called in debug mode.
///
/// In release mode this will lead to undefined behaviour.
///
/// # Example
///
/// ### `a.dll`
///
/// ```ignore
/// #[macro_use] extern crate crossdylib;
///
/// crossdylib! {
/// 	static THE_ANSWER: std::sync::Mutex<u32> = std::sync::Mutex::new(39);
/// }
///
/// #[no_mangle]
/// pub unsafe extern "C" fn increment() {
/// 	THE_ANSWER.sync().unwrap();
///
/// 	let mut lock = THE_ANSWER.lock().unwrap();
/// 	*lock += 1;
/// 	assert_eq!(*lock, 40);
/// }
/// ```
///
/// ### `b.dll`
///
/// ```ignore
/// #[macro_use] extern crate crossdylib;
///
/// crossdylib! {
/// 	static THE_ANSWER: std::sync::Mutex<u32> = std::sync::Mutex::new(39);
/// }
///
/// #[no_mangle]
/// pub unsafe extern "C" fn increment() {
/// 	THE_ANSWER.sync().unwrap();
///
/// 	let mut lock = THE_ANSWER.lock().unwrap();
/// 	*lock += 1;
/// 	assert_eq!(*lock, 41);
/// }
/// ```
///
/// ### `main.exe`
///
/// ```ignore
/// fn main() {
/// 	let a = Library::new("a.dll").unwrap();
/// 	a.get::<extern "C" fn()>("increment").unwrap()();
///
/// 	let b = Library::new("b.dll").unwrap();
/// 	b.get::<extern "C" fn()>("increment").unwrap()();
///
/// 	println!("Success");
/// }
/// ```
macro_rules! crossdylib {
	{ $($vis:vis static $ident:ident: $ty:ty = $expr:expr;)+ } => {
		$(
			$vis static $ident: $crate::CrossDylib<$ty> = $crate::CrossDylib {
				inner: $crate::__new_container::<$ty>(),
				symbol: concat!("__crossdylib_", stringify!($ident), "\0").as_bytes(),
				syncing: ::std::sync::atomic::AtomicBool::new(false),
				init: || $expr
			};

			$crate::__concat_idents!(export_name = __crossdylib_, $ident {
				#[no_mangle]
				#[doc(hidden)]
				#[allow(non_snake_case)]
				pub unsafe extern "Rust" fn export_name() -> ::core::option::Option<::std::sync::Arc<$ty>> {
					if $ident.syncing.load(std::sync::atomic::Ordering::Acquire) {
						None
					} else {
						Some($ident.new_ref())
					}
				}
			});
		)+
	};
}