use std::sync::Mutex;

crossdylib::crossdylib! {
	static THE_ANSWER: Mutex<u32> = Mutex::new(39);
}

#[no_mangle]
pub unsafe extern "C" fn increment() {
	THE_ANSWER.sync().unwrap();
	let mut lock = THE_ANSWER.lock().unwrap();
	*lock += 1;
	assert_eq!(*lock, 42);
}