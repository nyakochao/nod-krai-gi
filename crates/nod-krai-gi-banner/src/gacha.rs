use nod_krai_gi_data::custom::{BannerWeight, GachaBanner};
use nod_krai_gi_data::excel::common::MaterialType;
use nod_krai_gi_data::excel::{material_excel_config_collection, weapon_excel_config_collection};
use nod_krai_gi_proto::server_only::GachaBin;
use rand::prelude::SmallRng;
use rand::Rng;

pub fn roll_gacha_once_readonly(
    rng: &mut SmallRng,
    banner: &GachaBanner,
    bin: &GachaBin,
) -> (u32, bool, bool, bool, bool) {
    let material_excel_config_collection_clone =
        std::sync::Arc::clone(material_excel_config_collection::get());

    let weapon_excel_config_collection_clone =
        std::sync::Arc::clone(weapon_excel_config_collection::get());

    let default_avatar_5 = material_excel_config_collection_clone
        .iter()
        .filter(|(_, v)| v.material_type == MaterialType::Avatar && v.rank_level == 5)
        .map(|(k, _)| *k)
        .collect::<Vec<u32>>();

    let default_avatar_4 = material_excel_config_collection_clone
        .iter()
        .filter(|(_, v)| v.material_type == MaterialType::Avatar && v.rank_level == 4)
        .map(|(k, _)| *k)
        .collect::<Vec<u32>>();

    let default_weapon_5 = weapon_excel_config_collection_clone
        .iter()
        .filter(|(_, v)| v.rank_level == 5)
        .map(|(k, _)| *k)
        .collect::<Vec<u32>>();

    let default_weapon_4 = weapon_excel_config_collection_clone
        .iter()
        .filter(|(_, v)| v.rank_level == 4)
        .map(|(k, _)| *k)
        .collect::<Vec<u32>>();

    let default_weapon_3 = weapon_excel_config_collection_clone
        .iter()
        .filter(|(_, v)| v.rank_level == 3)
        .map(|(k, _)| *k)
        .collect::<Vec<u32>>();

    let mut rate_up_items_5 = banner.rate_up_items_5.clone();
    let mut rate_up_items_4 = banner.rate_up_items_4.clone();

    let mut fallback_items_5_pool = vec![];
    let mut fallback_items_4_pool = vec![];

    if banner.gacha_type == 302 {
        if rate_up_items_5.is_empty() {
            rate_up_items_5 = default_weapon_5;
        };
        if rate_up_items_4.is_empty() {
            rate_up_items_4 = default_weapon_4;
        };
        if fallback_items_5_pool.is_empty() {
            fallback_items_5_pool = banner.fallback_items_5_pool_2.clone().unwrap_or_default();
        }
        if fallback_items_4_pool.is_empty() {
            fallback_items_4_pool = banner.fallback_items_4_pool_2.clone().unwrap_or_default();
        }
    } else {
        if rate_up_items_5.is_empty() {
            rate_up_items_5 = default_avatar_5;
        };
        if rate_up_items_4.is_empty() {
            rate_up_items_4 = default_avatar_4;
        };
        if fallback_items_5_pool.is_empty() {
            fallback_items_5_pool = banner.fallback_items_5_pool_1.clone().unwrap_or_default();
        }
        if fallback_items_4_pool.is_empty() {
            fallback_items_4_pool = banner.fallback_items_4_pool_1.clone().unwrap_or_default();
        }
    }

    if fallback_items_5_pool.is_empty() {
        fallback_items_5_pool = rate_up_items_5.clone();
    }
    if fallback_items_4_pool.is_empty() {
        fallback_items_4_pool = rate_up_items_4.clone();
    }

    let mut fallback_items_3_pool = banner.fallback_items_3.clone().unwrap_or_default();
    if fallback_items_3_pool.is_empty() {
        fallback_items_3_pool = default_weapon_3;
    }

    assert!(!rate_up_items_5.is_empty(), "rate_up_items_5 is empty");
    assert!(!rate_up_items_4.is_empty(), "rate_up_items_4 is empty");
    assert!(
        !fallback_items_5_pool.is_empty(),
        "fallback_items_5_pool is empty"
    );
    assert!(
        !fallback_items_4_pool.is_empty(),
        "fallback_items_4_pool is empty"
    );
    assert!(
        !fallback_items_3_pool.is_empty(),
        "fallback_items_3_pool is empty"
    );

    // =====================================================
    // 1. 5★
    // =====================================================
    let is_5 = roll_star(rng, bin.fail_5_count, banner.weights_5.as_ref());
    if is_5 {
        let pity5 = if *bin.item5_is_up_list.last().unwrap_or(&true) {
            0
        } else {
            1
        };

        let is_up = if pity5 >= 1 {
            true
        } else {
            rng.gen_range(0..100) < 60
        };

        if is_up {
            let item = if bin.wish_item_id != 0 {
                bin.wish_item_id
            } else {
                let up = rate_up_items_5.as_slice();
                up[rng.gen_range(0..up.len())]
            };

            return (item, false, true, false, true);
        }

        let item = fallback_items_5_pool[rng.gen_range(0..fallback_items_5_pool.len())];

        return (item, false, true, false, false);
    }

    // =====================================================
    // 2. 4★
    // =====================================================
    let is_4 = roll_star(rng, bin.fail_4_count, banner.weights_4.as_ref());
    if is_4 {
        let pity4 = if *bin.item4_is_up_list.last().unwrap_or(&true) {
            0
        } else {
            1
        };

        let is_up = if pity4 >= 1 {
            true
        } else {
            rng.gen_range(0..100) < 60
        };

        if is_up {
            let up = rate_up_items_4.as_slice();
            let item = up[rng.gen_range(0..up.len())];
            return (item, true, false, true, false);
        }

        let item = fallback_items_4_pool[rng.gen_range(0..fallback_items_4_pool.len())];

        return (item, true, false, false, false);
    }

    // =====================================================
    // 3. 3★
    // =====================================================
    let item = fallback_items_3_pool[rng.gen_range(0..fallback_items_3_pool.len())];

    (item, false, false, false, false)
}

///（fail_count + weights）
fn roll_star(rng: &mut SmallRng, fail: u32, weights: Option<&Vec<BannerWeight>>) -> bool {
    let weights = match weights {
        Some(w) => w,
        None => return false,
    };

    let mut weight = 0;

    for bw in weights {
        if fail >= bw.pulls {
            weight = bw.weight;
        }
    }

    if weight >= 10000 {
        return true;
    }

    rng.gen_range(0..10000) < weight
}
