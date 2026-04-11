use std::collections::HashMap;
use std::sync::OnceLock;

pub static GADGET_MAPPING_COLLECTION: OnceLock<std::sync::Arc<HashMap<u32, String>>> =
    OnceLock::new();

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GadgetMapping {
    pub gadget_id: u32,
    pub server_controller: String,
}

impl GadgetMapping {
    pub fn load(custom_output_path: &str) {
        let gadget_mapping_json =
            std::fs::read(&format!("{custom_output_path}/GadgetMapping.json")).unwrap();
        let mappings: Vec<GadgetMapping> = serde_json::from_slice(&gadget_mapping_json).unwrap();

        let mut map = HashMap::new();
        for mapping in mappings {
            map.insert(mapping.gadget_id, mapping.server_controller);
        }

        GADGET_MAPPING_COLLECTION
            .set(std::sync::Arc::new(map))
            .unwrap();
    }

    pub fn get_gadget_lua_name(gadget_id: u32) -> Option<String> {
        GADGET_MAPPING_COLLECTION
            .get()
            .and_then(|map| map.get(&gadget_id).cloned())
    }
}
