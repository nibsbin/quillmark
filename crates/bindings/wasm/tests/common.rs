use js_sys::{Map, Object, Reflect, Uint8Array};
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

/// Build a plain-object file tree (`Record<string, Uint8Array>`).
#[allow(dead_code)]
pub fn tree_object(entries: &[(&str, &[u8])]) -> JsValue {
    let obj = Object::new();
    for (path, bytes) in entries {
        let array = Uint8Array::new_with_length(bytes.len() as u32);
        array.copy_from(bytes);
        Reflect::set(&obj, &JsValue::from_str(path), &array.into()).unwrap();
    }
    obj.into()
}
