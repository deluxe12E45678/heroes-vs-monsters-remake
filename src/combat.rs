//! Pure combat math (no Bevy types), so it can be unit-tested headlessly.
//! Every formula cites /workspace/hvsm_reference.md (section numbers in brackets).
use crate::types::{BaseStats, Formula, Formulas};

/// A combat-ready stat block for one unit at a given level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fighter {
    pub max_hp: f32,
    pub attack: i32,
    pub ac: i32,
    pub sr: i32,
    pub hit_rate: i32,
    /// Seconds between swings: AttackSpeed/1000 + F_AttSpd(L-1)/1000 [§2].
    pub attack_interval: f32,
    pub move_speed: f32,
    /// CritRare, in percent [§0, 0xf0].
    pub crit_rate: i32,
    /// addcritdamage bonus (0 = plain ×2 crit) [§0, 0xf8].
    pub crit_damage: f32,
    /// ACResistance = resist_physics, a flat subtraction [§0, 0x100; §1.3].
    pub resist_physics: i32,
    /// SRResistance, used for magic hits [§0, 0x108; inferred].
    pub resist_magic: i32,
    /// Weapon dice (minDamage..=maxDamage) [§1.2].
    pub weapon_min: i32,
    pub weapon_max: i32,
}

/// getFloatFromFormula [§2]: 0 at x=0, else a·x²+b·x+c.
/// x = displayed level − 1 (heroes use level−1; enemies use (L·8−8)·0.125, the same value) [§12.8].
pub fn eval_formula(f: Option<Formula>, x: f32) -> f32 {
    match f {
        Some(f) if x != 0.0 => f.a * x * x + f.b * x + f.c,
        _ => 0.0,
    }
}

