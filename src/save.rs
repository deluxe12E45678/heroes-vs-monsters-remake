//! Persistent progress in save.ron (RON, next to the executable / working dir).
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Equipment slots on the equip screen: helm, armor, left hand (shield/off-hand), right hand (weapon), ring L, ring R, boots.
pub const SLOTS: usize = 7;
pub const STASH: usize = 45; // 3 tabs × 5×3
pub const SHOP: usize = 15;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnedHero {
    pub hero: i32,
    pub level: i32,
    pub exp: i64,
    pub equip: [Option<i32>; SLOTS],
    /// skill-tree node index → points spent
    pub skills: BTreeMap<i32, i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct Save {
    /// Money is stored ×4 like the original (SceneMainGameMachine::addMoneyAndExp adds m*4) [§2, §7].
    pub money_x4: i64,
    pub heroes: Vec<OwnedHero>,
    /// up to 3 indices into `heroes`
    pub party: Vec<usize>,
    pub stash: Vec<Option<i32>>,
    pub shop: Vec<Option<i32>>,
    /// highest unlocked world-map node
    pub unlocked: u32,
    pub current_node: u32,
    pub sound: bool,
    #[serde(default = "yes")] pub music: bool,
    #[serde(default)] pub medals: i64,
    #[serde(default)] pub arena_score: i64,
    /// stash tabs unlocked (original: tab I free, II and III cost 50,000 gold each — SceneShop UnLockBagLabel)
    #[serde(default = "one")] pub bag_tabs: u8,
    /// party size (original: 3, a 4th slot costs 50,000 gold — SceneSelectHero ExpandTeamNumber "BUY_4player")
    #[serde(default = "three")] pub team_size: u8,
    /// SceneHelpGame guided first battle done (old saves: yes)
    #[serde(default = "yes")] pub tutorial_done: bool,
    /// Siege: captured castle indices (GVGConfig order), owned minions (boss enemy index, level), the one that fights
    #[serde(default)] pub castles: Vec<usize>,
    #[serde(default)] pub pets: Vec<(i32, i32)>,
    #[serde(default)] pub active_pet: Option<usize>,
    #[serde(default)] pub badges: i64,
    #[serde(default)] pub stars: i64,
    /// battle speed 1–3 (own addition)
    #[serde(default = "one")] pub battle_speed: u8,
    /// GameData's "unhired" list (DungeonHeroesCpp+0x27c): heroes found by beating a hero camp, shown for hire
    #[serde(default)] pub found: Vec<i32>,
    /// best Survival result (GameData::SetInfinityBattleInfos)
    #[serde(default)] pub survival_best: i32,
}
fn yes() -> bool { true }
fn one() -> u8 { 1 }
fn three() -> u8 { 3 }
pub const UNLOCK_COST: i64 = 50_000;

impl Save {
    pub fn money(&self) -> i64 { self.money_x4 / 4 }
    pub fn add_money(&mut self, m: i64) { self.money_x4 += m * 4; }
    /// save.ron in the working dir; HVSM_SAVE overrides it (self-tests use a throwaway save, never the player's).
    /// Save slots 1-3 (own design): slot 1 is save.ron (the existing save, untouched), slots 2/3 are save2.ron / save3.ron.
    /// With HVSM_SAVE the slots are that file, <file>.2 and <file>.3.
    pub fn path() -> std::path::PathBuf { Self::slot_path(SLOT.load(std::sync::atomic::Ordering::Relaxed)) }
    pub fn slot_path(slot: usize) -> std::path::PathBuf {
        let base = std::env::var("HVSM_SAVE").ok();
        let name = match (slot, base) {
            (0 | 1, Some(b)) => b, (n, Some(b)) => format!("{b}.{n}"),
            (0 | 1, None) => "save.ron".into(), (n, None) => format!("save{n}.ron"),
        };
        // Android: the app's internal storage (the working dir is not writable there).
        #[cfg(target_os = "android")]
        if let Some(dir) = bevy::android::ANDROID_APP.get().and_then(|a| a.internal_data_path()) { return dir.join(name); }
        std::path::PathBuf::from(name)
    }
    pub fn read_slot(slot: usize) -> Option<Save> {
        std::fs::read_to_string(Self::slot_path(slot)).ok().and_then(|s| ron::from_str::<Save>(&s).ok())
            .map(|mut s| { if s.unlocked >= 2 { s.team_size = s.team_size.max(4); } s })
    }
    pub fn load_or_new(new: impl FnOnce() -> Save) -> Save {
        Self::read_slot(SLOT.load(std::sync::atomic::Ordering::Relaxed)).unwrap_or_else(new)
    }
    pub fn write(&self) {
        if let Ok(s) = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) { let _ = std::fs::write(Self::path(), s); }
    }
    pub fn new_hero(hero: i32, root_skill: Option<i32>) -> OwnedHero {
        let mut skills = BTreeMap::new();
        if let Some(s) = root_skill { skills.insert(s, 1); }
        OwnedHero { hero, level: 1, exp: 0, equip: [None; SLOTS], skills }
    }
    pub fn first_free_stash(&self) -> Option<usize> { self.stash.iter().take(self.bag_tabs as usize * 15).position(|s| s.is_none()) }
}

/// active save slot (1..=3)
pub static SLOT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
