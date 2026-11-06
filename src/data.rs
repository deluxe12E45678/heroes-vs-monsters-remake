//! Game tables loaded at startup with plain ron + std::fs (assets/data/*.ron),
//! plus the pre-rendered sprite manifest (assets/sprites/manifest.json).
use crate::types::*;
use bevy::prelude::Resource;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

pub fn asset_dir() -> PathBuf {
    let local = PathBuf::from("assets");
    if local.join("data").exists() { std::env::current_dir().map(|d| d.join("assets")).unwrap_or(local) } else { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets") }
}

/// Reads a text file under assets/ (relative path). On Android it comes out of the APK via the AssetManager.
pub fn read_text(rel: &str) -> Result<String, String> {
    #[cfg(target_os = "android")]
    {
        use std::io::Read;
        let app = bevy::android::ANDROID_APP.get().ok_or("no android app")?;
        let c = std::ffi::CString::new(rel).map_err(|e| e.to_string())?;
        let mut a = app.asset_manager().open(&c).ok_or_else(|| format!("{rel}: not in apk"))?;
        let mut s = String::new(); a.read_to_string(&mut s).map_err(|e| format!("{rel}: {e}"))?; return Ok(s);
    }
    #[cfg(not(target_os = "android"))]
    { let p = asset_dir().join(rel); std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display())) }
}

/// Whether assets/<rel> exists (inside the APK on Android).
pub fn asset_exists(rel: impl AsRef<str>) -> bool {
    let rel = rel.as_ref();
    #[cfg(target_os = "android")]
    { let Some(app) = bevy::android::ANDROID_APP.get() else { return false };
      std::ffi::CString::new(rel).ok().and_then(|c| app.asset_manager().open(&c)).is_some() }
    #[cfg(not(target_os = "android"))]
    { asset_dir().join(rel).exists() }
}

fn load<T: serde::de::DeserializeOwned>(name: &str) -> Result<T, String> {
    let s = read_text(&format!("data/{name}"))?;
    ron::from_str(&s).map_err(|e| format!("{name}: {e}"))
}

/// WorldMoveMap.xml node (coords in the 1025x623 map image, y down).
#[derive(Debug, Clone, Deserialize)]
pub struct MapNode {
    pub index: u32, pub x: f32, pub y: f32, pub unlock_level: i32, pub icon: String,
    pub boss_level: i32, pub boss_map: u32, pub min_level: i32, pub max_level: i32, pub maps: Vec<u32>,
    #[serde(default)] pub radius: f32, #[serde(default)] pub boss_radius: f32, #[serde(default)] pub begin_dis: f32, #[serde(default)] pub per_pixel: f32,
    /// WorldMoveMap.xml <FallDown type="normalEquipIndex"> — pool + Usually Drops icons on the parchment.
    #[serde(default)] pub normal_drops: Vec<i32>,
    /// <FallDown type="normalEquipLevel"> — upgrade level rolled per drop (parallel to index picks).
    #[serde(default)] pub normal_drop_levels: Vec<i32>,
    /// <FallDown type="normalEquipNumber"> — possible drop counts; AddGameOverScene picks one uniformly (e.g. 0 0 1 1 2 → P(drop)≈60%, E≈0.8).
    #[serde(default)] pub normal_drop_numbers: Vec<i32>,
    /// <FallDown type="bossEquipIndex">.
    #[serde(default)] pub boss_drops: Vec<i32>,
    #[serde(default)] pub boss_drop_levels: Vec<i32>,
    /// Boss counts are typically 1 1 1 1 1 2 → always ≥1, E≈1.17.
    #[serde(default)] pub boss_drop_numbers: Vec<i32>,
}
/// Offline arena: the PVP_DummyList.xml opponents.
#[derive(Debug, Clone, Deserialize)]
pub struct ArenaTarget { pub name: String, pub rank: i32, pub score: i32, pub ave_level: i32, pub team: Vec<(i32, i32)> }
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ArenaTable { pub targets: Vec<ArenaTarget> }

/// GVGConfig.xml castles + battle reward formulas (a·x² + b·x + c of the boss level, same form as the enemy formulas).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Castle { pub boss: i32, pub name: String, pub level: i32, pub min: i32, pub max: i32, pub icon: String, pub x: f32, pub y: f32 }
#[derive(Debug, Clone, Deserialize, Default)]
pub struct SiegeTable { pub castles: Vec<Castle>, pub stars: (f32, f32, f32), pub exp: (f32, f32, f32), pub money: (f32, f32, f32), pub badges: (f32, f32, f32) }
impl SiegeTable { pub fn eval(f: (f32, f32, f32), x: f32) -> i64 { (f.0 * x * x + f.1 * x + f.2).round() as i64 } }

