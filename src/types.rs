//! Bevy-ready types for the Heroes vs Monsters data in this folder (*.ron).
//! Generated from the decoded game XML by /workspace/hvsm/toron.py.
//! Fields ending in `_code` hold raw game codes whose meaning is not confirmed.
//!
//! Loading in Bevy (bevy_common_assets, `ron` feature):
//! ```ignore
//! // Cargo.toml: bevy_common_assets = { version = "<match your Bevy>", features = ["ron"] }
//! // Build with `--features bevy` so the root tables derive Asset + TypePath.
//! use bevy_common_assets::ron::RonAssetPlugin;
//! app.add_plugins((
//!     RonAssetPlugin::<HeroTable>::new(&["heroes.ron"]),
//!     RonAssetPlugin::<EnemyTable>::new(&["enemies.ron"]),
//!     RonAssetPlugin::<SkillTable>::new(&["skills.ron"]),
//!     RonAssetPlugin::<EquipmentTable>::new(&["equipment.ron"]),
//!     RonAssetPlugin::<WeaponUpgradeTable>::new(&["weapon_upgrade.ron"]),
//!     RonAssetPlugin::<WaveTable>::new(&["waves.ron"]),
//!     RonAssetPlugin::<MoneyConfig>::new(&["money_config.ron"]),
//! ));
//! let heroes: Handle<HeroTable> = asset_server.load("data/heroes.ron");
//! ```
//! Each plugin needs a full extension that is unique, so register "heroes.ron" etc. as shown,
//! not "ron" for every table.
//!
//! Level formulas: value = base + a*x^2 + b*x + c, where x = displayed_level - 1 (and 0 at level 0).
use serde::Deserialize;
use std::collections::BTreeMap;


#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Formula { pub a: f32, pub b: f32, pub c: f32 }
impl Formula {
    /// The game's getFloatFromFormula, with x = displayed level - 1 (0 => 0).
    pub fn eval(&self, x: f32) -> f32 { if x == 0.0 { 0.0 } else { self.a * x * x + self.b * x + self.c } }
}
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Rect4 { pub x: f32, pub y: f32, pub z: f32, pub w: f32 }

