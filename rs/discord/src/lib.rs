pub struct Component;

mod api;
mod listener;

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        path: "wit/package.wasm",
        world: "component",
        generate_all,
    });
}

bindings::export!(Component with_types_in bindings);
