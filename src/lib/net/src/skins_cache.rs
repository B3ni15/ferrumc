use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Clone, Debug)]
pub struct SkinProperties {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

lazy_static::lazy_static! {
    static ref SKINS: RwLock<HashMap<i32, SkinProperties>> = RwLock::new(HashMap::new());
}

pub fn insert_skin(short_uuid: i32, props: SkinProperties) {
    let mut map = SKINS.write().unwrap();
    map.insert(short_uuid, props);
}

pub fn get_skin(short_uuid: i32) -> Option<SkinProperties> {
    let map = SKINS.read().unwrap();
    map.get(&short_uuid).cloned()
}

pub fn clear_cache() {
    let mut map = SKINS.write().unwrap();
    map.clear();
}