/// GORoleCB::BuildRole [§2]: MaxHP/Attack/HitRate get int(F(L−1)) added; the attack interval gets F_AttSpd/1000;
/// AC, SR, crit, resists and move speed are flat XML values.
pub fn build_fighter(s: &BaseStats, f: &Formulas, level: i32, weapon: (i32, i32)) -> Fighter {
    let x = (level - 1).max(0) as f32;
    Fighter {
        max_hp: (s.hp + eval_formula(f.hp, x) as i32) as f32,
        attack: s.attack + eval_formula(f.attack, x) as i32,
        ac: s.ac,
        sr: s.sr,
        hit_rate: s.hit_rate + eval_formula(f.hit_rate, x) as i32,
        attack_interval: ((s.attack_speed_ms as f32 + eval_formula(f.attack_speed, x)) / 1000.0).max(0.2),
        move_speed: s.move_speed as f32,
        crit_rate: s.crit_rate,
        crit_damage: 0.0,
        resist_physics: s.ac_resistance,
        resist_magic: s.sr_resistance,
        weapon_min: weapon.0,
        weapon_max: weapon.1,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Sr is used by staff projectiles in the original game
pub enum DefenseKind { Ac, Sr }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitResult { Miss, Hit }

/// RoleAttribute::CalculateHitRate / CalculateBlock [§1.1]:
/// d = rand()%20; d==0 → auto-miss, d==19 → auto-hit; otherwise HIT iff HitRate+d > AC+bonus (or > SR).
pub fn hit_roll_with(d: i32, hit_rate: i32, def: &Fighter, kind: DefenseKind, ac_bonus: i32) -> HitResult {
    if d == 0 { return HitResult::Miss; }
    if d == 19 { return HitResult::Hit; }
    let roll = hit_rate + d;
    let hit = match kind {
        DefenseKind::Ac => roll > def.ac + ac_bonus,
        DefenseKind::Sr => roll > def.sr,
    };
    if hit { HitResult::Hit } else { HitResult::Miss }
}

pub fn hit_roll(rng: &mut fastrand::Rng, atk: &Fighter, def: &Fighter, kind: DefenseKind) -> HitResult {
    hit_roll_with(rng.i32(0..20), atk.hit_rate, def, kind, 0)
}

/// Outgoing damage [§1.2]: dmg = Attack + (min + rand%(max−min+1));
/// CRIT iff CritRate > rand%100, then dmg ×= 2·(1 + CritDamage).
pub fn damage_with(atk: &Fighter, dice: i32, crit_d100: i32) -> (f32, bool) {
    let span = (atk.weapon_max - atk.weapon_min + 1).max(1);
    let mut dmg = (atk.attack + atk.weapon_min + dice.rem_euclid(span)) as f32;
    let crit = atk.crit_rate > crit_d100;
    if crit { dmg *= 2.0 * (1.0 + atk.crit_damage); }
    (dmg, crit)
}

pub fn roll_damage(rng: &mut fastrand::Rng, atk: &Fighter) -> (f32, bool) {
    damage_with(atk, rng.i32(0..i32::MAX), rng.i32(0..100))
}

/// Incoming damage [§1.3]: main −= resist[type]; main −= resist_damage_flat; main −= main·pct; clamped to ≥0.
pub fn mitigate(dmg: f32, resist: i32, resist_flat: i32, pct: f32) -> f32 {
    let mut m = dmg - resist as f32 - resist_flat as f32;
    if pct > 0.0 { m -= m * pct; }
    m.max(0.0)
}

/// Max melee attackers locked on one target [§0, 0x424..0x438: 6 attacker slots].
#[allow(dead_code)]
pub const MAX_ATTACKERS: usize = 6;

#[cfg(test)]
mod tests {
    use super::*;
    fn f(attack: i32, ac: i32, hit: i32, crit: i32) -> Fighter {
        Fighter { max_hp: 50.0, attack, ac, sr: 0, hit_rate: hit, attack_interval: 2.0, move_speed: 90.0,
            crit_rate: crit, crit_damage: 0.0, resist_physics: 0, resist_magic: 0, weapon_min: 1, weapon_max: 8 }
    }
    #[test] fn natural_rolls() {
        let d = f(2, 99, 0, 0);
        assert_eq!(hit_roll_with(0, 1000, &d, DefenseKind::Ac, 0), HitResult::Miss); // 0 always misses
        assert_eq!(hit_roll_with(19, -1000, &d, DefenseKind::Ac, 0), HitResult::Hit); // 19 always hits
    }
    #[test] fn ac_threshold_is_strict() {
        let d = f(2, 5, 0, 0);
        assert_eq!(hit_roll_with(3, 2, &d, DefenseKind::Ac, 0), HitResult::Miss); // 5 > 5 is false
        assert_eq!(hit_roll_with(4, 2, &d, DefenseKind::Ac, 0), HitResult::Hit);
        assert_eq!(hit_roll_with(4, 2, &d, DefenseKind::Ac, 1), HitResult::Miss); // block bonus
        assert_eq!(hit_roll_with(1, 0, &d, DefenseKind::Sr, 0), HitResult::Hit); // SR 0
    }
    #[test] fn damage_and_crit() {
        let a = f(2, 0, 0, 5);
        assert_eq!(damage_with(&a, 0, 50), (3.0, false));  // 2 + 1
        assert_eq!(damage_with(&a, 7, 50), (10.0, false)); // 2 + 8
        assert_eq!(damage_with(&a, 0, 4), (6.0, true));    // crit ×2
        let mut b = a; b.crit_damage = 0.1;
        assert!((damage_with(&b, 0, 0).0 - 6.6).abs() < 1e-4); // ×2·1.1
    }
    #[test] fn mitigation() {
        assert_eq!(mitigate(10.0, 3, 0, 0.0), 7.0);
        assert_eq!(mitigate(10.0, 0, 0, 0.5), 5.0);
        assert_eq!(mitigate(2.0, 5, 0, 0.0), 0.0);
    }
    #[test] fn formulas_from_data() {
        let d = crate::data::GameData::load().expect("load ron");
        let w = d.hero(0);
        let l1 = build_fighter(&w.stats, &w.formulas, 1, (1, 8));
        assert_eq!((l1.max_hp, l1.attack, l1.attack_interval), (50.0, 2, 2.0));
        let l5 = build_fighter(&w.stats, &w.formulas, 5, (1, 8));
        assert_eq!(l5.max_hp, 50.0 + 12.0 * 4.0); // HPFormula b=12
        let e = d.enemy(0);
        assert_eq!(build_fighter(&e.stats, &e.formulas, 2, (0, 0)).max_hp, 30.0); // 15 + 15·1
    }
    #[test] fn statistical_hit_rate() {
        // Warrior (Hit 2) vs AC 5: hits on d ∈ 4..=19 → 16/20 = 80%.
        let mut rng = fastrand::Rng::with_seed(7);
        let (a, d) = (f(2, 0, 2, 0), f(2, 5, 0, 0));
        let hits = (0..20000).filter(|_| hit_roll(&mut rng, &a, &d, DefenseKind::Ac) == HitResult::Hit).count();
        assert!((hits as f32 / 20000.0 - 0.80).abs() < 0.02, "{hits}");
    }
}
