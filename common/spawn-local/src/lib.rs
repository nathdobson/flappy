#[cfg(not(target_family = "wasm"))]
pub fn spawn_local<F: Future<Output = ()> + 'static>(f: F) {
    tokio::task::spawn_local(f);
}

#[cfg(target_family = "wasm")]
pub fn spawn_local<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

#[cfg(not(target_family = "wasm"))]
pub fn spawn<F: Send + Future<Output = ()> + 'static>(f: F) {
    tokio::task::spawn(f);
}

#[cfg(target_family = "wasm")]
pub fn spawn<F: Send + Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}
