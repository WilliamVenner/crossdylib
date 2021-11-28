use libloading::Library;

#[cfg(debug_assertions)]
macro_rules! lib_path {
	($lib:literal) => {
		concat!($lib, "/target/debug/lib", $lib, ".so")
	};
}

#[cfg(not(debug_assertions))]
macro_rules! lib_path {
	($lib:literal) => {
		concat!($lib, "/target/release/lib", $lib, ".so")
	};
}

fn main() {
	unsafe {
		let a = Library::new(lib_path!("a1")).unwrap();
		a.get::<extern "C" fn()>(b"increment\0").unwrap()();

		let b = Library::new(lib_path!("b2")).unwrap();
		b.get::<extern "C" fn()>(b"increment\0").unwrap()();

		let c = Library::new(lib_path!("c3")).unwrap();
		c.get::<extern "C" fn()>(b"increment\0").unwrap()();

		std::mem::forget(a);
		std::mem::forget(b);
		std::mem::forget(c);

		println!("Success");
	}
}
