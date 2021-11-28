# crossdylib

This library can be used to achieve shared state across shared libraries/modules.

## Support

Supported platforms

* Windows
* Linux

# Example

### `a.dll`

```rust
#[macro_use] extern crate crossdylib;

crossdylib! {
	static THE_ANSWER: std::sync::Mutex<u32> = std::sync::Mutex::new(39);
}

#[no_mangle]
pub unsafe extern "C" fn increment() {
	THE_ANSWER.sync().unwrap();

	let mut lock = THE_ANSWER.lock().unwrap();
	*lock += 1;
	assert_eq!(*lock, 40);
}
```

### `b.dll`

```rust
#[macro_use] extern crate crossdylib;

crossdylib! {
	static THE_ANSWER: std::sync::Mutex<u32> = std::sync::Mutex::new(39);
}

#[no_mangle]
pub unsafe extern "C" fn increment() {
	THE_ANSWER.sync().unwrap();

	let mut lock = THE_ANSWER.lock().unwrap();
	*lock += 1;
	assert_eq!(*lock, 41);
}
```

### `main.exe`

```rust
fn main() {
	let a = Library::new("a.dll").unwrap();
	a.get::<extern "C" fn()>("increment").unwrap()();

	let b = Library::new("b.dll").unwrap();
	b.get::<extern "C" fn()>("increment").unwrap()();

	println!("Success");
}
```