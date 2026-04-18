use js_sys::{Map, Uint8Array};
use wasm_bindgen::JsValue;

pub fn tree(entries: &[(&str, &[u8])]) -> JsValue {
    let map = Map::new();
    for (path, bytes) in entries {
        let array = Uint8Array::new_with_length(bytes.len() as u32);
        array.copy_from(bytes);
        map.set(&JsValue::from_str(path), &array.into());
    }
    map.into()
}
