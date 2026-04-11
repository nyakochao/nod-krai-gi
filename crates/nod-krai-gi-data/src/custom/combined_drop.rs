use common::string_util::InternString;
use std::collections::HashMap;

static COMBINED_DROP_DATA: std::sync::OnceLock<
    std::sync::Arc<HashMap<InternString, Vec<CombinedDrop>>>,
> = std::sync::OnceLock::new();

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CombinedDrop {
    pub index: InternString,
    pub min_level: u32,
    pub drop_id: u32,
    pub drop_count: u32,
}

impl CombinedDrop {
    fn key(&self) -> InternString {
        self.index
    }

    pub fn load(custom_output_path: &str) {
        // Load ChestDrop.json
        let chest_json = std::fs::read(&format!("{custom_output_path}/ChestDrop.json")).unwrap();
        let chest_list: Vec<CombinedDrop> = serde_json::from_slice(&*chest_json).unwrap();

        // Load MonsterDrop.json
        let monster_json =
            std::fs::read(&format!("{custom_output_path}/MonsterDrop.json")).unwrap();
        let monster_list: Vec<CombinedDrop> = serde_json::from_slice(&*monster_json).unwrap();

        // Combine both lists into a single HashMap
        let mut data = HashMap::new();

        // Add chest drops
        for item in chest_list {
            match data.get_mut(&item.key()) {
                None => {
                    data.insert(item.key(), vec![item.clone()]);
                }
                Some(list) => {
                    list.push(item.clone());
                }
            }
        }

        // Add monster drops
        for item in monster_list {
            match data.get_mut(&item.key()) {
                None => {
                    data.insert(item.key(), vec![item.clone()]);
                }
                Some(list) => {
                    list.push(item.clone());
                }
            }
        }

        data.iter_mut().for_each(|(_, v)| {
            v.sort_by(|a, b| b.min_level.cmp(&a.min_level));
        });

        COMBINED_DROP_DATA.set(std::sync::Arc::new(data)).unwrap();
    }

    pub fn get_drop_config(drop_tag: String, level: u32) -> Option<CombinedDrop> {
        match COMBINED_DROP_DATA
            .get()
            .unwrap()
            .get(&InternString::from(drop_tag))
        {
            None => {}
            Some(list) => {
                for item in list {
                    if level >= item.min_level {
                        return Some(item.clone());
                    }
                }
            }
        }
        None
    }
}