#[derive(Debug, Clone, Deserialize)]
pub struct WorldMap {
    pub width: f32, pub height: f32, pub nodes: Vec<MapNode>,
    #[serde(default)] pub bounds: (f32, f32, f32, f32), #[serde(default)] pub begin: (f32, f32), #[serde(default)] pub move_speed: f32,
    #[serde(default)] pub task_maps: Vec<u32>, #[serde(default)] pub task_dist: f32, #[serde(default)] pub task_level: i32,
    #[serde(default)] pub hero_maps: Vec<u32>, #[serde(default)] pub hero_dist: f32, #[serde(default)] pub hero_level: i32,
}

/// One pre-rendered unit sheet: rows = standby, move, attack, hurt, dead, skill.
#[derive(Debug, Clone, Deserialize)]
pub struct SheetInfo { pub cell: (u32, u32), pub cols: u32, pub anchor: (f32, f32), pub frames: Vec<u32>, pub anim: String,
    /// feet-to-top height of standby frame 0 in px (for fitting figures to a slot)
    #[serde(default)] pub body_h: f32,
    /// AnimationType.xml attack_a index: the frame (24 fps) where the blow lands / the arrow is released
    #[serde(default = "hit12")] pub hit: u32 }
fn hit12() -> u32 { 12 }

#[derive(Resource)]
pub struct GameData {
    pub heroes: HeroTable,
    pub enemies: EnemyTable,
    pub skills: SkillTable,
    pub equipment: EquipmentTable,
    pub upgrade: WeaponUpgradeTable,
    pub waves: WaveTable,
    pub world: WorldMap,
    pub arena: ArenaTable,
    pub siege: SiegeTable,
    pub shop_pool: Vec<i32>,
    pub money: Option<crate::types::MoneyConfig>,
    pub sheets: HashMap<String, SheetInfo>,
    /// EnemyRules <FallHero> per map: (hero indices, weights)
    pub survival: Survival,
    pub fall_hero: std::collections::BTreeMap<u32, (Vec<i32>, Vec<i32>)>,
}

impl GameData {
    pub fn load() -> Result<Self, String> {
        let sheets = read_text("sprites/manifest.json").ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Ok(Self {
            heroes: load("heroes.ron")?,
            enemies: load("enemies.ron")?,
            skills: load("skills.ron")?,
            equipment: load("equipment.ron")?,
            upgrade: load("weapon_upgrade.ron")?,
            waves: load("waves.ron")?,
            world: load("worldmap.ron")?,
            arena: load("arena.ron").unwrap_or_default(),
            siege: load("siege.ron").unwrap_or_default(),
            shop_pool: load("shop_pool.ron")?,
            money: load("money_config.ron").ok(),
            survival: load("survival.ron").unwrap_or_default(),
            fall_hero: load("fall_hero.ron").unwrap_or_default(),
            sheets,
        })
    }
    pub fn hero(&self, i: i32) -> &Hero { self.heroes.heroes.iter().find(|h| h.index == i).expect("hero") }
    pub fn enemy(&self, i: i32) -> &Enemy { self.enemies.enemies.iter().find(|e| e.index == i).expect("enemy") }
    pub fn skill(&self, i: i32) -> Option<&Skill> { self.skills.skills.iter().find(|s| s.index == i) }
    /// MoneyConfig.xml BossCastle row for the player's progress (row = areas unlocked, 1-based; approximation of the castle tier).
    pub fn prices(&self, unlocked: u32) -> Option<&crate::types::CastlePrices> {
        let m = self.money.as_ref()?; let k = (unlocked + 1).min(*m.boss_castle.keys().last()?);
        m.boss_castle.range(..=k).next_back().map(|(_, v)| v)
    }
    pub fn item(&self, i: i32) -> Option<&Equipment> { self.equipment.items.iter().find(|e| e.index == i) }
}

/// Heroes offered in the Inn (one of each class, as in the original hire list).
pub const INN_HEROES: [i32; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 29];

/// EnemyRules_200.xml (SurvivalMode). rules: (lastdienum, deltatime ms, x); boss: (enemy, lastdienum, deltatime, x); award: (teamNum, items)
#[derive(serde::Deserialize, Default, Clone)]
pub struct Survival {
    pub map_tex: Vec<i32>, pub base_level: i32, pub add_level: i32, pub rules: Vec<Vec<(i32, i32, i32)>>, pub pool: Vec<i32>,
    pub boss_base: i32, pub boss_add: i32, pub boss: Vec<Vec<(i32, i32, i32, i32)>>, pub award: Vec<(i32, Vec<i32>)>,
}