#[derive(Debug, Clone, Deserialize)]
pub struct BaseStats {
    pub hp: i32, pub attack: i32, pub ac: i32, pub sr: i32, pub hit_rate: i32,
    pub attack_speed_ms: i32, pub move_speed: i32, pub crit_rate: i32,
    pub ac_resistance: i32, pub sr_resistance: i32,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Formulas {
    pub hp: Option<Formula>, pub attack: Option<Formula>, pub attack_speed: Option<Formula>,
    pub hit_rate: Option<Formula>, pub exp: Option<Formula>, pub money: Option<Formula>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum AttackType { Attack, Buff }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum AttackMode { Near, Far, Skill }
/// raw: solfier, arrow, firestaff, icestaff, lightstaff, magicstaff, heal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Profession { Soldier, Archer, FireStaff, IceStaff, LightStaff, MagicStaff, Healer }

#[derive(Debug, Clone, Deserialize)]
pub struct SkillTreeNode {
    pub skill: i32, pub before: i32, pub need_had_points: i32, pub need_before_points: i32,
    pub max_points: i32, pub row: i32, pub line: i32, pub exclusion_flag: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Hero {
    pub index: i32, pub stats: BaseStats, pub attack_type: AttackType, pub attack_mode: AttackMode,
    pub profession: Profession, pub hatred: i32, pub resist_hatred: i32,
    pub attack_rect: Option<Rect4>, pub trigger_rect: Option<Rect4>, pub formulas: Formulas,
    /// component -> sprite index (-1 = none; empty raw entries are omitted)
    pub base_ware: BTreeMap<String, i32>, pub skill_tree: Vec<SkillTreeNode>,
    pub class_name: String,
    #[serde(default)] pub class_name_zh: String,
    /// allowed WeaponType / armour type codes
    pub weapon_types: Vec<i32>, pub clothes_types: Vec<i32>, pub hat_types: Vec<i32>, pub shoe_types: Vec<i32>,
    pub price: i32, pub super_hero: bool,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Enemy {
    pub index: i32, #[serde(default)] pub name: String, pub stats: BaseStats, pub attack_type: AttackType, pub attack_mode: AttackMode,
    pub profession: Profession, pub hatred: i32, pub resist_hatred: i32,
    pub attack_rect: Option<Rect4>, pub trigger_rect: Option<Rect4>, pub formulas: Formulas,
    pub base_ware: BTreeMap<String, i32>, pub skill_tree: Vec<SkillTreeNode>,
    pub exp: i32, pub money: i32,
    /// Race 0..=4; names unconfirmed (undead-bonus items target one of them)
    pub race_code: i32, pub animation: Option<String>, pub tower: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum SkillKind { Active, Passive }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ValueKind { Flat, Percent }
/// raw target 0..=4
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum SkillTarget { Caster, Ally, Enemy, Dead, Summon }

#[derive(Debug, Clone, Deserialize)]
pub struct SkillAttr {
    pub effect: SkillEffect, pub a: Option<f32>, pub prop: Option<f32>, pub data: Option<f32>,
    pub value_kind: ValueKind, pub follow_level: bool, pub time_ms: i32,
    pub width: Option<f32>, pub height: Option<f32>, pub target: SkillTarget, pub begin_pos_code: i32,
}
#[derive(Debug, Clone, Deserialize)]
pub struct SkillVisual {
    pub name: String, pub init_index: i32, pub action_type: i32, pub move_type: i32, pub action_index: i32,
    pub time_ms: Option<i32>, pub number: i32, pub delta: i32, pub pos: i32,
    pub begin_sound: String, pub action_sound: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Skill {
    /// English (official 3.4.3 text); the Chinese original is kept in *_zh
    pub index: i32, pub name: String, pub introduction: String,
    #[serde(default)] pub name_zh: String, #[serde(default)] pub introduction_zh: String,
    pub range_code: i32, pub radius_code: i32, pub kind: SkillKind,
    /// cooldown in ms; None where the game data leaves it empty
    pub cd_ms: Option<i32>, pub icon: String, pub anim: String, pub count: i32,
    pub attrs: Vec<SkillAttr>, pub effects: Vec<SkillVisual>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum EquipSlot { Weapon, Clothes, Hat, Shoes, Jewelry }
/// type_code 0..=13, named from item names. Uncertain: Longsword (4) vs Sword (2), HeavyAxe (8), Fist (9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum WeaponType { Club, Dagger, Sword, Scimitar, Longsword, Axe, Mace, TwoHanded, HeavyAxe, Fist, SmallShield, TowerShield, Bow, Staff }
/// Clothes 0/1/2 = Robe/Leather/Plate; Hat and Shoes 0/1 = Light/Heavy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ArmorClass { Robe, Leather, Plate, Light, Heavy }
#[derive(Debug, Clone, Deserialize)]
pub struct EquipAttr { pub effect: EquipEffect, pub chance_pct: i32, pub data: i32, pub time_ms: Option<i32> }
#[derive(Debug, Clone, Deserialize)]
pub struct Equipment {
    pub index: i32, pub slot: EquipSlot, pub name: String, pub intro: String, pub level: i32,
    #[serde(default)] pub name_zh: String, #[serde(default)] pub intro_zh: String,
    pub type_code: Option<i32>, pub weapon_type: Option<WeaponType>, pub armor_class: Option<ArmorClass>,
    /// row in WeaponUpgradeTable.growth / .cost
    pub growth_row: i32, pub cost_row: i32, pub price: i32, pub tex_index: i32,
    pub min_attack: Option<i32>, pub max_attack: Option<i32>, pub ac: Option<i32>, pub sr: Option<i32>,
    /// battleType 0/1/2, meaning unconfirmed
    pub battle_type_code: i32, pub pvp_medal: Option<i32>, pub pvp_score: Option<i32>,
    pub attrs: Vec<EquipAttr>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GrowthRow { pub index: i32, pub min_damage: Option<f32>, pub max_damage: Option<f32>, pub defense: Option<f32> }
/// cost of upgrade step k = a*k^2 + b*k + c
#[derive(Debug, Clone, Deserialize)]
pub struct CostRow { pub index: i32, pub formula: Formula }

#[derive(Debug, Clone, Deserialize)]
pub struct Spawn {
    /// one enemy picked from this list (usually one entry)
    pub enemy_choices: Vec<i32>, pub level: i32,
    /// spawns once this many of the team are dead, then waits delay_ms
    pub after_dead: i32, pub delay_ms: i32, pub x: i32,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Team { pub index: i32, pub spawns: Vec<Spawn> }
#[derive(Debug, Clone, Deserialize)]
pub struct MapWaves { pub map_tex_choices: Vec<i32>, pub has_fall_hero: bool, pub teams: Vec<Team> }
#[derive(Debug, Clone, Deserialize)]
pub struct CastlePrices {
    pub refresh_shop: i32, pub challenge_now: i32, pub refresh_arena: i32,
    pub iap_99_cents: i32, pub iap_499_cents: i32, pub iap_1999_cents: i32, pub iap_4999_cents: i32, pub iap_9999_cents: i32,
}

// ---- root tables (one per .ron file) ----
#[cfg_attr(feature = "bevy", derive(bevy::asset::Asset, bevy::reflect::TypePath))]
#[derive(Debug, Clone, Deserialize)] pub struct HeroTable { pub heroes: Vec<Hero> }
#[cfg_attr(feature = "bevy", derive(bevy::asset::Asset, bevy::reflect::TypePath))]
#[derive(Debug, Clone, Deserialize)] pub struct EnemyTable { pub enemies: Vec<Enemy> }
#[cfg_attr(feature = "bevy", derive(bevy::asset::Asset, bevy::reflect::TypePath))]
#[derive(Debug, Clone, Deserialize)] pub struct SkillTable { pub skills: Vec<Skill> }
#[cfg_attr(feature = "bevy", derive(bevy::asset::Asset, bevy::reflect::TypePath))]
#[derive(Debug, Clone, Deserialize)] pub struct EquipmentTable { pub items: Vec<Equipment> }
#[cfg_attr(feature = "bevy", derive(bevy::asset::Asset, bevy::reflect::TypePath))]
#[derive(Debug, Clone, Deserialize)] pub struct WeaponUpgradeTable { pub growth: Vec<GrowthRow>, pub cost: Vec<CostRow> }
#[cfg_attr(feature = "bevy", derive(bevy::asset::Asset, bevy::reflect::TypePath))]
#[derive(Debug, Clone, Deserialize)] pub struct WaveTable { pub maps: BTreeMap<u32, MapWaves> }
#[cfg_attr(feature = "bevy", derive(bevy::asset::Asset, bevy::reflect::TypePath))]
#[derive(Debug, Clone, Deserialize)] pub struct MoneyConfig { pub boss_castle: BTreeMap<u32, CastlePrices> }

/// Skill attr effect IDs (raw `name` in SkillList.xml)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum SkillEffect {
    /// raw `addac`
    Addac,
    /// raw `addac_base`
    AddacBase,
    /// raw `addac_height`
    AddacHeight,
    /// raw `addac_time`
    AddacTime,
    /// raw `addac_width`
    AddacWidth,
    /// raw `addacheal`
    Addacheal,
    /// raw `addcritdamage_axe_base`
    AddcritdamageAxeBase,
    /// raw `addcritdamage_base`
    AddcritdamageBase,
    /// raw `addcritdamage_mace_base`
    AddcritdamageMaceBase,
    /// raw `addcritdamage_sword_base`
    AddcritdamageSwordBase,
    /// raw `addcritdamage_twohanded_base`
    AddcritdamageTwohandedBase,
    /// raw `addcritrate`
    Addcritrate,
    /// raw `addcritrate_axe_base`
    AddcritrateAxeBase,
    /// raw `addcritrate_mace_base`
    AddcritrateMaceBase,
    /// raw `addcritrate_sword_base`
    AddcritrateSwordBase,
    /// raw `addcritrate_twohanded_base`
    AddcritrateTwohandedBase,
    /// raw `adddamage`
    Adddamage,
    /// raw `adddamage_axe_base`
    AdddamageAxeBase,
    /// raw `adddamage_base`
    AdddamageBase,
    /// raw `adddamage_club_base`
    AdddamageClubBase,
    /// raw `adddamage_dagger_base`
    AdddamageDaggerBase,
    /// raw `adddamage_height`
    AdddamageHeight,
    /// raw `adddamage_mace_base`
    AdddamageMaceBase,
    /// raw `adddamage_scimitar_base`
    AdddamageScimitarBase,
    /// raw `adddamage_sword_base`
    AdddamageSwordBase,
    /// raw `adddamage_twohanded_base`
    AdddamageTwohandedBase,
    /// raw `adddamage_width`
    AdddamageWidth,
    /// raw `adddhitrate_club_base`
    AdddhitrateClubBase,
    /// raw `adddhitrate_dagger_base`
    AdddhitrateDaggerBase,
    /// raw `adddhitrate_scimitar_base`
    AdddhitrateScimitarBase,
    /// raw `addhitrate_axe_base`
    AddhitrateAxeBase,
    /// raw `addhitrate_base`
    AddhitrateBase,
    /// raw `addhitrate_club_base`
    AddhitrateClubBase,
    /// raw `addhitrate_dagger_base`
    AddhitrateDaggerBase,
    /// raw `addhitrate_mace_base`
    AddhitrateMaceBase,
    /// raw `addhitrate_scimitar_base`
    AddhitrateScimitarBase,
    /// raw `addhitrate_sword_base`
    AddhitrateSwordBase,
    /// raw `addhitrate_twohanded_base`
    AddhitrateTwohandedBase,
    /// raw `addmaxhp`
    Addmaxhp,
    /// raw `addmaxhp_base`
    AddmaxhpBase,
    /// raw `addoffhanddamage`
    Addoffhanddamage,
    /// raw `animatedead`
    Animatedead,
    /// raw `animatedead_time`
    AnimatedeadTime,
    /// raw `attackspeed`
    Attackspeed,
    /// raw `attackspeed_base`
    AttackspeedBase,
    /// raw `attracthatred`
    Attracthatred,
    /// raw `avoiddamage`
    Avoiddamage,
    /// raw `betterprice`
    Betterprice,
    /// raw `CD_time`
    CDTime,
    /// raw `changefirearrow`
    Changefirearrow,
    /// raw `changefirearrow_height`
    ChangefirearrowHeight,
    /// raw `changefirearrow_width`
    ChangefirearrowWidth,
    /// raw `changefireexplosive`
    Changefireexplosive,
    /// raw `changefireexplosive_height`
    ChangefireexplosiveHeight,
    /// raw `changefireexplosive_width`
    ChangefireexplosiveWidth,
    /// raw `changemagicarrow`
    Changemagicarrow,
    /// raw `damage_cold`
    DamageCold,
    /// raw `damage_fire`
    DamageFire,
    /// raw `damage_fire_height`
    DamageFireHeight,
    /// raw `damage_fire_width`
    DamageFireWidth,
    /// raw `damage_light`
    DamageLight,
    /// raw `damage_physics`
    DamagePhysics,
    /// raw `damage_poison`
    DamagePoison,
    /// raw `damagetransfer`
    Damagetransfer,
    /// raw `dispel`
    Dispel,
    /// raw `dizzy_held`
    DizzyHeld,
    /// raw `doubleanimate`
    Doubleanimate,
    /// raw `effect_time`
    EffectTime,
    /// raw `fireexplosive`
    Fireexplosive,
    /// raw `firesheild`
    Firesheild,
    /// raw `frozen`
    Frozen,
    /// raw `holy_held`
    HolyHeld,
    /// raw `holy_held_height`
    HolyHeldHeight,
    /// raw `holy_held_width`
    HolyHeldWidth,
    /// raw `holy_kill`
    HolyKill,
    /// raw `humaninveterate`
    Humaninveterate,
    /// raw `inveteratekiller`
    Inveteratekiller,
    /// raw `invisible`
    Invisible,
    /// raw `kill`
    Kill,
    /// raw `normaldamage`
    Normaldamage,
    /// raw `ogresinveterate`
    Ogresinveterate,
    /// raw `onfire`
    Onfire,
    /// raw `orcinveterate`
    Orcinveterate,
    /// raw `organisminveterate`
    Organisminveterate,
    /// raw `powerofgiant`
    Powerofgiant,
    /// raw `recoverHP`
    RecoverHP,
    /// raw `recoverHPslow`
    RecoverHPslow,
    /// raw `resist_cold`
    ResistCold,
    /// raw `resist_damage`
    ResistDamage,
    /// raw `resist_fire`
    ResistFire,
    /// raw `resist_physics`
    ResistPhysics,
    /// raw `resist_poison`
    ResistPoison,
    /// raw `resurrection`
    Resurrection,
    /// raw `rightweapon`
    Rightweapon,
    /// raw `rightweaponDefense`
    RightweaponDefense,
    /// raw `sacrifice`
    Sacrifice,
    /// raw `selfheal`
    Selfheal,
    /// raw `skillCD`
    SkillCD,
    /// raw `skillCDto0`
    SkillCDto0,
    /// raw `special_held`
    SpecialHeld,
    /// raw `speed`
    Speed,
    /// raw `speed_base`
    SpeedBase,
    /// raw `spiritofhero`
    Spiritofhero,
    /// raw `spiritofhero_addac`
    SpiritofheroAddac,
    /// raw `spiritofhero_adddamage`
    SpiritofheroAdddamage,
    /// raw `spiritofhero_resist_damage`
    SpiritofheroResistDamage,
    /// raw `stumble`
    Stumble,
    /// raw `stumble_sleep`
    StumbleSleep,
    /// raw `stumble_time`
    StumbleTime,
    /// raw `summonskill`
    Summonskill,
    /// raw `towershield`
    Towershield,
    /// raw `transferbound`
    Transferbound,
    /// raw `undeadinveterate`
    Undeadinveterate,
}

/// Equipment attr effect IDs (raw `type` in AllEquipped.xml)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum EquipEffect {
    /// raw `addac`
    Addac,
    /// raw `addac_enemy`
    AddacEnemy,
    /// raw `addcritdamage`
    Addcritdamage,
    /// raw `addcritrate`
    Addcritrate,
    /// raw `adddamage`
    Adddamage,
    /// raw `adddamage_percent`
    AdddamagePercent,
    /// raw `adddamage_undead`
    AdddamageUndead,
    /// raw `adddamage_undead_percent`
    AdddamageUndeadPercent,
    /// raw `addhitrate`
    Addhitrate,
    /// raw `addmaxhp`
    Addmaxhp,
    /// raw `attackspeed`
    Attackspeed,
    /// raw `avoiddamage`
    Avoiddamage,
    /// raw `avoidrangeddamage`
    Avoidrangeddamage,
    /// raw `bloodsuck`
    Bloodsuck,
    /// raw `damage_cold`
    DamageCold,
    /// raw `damage_fire`
    DamageFire,
    /// raw `damage_fire_SwordPower`
    DamageFireSwordPower,
    /// raw `damage_ice_SwordPower`
    DamageIceSwordPower,
    /// raw `damage_physics`
    DamagePhysics,
    /// raw `damage_poison`
    DamagePoison,
    /// raw `Deviltongue_state_dizzy`
    DeviltongueStateDizzy,
    /// raw `dizzy_held`
    DizzyHeld,
    /// raw `firesheild`
    Firesheild,
    /// raw `HPrecover`
    HPrecover,
    /// raw `immune_held`
    ImmuneHeld,
    /// raw `immune_kill`
    ImmuneKill,
    /// raw `immune_stumble`
    ImmuneStumble,
    /// raw `kill`
    Kill,
    /// raw `resist_cold`
    ResistCold,
    /// raw `resist_damage`
    ResistDamage,
    /// raw `resist_fire`
    ResistFire,
    /// raw `resist_physics`
    ResistPhysics,
    /// raw `resist_poison`
    ResistPoison,
    /// raw `speed`
    Speed,
    /// raw `stumble`
    Stumble,
    /// raw `toughnessvalue`
    Toughnessvalue,
}
