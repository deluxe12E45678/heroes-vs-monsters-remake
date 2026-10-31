//! Builds a hero's battle stats from save data: level formulas [§2], equipment (equipment.ron)
//! and learned passive skills (PassiveSkill Mode B [§3]).
use crate::{combat::*, data::GameData, save::OwnedHero, types::*};

/// The skill-tree node is the level-1 skill; each extra point uses the next skill index (e.g. 101..105).
pub fn skill_at_points(data: &GameData, base: i32, points: i32) -> Option<&Skill> {
    let b = data.skill(base)?;
    let i = base + points.max(1) - 1;
    data.skill(i).filter(|s| s.name == b.name).or(Some(b))
}

/// [§3] v = isPct ? data/100 : data; if follow_level: v = (L8/8 + 1)·v + base, with L8 = level·8.
pub fn attr_value(a: &SkillAttr, level: i32) -> f32 {
    let mut v = a.data.unwrap_or(0.0);
    if a.value_kind == ValueKind::Percent { v /= 100.0; }
    if a.follow_level { v = (level as f32 + 1.0) * v + a.a.unwrap_or(0.0); }
    v
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Extra { pub avoid_pct: i32, pub resist_fire: i32, pub resist_cold: i32, pub resist_flat: i32 }

pub fn hero_fighter(data: &GameData, oh: &OwnedHero) -> (Fighter, Extra) { hero_fighter_with(data, oh, true, true) }

/// Stats with or without equipment / learned passives (the equip page's Base / Equipment / Perks breakdown).
pub fn hero_fighter_with(data: &GameData, oh: &OwnedHero, use_equip: bool, use_perks: bool) -> (Fighter, Extra) {
    let h = data.hero(oh.hero);
    // weapon dice: right hand, else left hand, else bare fists 1..2
    let w = oh.equip[3].or(oh.equip[2]).and_then(|i| data.item(i)).filter(|_| use_equip);
    let dice = w.map(|w| (w.min_attack.unwrap_or(0), w.max_attack.unwrap_or(0))).unwrap_or((1, 2));
    let mut f = build_fighter(&h.stats, &h.formulas, oh.level, dice);
    let mut ex = Extra::default();
    let (mut hp_pct, mut atk_pct) = (0.0f32, 0.0f32);
    // --- equipment (AllEquipped attr: prop = chance %, data = value) ---
    for item in oh.equip.iter().flatten().filter(|_| use_equip).filter_map(|i| data.item(*i)) {
        f.ac += item.ac.unwrap_or(0);
        f.sr += item.sr.unwrap_or(0);
        for a in &item.attrs {
            let v = a.data;
            match a.effect {
                EquipEffect::Addmaxhp => f.max_hp += v as f32,
                EquipEffect::Adddamage => f.attack += v,
                EquipEffect::AdddamagePercent => atk_pct += v as f32 / 100.0,
                EquipEffect::Addhitrate => f.hit_rate += v,
                EquipEffect::Addcritrate => f.crit_rate += v,
                EquipEffect::Addcritdamage => f.crit_damage += v as f32 / 100.0,
                EquipEffect::Addac => f.ac += v,
                EquipEffect::ResistPhysics => f.resist_physics += v,
                EquipEffect::ResistFire => ex.resist_fire += v,
                EquipEffect::ResistCold => ex.resist_cold += v,
                EquipEffect::ResistDamage => ex.resist_flat += v,
                EquipEffect::Avoiddamage => ex.avoid_pct += a.chance_pct,
                // element damage that always procs (prop 100) is folded into Attack (approximation)
                EquipEffect::DamageFire | EquipEffect::DamageCold | EquipEffect::DamagePoison | EquipEffect::DamagePhysics
                    if a.chance_pct >= 100 => f.attack += v,
                EquipEffect::Speed => f.move_speed += v as f32,
                _ => {}
            }
        }
    }
    // --- learned passives (Mode B self stats) ---
    for (&node, &pts) in &oh.skills {
        if pts <= 0 || !use_perks { continue; }
        let Some(s) = skill_at_points(data, node, pts) else { continue };
        if s.kind != SkillKind::Passive { continue; }
        for a in &s.attrs {
            let v = attr_value(a, oh.level);
            let pct = a.value_kind == ValueKind::Percent;
            match a.effect {
                SkillEffect::AddmaxhpBase | SkillEffect::Addmaxhp => if pct { hp_pct += v } else { f.max_hp += v },
                SkillEffect::AdddamageBase => if pct { atk_pct += v } else { f.attack += v as i32 },
                SkillEffect::AddacBase => if pct { f.ac += (f.ac as f32 * v) as i32 } else { f.ac += v as i32 },
                SkillEffect::AddhitrateBase => if pct { f.hit_rate += (f.hit_rate as f32 * v).round() as i32 } else { f.hit_rate += v as i32 },
                SkillEffect::AddcritdamageBase => f.crit_damage += if pct { v } else { v / 100.0 },
                SkillEffect::AttackspeedBase => f.attack_interval = (f.attack_interval - if pct { v * f.attack_interval } else { v / 1000.0 }).max(0.3),
                SkillEffect::SpeedBase => f.move_speed += if pct { f.move_speed * v } else { v },
                SkillEffect::Avoiddamage => ex.avoid_pct += (if pct { v * 100.0 } else { v }) as i32,
                SkillEffect::ResistDamage => ex.resist_flat += v as i32,
                SkillEffect::ResistFire => ex.resist_fire += v as i32,
                SkillEffect::ResistCold => ex.resist_cold += v as i32,
                SkillEffect::ResistPhysics => f.resist_physics += v as i32,
                _ => {}
            }
        }
    }
    f.max_hp *= 1.0 + hp_pct;
    f.attack = (f.attack as f32 * (1.0 + atk_pct)).round() as i32;
    (f, ex)
}

/// Skill points available = hero level − points spent (1 point per level).
pub fn points_left(oh: &OwnedHero) -> i32 { oh.level - oh.skills.values().sum::<i32>() }
/// [§2] XP needed for the next level = F_Exp(level) with the hero's ExpFormula.
pub fn exp_to_next(data: &GameData, oh: &OwnedHero) -> i64 { eval_formula(data.hero(oh.hero).formulas.exp, oh.level as f32).max(10.0) as i64 }

/// Which save slot an item may go into (helm, armor, L hand, R hand, ring L, ring R, boots).
pub fn slots_for(item: &Equipment) -> &'static [usize] {
    match item.slot {
        EquipSlot::Hat => &[0],
        EquipSlot::Clothes => &[1],
        EquipSlot::Weapon => match item.weapon_type {
            Some(WeaponType::SmallShield) | Some(WeaponType::TowerShield) => &[2],
            Some(WeaponType::Dagger) | Some(WeaponType::Scimitar) | Some(WeaponType::Sword) | Some(WeaponType::Club) => &[3, 2],
            _ => &[3],
        },
        EquipSlot::Jewelry => &[4, 5],
        EquipSlot::Shoes => &[6],
    }
}
/// Hero class restrictions from heroes.ron (weapon_types / clothes_types / hat_types / shoe_types).
pub fn hero_can_use(h: &Hero, item: &Equipment) -> bool {
    let t = item.type_code.unwrap_or(0);
    match item.slot {
        EquipSlot::Weapon => h.weapon_types.contains(&t),
        EquipSlot::Clothes => h.clothes_types.contains(&t),
        EquipSlot::Hat => h.hat_types.contains(&t),
        EquipSlot::Shoes => h.shoe_types.contains(&t),
        EquipSlot::Jewelry => true,
    }
}
