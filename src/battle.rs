//! Battle (SceneMainGameMachine): map_NN.jpeg background, the unit sheets moving freely in 2D, green HP bars,
//! glowing selection ring (guangquan), pause button (paused) top-right, portrait box top-left, wave director
//! from waves.ron (teams → spawns with after_dead/delay), kill rewards per the economy formulas.
//! Controls: click/1-3 select (drag = box select), click/right-click ground = move, click enemy = attack it,
//! Q/W/E or the skill buttons = cast, A = auto-battle, P/pause button = pause, R = restart, Esc = retreat.
use crate::{anim::*, combat::*, data::GameData, save::Save, stats, types::*, ui::*, AppState};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.init_resource::<Run>().init_resource::<NewHeroNotice>()
        .add_systems(OnEnter(AppState::Battle), enter)
        .init_resource::<AutoAim>().add_systems(Update, (input, director, tutorial, ai, projectiles, effects, order_marks, hud, end_check, outro, outro_hide).chain().run_if(in_state(AppState::Battle)))
        .add_systems(OnExit(AppState::Battle), (despawn_screen, unpause));
}

#[derive(Resource, Default, Clone)]
pub struct BattleSetup { pub node: u32, pub map: u32, pub level: (i32, i32), pub boss: bool, pub arena: Option<usize>, pub tutorial: bool, pub siege: Option<usize>, pub survival: bool }

const FIELD_MIN: Vec2 = Vec2::new(-440.0, -250.0);
const FIELD_MAX: Vec2 = Vec2::new(440.0, 110.0);
const SCALE: f32 = 1.6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side { Hero, Enemy }

#[derive(Clone)]
pub struct ActiveSkill { pub skill: Skill, pub cd: f32, pub left: f32 }

#[derive(Component)]
pub struct Unit {
    pub side: Side, pub f: Fighter, pub ex: stats::Extra, pub hp: f32, pub atk_cd: f32,
    pub target: Option<Entity>, pub forced: Option<Entity>, pub move_to: Option<Vec2>,
    pub roster: Option<usize>, pub slot: usize, pub enemy: i32, pub level: i32,
    pub ranged: bool, pub healer: bool, pub range: f32, pub hatred: i32,
    pub taunted_by: Option<(Entity, f32)>, pub buff: (i32, i32, f32), // (ac, attack, time)
    pub skills: Vec<ActiveSkill>, pub dead: bool, pub dead_t: f32, pub team: usize,
    pub st: Status,
    /// seconds from attack start to the release/impact frame
    pub hit_t: f32,
    /// TriggerRect (left, right, down, up) an idle hero scans in manual mode (GOHero::OnUpdate)
    pub trig: Vec4,
}
/// Timed states set by skill attrs (SkillList.xml effect types).
#[derive(Clone, Copy, Default, Debug)]
pub struct Status {
    /// DizzyHeld / Stumble / StumbleSleep / Frozen / HolyHeld / SpecialHeld: cannot act
    pub stun: f32,
    /// Onfire / DamagePoison over time: (damage per second, seconds left)
    pub dot: (f32, f32),
    /// RecoverHPslow: (heal per second, seconds left)
    pub hot: (f32, f32),
    /// ResistDamage / Avoiddamage: (fraction of damage removed, seconds left)
    pub reduce: (f32, f32),
    /// Invisible: enemies ignore the unit
    pub invis: f32,
    /// Animatedead summons: seconds left to live (0 = permanent unit)
    pub life: f32,
    /// timed Adddamage / Addac / Attackspeed / Addmaxhp skill attrs: (amount, seconds left).
    /// atk = attack points added, ac = armour added, aspd = fraction added to the swing interval, hpmax = max hp added
    pub atk: (f32, f32), pub ac: (f32, f32), pub aspd: (f32, f32), pub hpmax: (f32, f32),
    /// Shield Wall (ResistPhysics/Fire/Cold/Poison together): takes no damage
    pub immune: f32,
    /// Flaming Arrows (Changefirearrow): (fire damage, box w, box h, seconds left) added to each arrow
    pub fire_arrow: (f32, f32, f32, f32),
}
#[derive(Component)]
struct Selected;
#[derive(Component)]
struct Ring(Entity);
#[derive(Component)]
struct HpFill { unit: Entity, w: f32 }
#[derive(Component)]
struct Projectile { from: Entity, to: Entity, speed: f32, heal: bool, dmg: f32, delay: f32 }
#[derive(Component)]
struct Floating(f32);
#[derive(Component)]
struct TutText;
#[derive(Component)]
struct TutPanel;
#[derive(Component)]
struct CdOverlay(usize);
#[derive(Component)]
struct PausedOverlay;
#[derive(Component)]
struct PauseTick(usize);
#[derive(Component)]
struct AutoText;
/// HeroAttribute trigger_rect half-width 80 (most melee heroes) in 2x coordinates: melee heroes not on auto only engage foes this close
#[derive(Component)]
struct SelIcon(usize, String);
#[derive(Component)]
struct SkillFrame(usize);
#[derive(Component)]
struct WaveHud;
#[derive(Component)]
struct PartyHud(usize); // roster index into party display rows
#[derive(Component)]
struct PartyHpFill(usize);
#[derive(Component)]
struct BoxSel;

#[derive(Resource, Default)]
pub struct Run {
    pub team: usize, pub spawned: Vec<bool>, pub timers: Vec<f32>, pub dead_in_team: usize,
    pub auto: bool, pub exp: i64, pub money: i64, pub done: bool, pub won: bool,
    /// tutorial step (SceneHelpGame states) and time in step
    pub tut: u8, pub tut_t: f32,
    /// SceneGameOver outro played over the battlefield (normal battles)
    pub outro: Option<Outro>,
    /// Survival: current generated team, enemy level, waves completed, next BOSS team, kills
    pub surv: Option<Team>, pub surv_level: i32, pub surv_waves: i32, pub surv_boss: usize, pub kills: i32,
    /// queued attacks: (seconds to release frame, attacker, target, projectile?, heal?)
    pub pending: Vec<(f32, Entity, Entity, bool, bool)>,
    drag_start: Option<Vec2>, drag_armed: bool, ui_press: bool, rng: Option<fastrand::Rng>, drag_hero: Option<Entity>,
}

fn unpause(mut t: ResMut<Time<Virtual>>) { t.unpause(); t.set_relative_speed(1.0); }

fn sheet_scale(_data: &GameData, _key: &str) -> f32 { SCALE }

fn spawn_unit(cmd: &mut Commands, a: &AssetServer, data: &GameData, sheets: &Sheets, key: &str, pos: Vec2, u: Unit) -> Entity { spawn_unit_look(cmd, a, data, sheets, key, pos, u, None) }
#[allow(clippy::too_many_arguments)]
fn spawn_unit_look(cmd: &mut Commands, a: &AssetServer, data: &GameData, sheets: &Sheets, key: &str, pos: Vec2, mut u: Unit, look: Option<crate::anim::Look>) -> Entity {
    u.hit_t = data.sheets.get(key).map(|s| s.hit as f32 / crate::anim::FPS).unwrap_or(0.5);
    let side = u.side; let u_enemy = u.enemy;
    let e = cmd.spawn((Transform::from_translation(pos.extend(10.0)), Visibility::default(), Scr, u)).id();
    if let Some(l) = look { crate::anim::attach_rig(cmd, e, key, l, sheet_scale(data, key), 0.0); } else { attach_sheet(cmd, data, sheets, e, key, sheet_scale(data, key), 0.0); }
    let h = data.sheets.get(key).map(|s| s.anchor.1 * SCALE).unwrap_or(120.0).clamp(60.0, 200.0);
    // HP bar (HPbar_bg + HPbar_mc / HPbar_enemy)
    let mut bg = Sprite::from_image(a.load("ui/HPbar_bg.png")); bg.custom_size = Some(Vec2::new(64.0, 9.0));
    let b = cmd.spawn((bg, Transform::from_xyz(0.0, h + 8.0, 5.0))).id();
    let mut fg = Sprite::from_image(a.load(if side == Side::Hero { "ui/HPbar_mc.png" } else { "ui/HPbar_enemy.png" }));
    fg.custom_size = Some(Vec2::new(60.0, 6.0));
    let fe = cmd.spawn((fg, Transform::from_xyz(0.0, h + 8.0, 5.1), HpFill { unit: e, w: 60.0 })).id();
    cmd.entity(e).add_children(&[b, fe]);
    if side == Side::Enemy {
        let name = if u_enemy >= 0 { data.enemy(u_enemy).name.clone() } else { String::new() };
        if !name.is_empty() {
            let n = cmd.spawn((Text2d::new(name), TextFont { font: a.load::<Font>("fonts/gilsanub.ttf").into(), font_size: bevy::text::FontSize::Px(13.0), ..default() },
                TextColor(Color::srgb(1.0, 0.85, 0.75)), Transform::from_xyz(0.0, h + 21.0, 5.2))).id();
            cmd.entity(e).add_child(n);
        }
    }
    if side == Side::Hero {
        let mut r = Sprite::from_image(a.load("ui/selected.png")); r.custom_size = Some(Vec2::new(96.0, 46.0));
        let re = cmd.spawn((r, Transform::from_xyz(0.0, 0.0, -1.0), Visibility::Hidden, Ring(e))).id();
        cmd.entity(e).add_child(re);
        // order marker (arrow_attack red under the forced enemy / arrow_move green at the move point), world-space
        let mut m = Sprite::from_image(a.load("ui/arrow_move.png")); m.custom_size = Some(Vec2::new(110.0, 52.0));
        cmd.spawn((m, Transform::from_xyz(0.0, 0.0, 9.0), Visibility::Hidden, Scr, OrderMark(e)));
    }
    e
}

#[derive(Component)]
struct OrderMark(Entity);

/// Where the self-test's `t` step clicks: the body centre of the enemy nearest to the field centre.
#[derive(Resource, Default)]
pub struct AutoAim(pub Option<Vec2>, pub Option<Vec2>);

/// Unit under the cursor: the one whose sprite box contains `p` with the nearest centre.
fn pick(p: Vec2, units: &Query<(Entity, &mut Unit, &Transform)>, side: Side) -> Option<Entity> {
    units.iter().filter(|(_, u, t)| u.side == side && !u.dead && hit_test(p, t))
        .min_by(|a, b| (p - a.2.translation.truncate() - Vec2::Y * 45.0).length_squared().total_cmp(&(p - b.2.translation.truncate() - Vec2::Y * 45.0).length_squared()))
        .map(|x| x.0)
}

fn order_marks(mut aim: ResMut<AutoAim>, a: Res<AssetServer>, units: Query<(&Unit, &Transform), Without<OrderMark>>, sel: Query<(), With<Selected>>,
               mut marks: Query<(&OrderMark, &mut Sprite, &mut Transform, &mut Visibility)>) {
    aim.0 = units.iter().filter(|(u, _)| u.side == Side::Enemy && !u.dead && u.hp > u.f.max_hp * 0.6).map(|(_, t)| t.translation.truncate() + Vec2::Y * 45.0)
        .next();
    aim.1 = units.iter().find(|(u, _)| u.side == Side::Hero && u.slot == 0 && !u.dead).map(|(_, t)| t.translation.truncate() + Vec2::Y * 45.0);
    for (m, mut s, mut t, mut v) in marks.iter_mut() {
        *v = Visibility::Hidden;
        let Ok((u, _)) = units.get(m.0) else { continue };
        if u.dead || sel.get(m.0).is_err() { continue; }
        let (pos, img) = if let Some((_, et)) = u.forced.or(u.target).and_then(|f| units.get(f).ok()).filter(|(eu, _)| !eu.dead) {
            (et.translation.truncate(), if u.healer { "ui/arrow_heal.png" } else { "ui/arrow_attack.png" })
        } else if let Some(mp) = u.move_to { (mp, "ui/arrow_move.png") } else { continue };
        let h: Handle<Image> = a.load(img);
        if s.image != h { s.image = h; }
        t.translation = pos.extend(9.0);
        *v = Visibility::Visible;
    }
}

/// Every learned active skill of the hero, upgrades folded in (SceneMainGameMachine::ChangeSkillIcon fills one button per
/// skill of the selected hero, controls 50+i).
fn hero_actives(data: &GameData, oh: &crate::save::OwnedHero) -> Vec<ActiveSkill> {
    stats::learned_actives(data, oh).into_iter().take(MAX_SKILLS).map(|(s, cd)| ActiveSkill { skill: s, cd, left: 0.0 }).collect()
}
pub const MAX_SKILLS: usize = 4;

/// Self-test hook: HVSM_SKILLS="0,11,5" gives party slot i that skill (level 1 data) with a 3 s cooldown.
fn test_skill(data: &GameData, slot: usize) -> Option<ActiveSkill> {
    let v = std::env::var("HVSM_SKILLS").ok()?;
    let id: i32 = v.split(',').nth(slot)?.trim().parse().ok()?;
    let s = data.skill(id)?.clone();
    Some(ActiveSkill { skill: s, cd: 3.0, left: 0.0 })
}

fn is_ranged(p: Profession, m: AttackMode) -> bool { m == AttackMode::Far || p != Profession::Soldier }

#[allow(clippy::too_many_arguments)]
fn enter(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, data: Res<GameData>, sheets: Res<Sheets>, save: Res<Save>, setup: Res<BattleSetup>, mut run: ResMut<Run>) {
    let mw = data.waves.maps.get(&setup.map).or_else(|| data.waves.maps.values().next()).unwrap();
    let tex = mw.map_tex_choices.first().copied().unwrap_or(0);
    let tex = mw.map_tex_choices.get(fastrand::usize(..mw.map_tex_choices.len().max(1))).copied().unwrap_or(tex);
    let tex = if setup.survival && !data.survival.map_tex.is_empty() { data.survival.map_tex[fastrand::usize(..data.survival.map_tex.len())] } else { tex };
    let path = format!("maps/map_{tex:02}.jpeg");
    let path = if crate::data::asset_exists(&path) && !setup.tutorial { path } else { "maps/map_00.jpeg".into() };
    if setup.siege.is_some() { // GVG battles are fought in front of the castle (castle_background, 1024² sheet, top 960×640)
        let mut s = Sprite::from_image(a.load("ui/castle_background.png")); s.rect = Some(Rect::new(0.0, 0.0, 1024.0, 640.0)); s.custom_size = Some(SCREEN);
        cmd.spawn((s, Transform::default(), Scr));
    } else { image(&mut cmd, &a, &path, Vec2::ZERO, Some(SCREEN), 0.0); }
    *run = Run { auto: true, rng: Some(fastrand::Rng::new()), spawned: vec![false; 64], timers: vec![0.0; 64], ..default() };
    let party: Vec<usize> = if setup.tutorial {
        let w = save.heroes.iter().position(|o| data.hero(o.hero).attack_type != AttackType::Buff).unwrap_or(0);
        let c = save.heroes.iter().position(|o| data.hero(o.hero).attack_type == AttackType::Buff);
        std::iter::once(w).chain(c).collect()
    } else { save.party.clone() };
    if setup.tutorial {
        run.auto = false;
        let p = image(&mut cmd, &a, "ui/description.png", Vec2::new(0.0, 205.0), Some(Vec2::new(760.0, 110.0)), 70.0);
        cmd.entity(p).insert(TutPanel);
        let t = text(&mut cmd, &f, "", Vec2::new(0.0, 205.0), 22.0, Color::WHITE, 71.0);
        cmd.entity(t).insert((TutText, TutPanel));
    }
    for (i, &r) in party.iter().enumerate() {
        let oh = &save.heroes[r];
        let h = data.hero(oh.hero);
        let (ft, ex) = stats::hero_fighter(&data, oh);
        let ranged = is_ranged(h.profession, h.attack_mode);
        let u = Unit { side: Side::Hero, trig: h.trigger_rect.as_ref().map(|r| Vec4::new(r.x, r.y, r.z, r.w)).unwrap_or(Vec4::new(80.0, 80.0, 40.0, 40.0)), f: ft, ex, hp: ft.max_hp, atk_cd: 0.5, target: None, forced: None, move_to: None, roster: Some(r), slot: i, enemy: -1,
            level: oh.level, ranged, healer: h.attack_type == AttackType::Buff, range: if h.attack_type == AttackType::Buff { 330.0 } else if ranged { h.trigger_rect.as_ref().map(|r| r.x * 2.0).unwrap_or(330.0) } else { 70.0 }, hatred: h.hatred,
            taunted_by: None, buff: (0, 0, 0.0), skills: test_skill(&data, i).map(|s| vec![s]).unwrap_or_else(|| hero_actives(&data, oh)), dead: false, dead_t: 0.0, team: 0, st: Status::default(), hit_t: 0.5 };
        let pos = Vec2::new(-330.0 + if ranged { -60.0 } else { 40.0 }, if party.len() > 3 { -20.0 - 72.0 * i as f32 } else { -40.0 - 88.0 * i as f32 });
        let e = spawn_unit_look(&mut cmd, &a, &data, &sheets, &hero_key(&data, oh.hero), pos, u, Some(crate::anim::hero_look(&data, oh.hero, Some(&oh.equip))));
        if i == 0 { cmd.entity(e).insert(Selected); }
        // (the original has no bottom panel: HP bars over heads, the selected hero's skill in the top-left box)
    }
    // pause (top-right) and portrait box (top-left)
    button(&mut cmd, &a, "ui/paused.png", Some("ui/paused_1.png"), Vec2::new(440.0, 280.0), Vec2::new(64.0, 70.0), 60.0, Act::Custom(11, 0));
    // top-left box: the selected hero's skill (StateHelpGame: "Tap on the heal skill icon in the upper left"), tap to cast
    // one box per learned active skill of the selected hero, left to right
    for k in 0..MAX_SKILLS {
        let p = Vec2::new(-430.0 + 88.0 * k as f32, 270.0);
        let fr = image(&mut cmd, &a, "ui/item.png", p, Some(Vec2::splat(86.0)), 60.0);
        cmd.entity(fr).insert(SkillFrame(k));
        let si = button(&mut cmd, &a, "ui/item.png", None, p, Vec2::splat(66.0), 60.5, Act::Custom(12, k as i32));
        cmd.entity(si).insert(SelIcon(k, String::new()));
        let mut ov = Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.65), Vec2::splat(66.0)); ov.custom_size = Some(Vec2::new(66.0, 0.0));
        cmd.spawn((ov, Transform::from_translation(p.extend(61.0)), Scr, CdOverlay(k), SkillFrame(k)));
    }
    // wave / enemy counter (own layout; the original only flashes Wave_Incoming between teams)
    let wh = text(&mut cmd, &f, "Wave 1/1", Vec2::new(0.0, 280.0), 22.0, Color::srgb(1.0, 0.95, 0.75), 61.0);
    cmd.entity(wh).insert(WaveHud);
    // party name + HP strip along the left (own design: original only has over-head HPbars; names help when units stack)
    for (i, &r) in party.iter().enumerate() {
        let oh = &save.heroes[r];
        let name = data.hero(oh.hero).class_name.clone();
        let y = 180.0 - 48.0 * i as f32;
        let label = text(&mut cmd, &f, format!("{name}"), Vec2::new(-430.0, y + 12.0), 16.0, Color::srgb(1.0, 0.95, 0.8), 61.0);
        cmd.entity(label).insert((PartyHud(i), bevy::sprite::Anchor::CENTER_LEFT));
        image(&mut cmd, &a, "ui/HPbar_bg.png", Vec2::new(-370.0, y - 6.0), Some(Vec2::new(120.0, 10.0)), 61.0);
        let mut fill = Sprite::from_image(a.load("ui/HPbar_mc.png"));
        fill.custom_size = Some(Vec2::new(120.0, 10.0));
        cmd.spawn((fill, Transform::from_translation(Vec3::new(-370.0, y - 6.0, 61.5)), Scr, PartyHpFill(i)));
    }
    // battle speed 1×/2×/3× (own addition; remembered in the save), styled as an item frame like the skill box
    image(&mut cmd, &a, "ui/item.png", Vec2::new(360.0, 280.0), Some(Vec2::splat(64.0)), 60.0);
    hotspot(&mut cmd, Vec2::new(360.0, 280.0), Vec2::splat(64.0), 60.5, Act::Custom(13, 0));
    let sp = text(&mut cmd, &f, format!("x{}", save.battle_speed.max(1)), Vec2::new(360.0, 280.0), 26.0, Color::srgb(1.0, 0.85, 0.3), 61.0);
    cmd.entity(sp).insert(AutoText);
    // StateMainGamePause: gamepaused title, music / sound tick boxes, RESUME and RETREAT (CampChallenge::RetreatChallenge)
    let po = cmd.spawn((Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.6), SCREEN * 1.3), Transform::from_xyz(0.0, 0.0, 80.0), Visibility::Hidden, Scr, PausedOverlay)).id();
    let mut kids = vec![image(&mut cmd, &a, "ui/gamepaused.png", Vec2::new(0.0, 170.0), None, 1.0)];
    kids.push(button(&mut cmd, &a, "ui/RESUME.png", Some("ui/RESUME_1.png"), Vec2::new(0.0, 70.0), Vec2::new(254.0, 84.0), 1.0, Act::Custom(14, 0)));
    kids.push(button(&mut cmd, &a, "ui/RETREAT.png", Some("ui/RETREAT_1.png"), Vec2::new(0.0, -20.0), Vec2::new(254.0, 84.0), 1.0, Act::Custom(15, 0)));
    // music (116 px wide) and sound fx (186 px) captions, each followed by its tick box
    for (i, (lbl, on, x, w)) in [("ui/music.png", save.music, -250.0, 116.0), ("ui/sound.png", save.sound, 40.0, 186.0)].into_iter().enumerate() {
        kids.push(image(&mut cmd, &a, lbl, Vec2::new(x + w / 2.0, -120.0), None, 1.0));
        let tx = x + w + 45.0;
        kids.push(button(&mut cmd, &a, "ui/tick_BG.png", None, Vec2::new(tx, -120.0), Vec2::new(50.0, 52.0), 1.0, Act::Custom(16, i as i32)));
        let t = image(&mut cmd, &a, "ui/tick.png", Vec2::new(tx + 4.0, -116.0), None, 1.5);
        cmd.entity(t).insert((PauseTick(i), if on { Visibility::Inherited } else { Visibility::Hidden }));
        kids.push(t);
    }
    if !setup.tutorial { kids.push(text(&mut cmd, &f, "Retreat: leave this battle with no rewards", Vec2::new(0.0, -190.0), 18.0, Color::srgb(1.0, 0.9, 0.7), 1.0)); }
    for k in kids { cmd.entity(po).add_child(k); }
    if !setup.tutorial { // the tutorial has its own instruction panel there
        let wi = image(&mut cmd, &a, "ui/Wave_Incoming.png", Vec2::new(0.0, 160.0), None, 70.0);
        cmd.entity(wi).insert(Floating(-2.0));
    }
    // siege: the castle boss (GVGConfig bossIndex/bossLevel) and 3 guards from the waves of a similar level
    if let Some(ci) = setup.siege {
        if let Some(c) = data.siege.castles.get(ci) {
            let guards: Vec<i32> = data.waves.maps.get(&setup.map).and_then(|mw| mw.teams.first()).map(|t| t.spawns.iter().flat_map(|s| s.enemy_choices.iter().copied()).collect()).unwrap_or_default();
            let mut list = vec![(c.boss, c.level, true)];
            for k in 0..3 { if !guards.is_empty() { list.push((guards[fastrand::usize(..guards.len())], (c.level - 2 + k).max(1), false)); } }
            for (i, &(id, lv, boss)) in list.iter().enumerate() {
                let en = data.enemy(id);
                let ft = build_fighter(&en.stats, &en.formulas, lv, (1, 3));
                let ranged = is_ranged(en.profession, en.attack_mode);
                let u = Unit { side: Side::Enemy, trig: Vec4::ZERO, f: ft, ex: Default::default(), hp: ft.max_hp, atk_cd: 1.0, target: None, forced: None, move_to: None, roster: None, slot: i, enemy: id,
                    level: lv, ranged, healer: en.attack_type == AttackType::Buff, range: if ranged { 300.0 } else { 70.0 }, hatred: en.hatred,
                    taunted_by: None, buff: (0, 0, 0.0), skills: vec![], dead: false, dead_t: 0.0, team: 0, st: Status::default(), hit_t: 0.5 };
                let p = if boss { Vec2::new(330.0, -80.0) } else { Vec2::new(220.0 + (i % 2) as f32 * 90.0, 20.0 - i as f32 * 70.0) };
                let e = spawn_unit(&mut cmd, &a, &data, &sheets, &enemy_key(&data, id), p, u);
                if boss { cmd.entity(e).insert(Transform::from_translation(p.extend(10.0)).with_scale(Vec3::splat(1.25))); }
            }
        }
    }
    // Summon: the active minion (a captured castle boss) fights on the heroes' side by itself (Minion Auto-Attack)
    if let (Some(&(id, lv)), false) = (save.active_pet.and_then(|i| save.pets.get(i)), setup.tutorial) {
        let en = data.enemy(id);
        let ft = build_fighter(&en.stats, &en.formulas, lv.max(1), (1, 3));
        let ranged = is_ranged(en.profession, en.attack_mode);
        let u = Unit { side: Side::Hero, trig: Vec4::ZERO, f: ft, ex: Default::default(), hp: ft.max_hp, atk_cd: 0.5, target: None, forced: None, move_to: None, roster: None, slot: 98, enemy: id,
            level: lv, ranged, healer: false, range: if ranged { 300.0 } else { 70.0 }, hatred: en.hatred, taunted_by: None, buff: (0, 0, 0.0), skills: vec![],
            dead: false, dead_t: 0.0, team: 0, st: Status::default(), hit_t: 0.5 };
        spawn_unit(&mut cmd, &a, &data, &sheets, &enemy_key(&data, id), Vec2::new(-250.0, 40.0), u);
    }
    // arena (offline PVP): the opponent's heroes from PVP_DummyList.xml fight on the enemy side
    if let Some(ti) = setup.arena {
        if let Some(tg) = data.arena.targets.get(ti) {
            for (i, &(hi, lv)) in tg.team.iter().enumerate() {
                if data.heroes.heroes.iter().all(|h| h.index != hi) { continue; }
                let h = data.hero(hi);
                let ft = build_fighter(&h.stats, &h.formulas, lv.max(1), (2, 6));
                let ranged = is_ranged(h.profession, h.attack_mode);
                let u = Unit { side: Side::Enemy, trig: Vec4::ZERO, f: ft, ex: Default::default(), hp: ft.max_hp, atk_cd: 1.0, target: None, forced: None, move_to: None, roster: None, slot: i, enemy: -1,
                    level: lv, ranged, healer: h.attack_type == AttackType::Buff, range: if ranged { 330.0 } else { 70.0 }, hatred: h.hatred,
                    taunted_by: None, buff: (0, 0, 0.0), skills: vec![], dead: false, dead_t: 0.0, team: 0, st: Status::default(), hit_t: 0.5 };
                let p = Vec2::new(300.0 + (i % 2) as f32 * 80.0, -40.0 - i as f32 * 50.0);
                let e = spawn_unit(&mut cmd, &a, &data, &sheets, &hero_key(&data, hi), p, u);
                let n = text(&mut cmd, &f, &h.class_name, Vec2::new(0.0, 150.0), 13.0, Color::srgb(1.0, 0.75, 0.6), 5.2);
                cmd.entity(n).remove::<Scr>();
                cmd.entity(e).add_child(n);
            }
        }
    }
    let mut b = Sprite::from_color(Color::srgba(0.4, 1.0, 0.4, 0.15), Vec2::ONE);
    b.custom_size = Some(Vec2::ZERO);
    cmd.spawn((b, Transform::from_xyz(0.0, 0.0, 70.0), Scr, BoxSel));
    // drag-line order (StateHelpGame*: line_move/attack/heal + arrow_move/attack/heal while dragging from a hero)
    let imgs = |n: &str| [a.load(format!("ui/{n}_move.png")), a.load(format!("ui/{n}_attack.png")), a.load(format!("ui/{n}_heal.png"))];
    cmd.spawn((Sprite::from_image(a.load("ui/line_move.png")), Transform::from_xyz(0.0, 0.0, 69.0), Visibility::Hidden, Scr, DragViz { end: false, imgs: imgs("line") }));
    cmd.spawn((Sprite { image: a.load("ui/arrow_move.png"), custom_size: Some(Vec2::new(96.0, 46.0)), ..default() }, Transform::from_xyz(0.0, 0.0, 9.5), Visibility::Hidden, Scr, DragViz { end: true, imgs: imgs("arrow") }));
}

#[derive(Component)]
struct DragViz { end: bool, imgs: [Handle<Image>; 3] }

fn enemy_level(setup: &BattleSetup, file: i32, rng: &mut fastrand::Rng) -> i32 {
    if setup.level.1 > 0 { rng.i32(setup.level.0.max(1)..=setup.level.1.max(setup.level.0.max(1))) } else { file.max(1) }
}

#[allow(clippy::too_many_arguments)]
fn director(mut cmd: Commands, a: Res<AssetServer>, time: Res<Time>, data: Res<GameData>, sheets: Res<Sheets>, setup: Res<BattleSetup>, mut run: ResMut<Run>, units: Query<&Unit>) {
    if run.done { return; }
    if setup.tutorial { return; } // enemies are spawned by `tutorial`
    if setup.arena.is_some() || setup.siege.is_some() {
        run.won = !units.iter().any(|u| u.side == Side::Enemy && !u.dead);
        if run.won { run.done = true; }
        return;
    }
    let mw = data.waves.maps.get(&setup.map).or_else(|| data.waves.maps.values().next()).unwrap();
    if setup.survival && run.surv.is_none() {
        run.surv_level = data.survival.base_level.max(1);
        let mut rng = run.rng.take().unwrap_or_default();
        let t = surv_team(&data, &mut run, &mut rng); run.surv = Some(t); run.rng = Some(rng);
    }
    let surv_t = run.surv.clone();
    let team = if setup.survival { surv_t.as_ref().unwrap() } else { match mw.teams.get(run.team) { Some(t) => t, None => { run.done = true; run.won = true; return } } };
    // corpses are despawned 2.5 s after death, so count kills as spawned − still alive (not as dead units present)
    let alive = units.iter().filter(|u| u.side == Side::Enemy && u.team == run.team && !u.dead).count();
    let dead = run.spawned.iter().take(team.spawns.len().min(64)).filter(|&&b| b).count().saturating_sub(alive);
    run.dead_in_team = dead;
    let dt = time.delta_secs();
    let mut rng = run.rng.take().unwrap_or_default();
    for (i, s) in team.spawns.iter().enumerate().take(64) {
        if run.spawned[i] || dead < s.after_dead.max(0) as usize { continue; }
        run.timers[i] += dt;
        if run.timers[i] * 1000.0 < s.delay_ms as f32 { continue; }
        run.spawned[i] = true;
        let id = s.enemy_choices[rng.usize(..s.enemy_choices.len().max(1)).min(s.enemy_choices.len() - 1)];
        let e = data.enemy(id);
        let lvl = enemy_level(&setup, s.level, &mut rng);
        let ft = build_fighter(&e.stats, &e.formulas, lvl, (1, 3));
        let ranged = is_ranged(e.profession, e.attack_mode);
        let y = rng.f32() * (FIELD_MAX.y - FIELD_MIN.y - 40.0) + FIELD_MIN.y + 20.0;
        let u = Unit { side: Side::Enemy, trig: Vec4::ZERO, f: ft, ex: Default::default(), hp: ft.max_hp, atk_cd: 1.0, target: None, forced: None, move_to: None, roster: None, slot: 0, enemy: id,
            level: lvl, ranged, healer: e.attack_type == AttackType::Buff, range: if ranged { 300.0 } else { 70.0 }, hatred: e.hatred,
            taunted_by: None, buff: (0, 0, 0.0), skills: vec![], dead: false, dead_t: 0.0, team: run.team, st: Status::default(), hit_t: 0.5 };
        // Survival BOSS teams: 10% of enemies get x negated (they enter from the left)
        let x = if s.x < 0 { -520.0 } else { 520.0 };
        spawn_unit(&mut cmd, &a, &data, &sheets, &enemy_key(&data, id), Vec2::new(x, y), u);
    }
    run.rng = Some(rng);
    let all_spawned = (0..team.spawns.len().min(64)).all(|i| run.spawned[i]);
    if all_spawned && dead >= team.spawns.len() && setup.survival {
        // DeleteEnemy (survival): level += addLevel, waves+1, "Wave Incoming", next team
        run.kills += team.spawns.len() as i32;
        run.surv_waves += 1; run.surv_level += data.survival.add_level;
        run.spawned = vec![false; 64]; run.timers = vec![0.0; 64];
        let mut rng = run.rng.take().unwrap_or_default();
        let t = surv_team(&data, &mut run, &mut rng); run.surv = Some(t); run.rng = Some(rng);
        let wi = image(&mut cmd, &a, "ui/Wave_Incoming.png", Vec2::new(0.0, 160.0), None, 70.0);
        cmd.entity(wi).insert(Floating(-2.0));
        return;
    }
    if all_spawned && dead >= team.spawns.len() {
        run.team += 1;
        run.spawned = vec![false; 64]; run.timers = vec![0.0; 64];
        if run.team < mw.teams.len() {
            let wi = image(&mut cmd, &a, "ui/Wave_Incoming.png", Vec2::new(0.0, 160.0), None, 70.0);
            cmd.entity(wi).insert(Floating(-2.0));
        }
    }
}

fn hit_test(p: Vec2, t: &Transform) -> bool { let c = t.translation.truncate() + Vec2::Y * 45.0; (p - c).abs().cmple(Vec2::new(48.0, 65.0)).all() }

#[allow(clippy::too_many_arguments)]
fn input(mut cmd: Commands, keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, cur: Res<Cursor>, mut run: ResMut<Run>,
         mut clicked: ResMut<Clicked>, mut vt: ResMut<Time<Virtual>>, mut next: ResMut<NextState<AppState>>,
         mut units: Query<(Entity, &mut Unit, &Transform)>, sel: Query<Entity, With<Selected>>, hots: Query<(Entity, &Hot, &GlobalTransform, &InheritedVisibility)>,
         mut boxq: Query<(&mut Sprite, &mut Transform), (With<BoxSel>, Without<Unit>)>, mut ov: Query<&mut Visibility, With<PausedOverlay>>, mut casts: Local<Vec<(usize, usize)>>,
         (mut sfx, mut save, ticks): (ResMut<crate::audio::Sfx>, ResMut<Save>, Query<(&PauseTick, Entity)>), mut viz: Query<(&DragViz, &mut Sprite, &mut Transform, &mut Visibility), (Without<Unit>, Without<BoxSel>, Without<PausedOverlay>)>) {
    if run.outro.is_some() { return; }
    let mut toggle_pause = keys.just_pressed(KeyCode::KeyP);
    let mut speed_up = keys.just_pressed(KeyCode::KeyF);
    for act in clicked.0.drain(..) {
        match act {
            Act::Custom(11, _) => toggle_pause = true,
            Act::Custom(12, k) => if let Some((_, u, _)) = sel.iter().next().and_then(|e| units.get(e).ok()) { casts.push((u.slot, k as usize)); },
            Act::Custom(13, _) => speed_up = true,
            Act::Custom(14, _) => if vt.is_paused() { toggle_pause = true; },
            Act::Custom(15, _) => if vt.is_paused() { vt.unpause(); sfx.play("sfx_select"); next.set(AppState::WorldMap); return; },
            Act::Custom(16, i) => {
                if i == 0 { save.music = !save.music } else { save.sound = !save.sound }
                save.write();
                let on = if i == 0 { save.music } else { save.sound };
                for (t, e) in ticks.iter() { if t.0 == i as usize { cmd.entity(e).insert(if on { Visibility::Inherited } else { Visibility::Hidden }); } }
            }
            _ => {}
        }
    }
    if toggle_pause { if vt.is_paused() { vt.unpause() } else { vt.pause() } }
    if speed_up { save.battle_speed = save.battle_speed % 3 + 1; save.write(); sfx.play("sfx_select"); }
    vt.set_relative_speed(save.battle_speed.clamp(1, 3) as f32);
    if let Ok(mut v) = ov.single_mut() { *v = if vt.is_paused() { Visibility::Visible } else { Visibility::Hidden }; }
    if keys.just_pressed(KeyCode::Escape) { next.set(AppState::WorldMap); return; }
    if keys.just_pressed(KeyCode::F5) { next.set(AppState::Battle); return; }
    if keys.just_pressed(KeyCode::KeyA) { run.auto = !run.auto; }
    // Q/W/E/R: the selected hero's skills 1-4
    for (k, i) in [(KeyCode::KeyQ, 0), (KeyCode::KeyW, 1), (KeyCode::KeyE, 2), (KeyCode::KeyR, 3)] {
        if keys.just_pressed(k) { if let Some((_, u, _)) = sel.iter().next().and_then(|e| units.get(e).ok()) { casts.push((u.slot, i)); } }
    }
    for (k, i) in [(KeyCode::Digit1, 0), (KeyCode::Digit2, 1), (KeyCode::Digit3, 2), (KeyCode::Digit4, 3)] {
        if keys.just_pressed(k) {
            for e in sel.iter() { cmd.entity(e).remove::<Selected>(); }
            if let Some((e, _, _)) = units.iter().find(|(_, u, _)| u.side == Side::Hero && u.slot == i && !u.dead) { cmd.entity(e).insert(Selected); sfx.play("sfx_select"); }
        }
    }
    // skill casts are queued into the units' `skill.left` = -1 marker, executed by `ai`
    for (i, k) in casts.drain(..) {
        if let Some((_, mut u, _)) = units.iter_mut().find(|(_, u, _)| u.side == Side::Hero && u.slot == i && !u.dead) {
            if let Some(s) = u.skills.get_mut(k) { if s.left <= 0.0 { s.left = -1.0; } }
        }
    }
    if vt.is_paused() { return; }
    let Some(p) = cur.world else { return };
    let (lp, lr) = (mouse.just_pressed(MouseButton::Left), mouse.just_released(MouseButton::Left));
    if lp { run.ui_press = hot_at(p, &hots).is_some(); if !run.ui_press { run.drag_start = Some(p); run.drag_hero = pick(p, &units, Side::Hero); } }
    // ----- drag from a hero: line to the cursor; release orders that hero (enemy = attack, ally = heal for healers, else move)
    let drag_kind = |h: Entity, units: &Query<(Entity, &mut Unit, &Transform)>| -> (usize, Option<Entity>) {
        let healer = units.get(h).is_ok_and(|(_, u, _)| u.healer);
        if let Some(en) = pick(p, units, Side::Enemy).filter(|_| !healer) { return (1, Some(en)); }
        if let Some(al) = pick(p, units, Side::Hero).filter(|_| healer) { return (2, Some(al)); }
        (0, None)
    };
    if lp { run.drag_armed = false; }
    // once the finger has left the hero (>20 px) the drag stays armed, so dragging back onto a healer targets herself
    if run.drag_start.is_some_and(|s| (p - s).length() > 20.0) { run.drag_armed = true; }
    let dragging = run.drag_hero.filter(|_| run.drag_armed);
    for (dv, mut sp, mut t, mut v) in viz.iter_mut() {
        *v = Visibility::Hidden;
        let Some(h) = dragging.filter(|_| mouse.pressed(MouseButton::Left)) else { continue };
        let Ok((_, _, ht)) = units.get(h) else { continue };
        let (k, tgt) = drag_kind(h, &units);
        if sp.image != dv.imgs[k] { sp.image = dv.imgs[k].clone(); }
        let from = ht.translation.truncate() + Vec2::Y * 20.0;
        if dv.end {
            let at = tgt.and_then(|x| units.get(x).ok()).map(|(_, _, tt)| tt.translation.truncate()).unwrap_or(p);
            t.translation = at.extend(9.5);
        } else {
            let d = p - from;
            sp.custom_size = Some(Vec2::new(d.length(), 16.0));
            t.translation = ((from + p) / 2.0).extend(69.0);
            t.rotation = Quat::from_rotation_z(d.y.atan2(d.x));
        }
        *v = Visibility::Visible;
    }
    if lr && !run.ui_press {
        if let Some(h) = dragging {
            let (k, tgt) = drag_kind(h, &units);
            if let Ok((_, mut u, _)) = units.get_mut(h) {
                match k { 1 | 2 => { u.forced = tgt; u.move_to = None; } _ => { u.move_to = Some(p.clamp(FIELD_MIN, FIELD_MAX)); u.forced = None; u.target = None; } }
            }
            for o in sel.iter() { cmd.entity(o).remove::<Selected>(); }
            cmd.entity(h).insert(Selected);
            sfx.play("sfx_select");
            run.drag_start = None;
        }
    }
    if lr { run.drag_hero = None; }
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    // box select visual
    if let (Some(s), Ok((mut sp, mut t))) = (run.drag_start.filter(|_| run.drag_hero.is_none()), boxq.single_mut()) {
        let size = (p - s).abs();
        let show = mouse.pressed(MouseButton::Left) && size.length() > 10.0;
        sp.custom_size = Some(if show { size } else { Vec2::ZERO });
        t.translation = ((p + s) / 2.0).extend(70.0);
    }
    let order = |p: Vec2, units: &mut Query<(Entity, &mut Unit, &Transform)>, sel: &Query<Entity, With<Selected>>| {
        let enemy = pick(p, units, Side::Enemy);
        for e in sel.iter() {
            if let Ok((_, mut u, _)) = units.get_mut(e) {
                if u.healer && enemy.is_some() { continue; }
                if let Some(en) = enemy { u.forced = Some(en); u.move_to = None; }
                else { u.move_to = Some(p.clamp(FIELD_MIN, FIELD_MAX)); u.forced = None; u.target = None; }
            }
        }
        enemy.is_some()
    };
    if lr && !run.ui_press {
        if let Some(s) = run.drag_start.take() {
            if (p - s).length() > 10.0 {
                let (mn, mx) = (s.min(p), s.max(p));
                let inside: Vec<Entity> = units.iter().filter(|(_, u, t)| u.side == Side::Hero && !u.dead && {
                    let q = t.translation.truncate() + Vec2::Y * 40.0; q.cmpge(mn).all() && q.cmple(mx).all() }).map(|x| x.0).collect();
                if !inside.is_empty() {
                    if !shift { for e in sel.iter() { cmd.entity(e).remove::<Selected>(); } }
                    for e in inside { cmd.entity(e).insert(Selected); }
                }
            } else if !sel.is_empty() && pick(p, &units, Side::Enemy).is_some() {
                // with heroes selected, a click on an enemy is always an attack order (enemies win over overlapping heroes)
                order(p, &mut units, &sel);
                sfx.play("sfx_select");
            } else if let Some(e) = pick(p, &units, Side::Hero).filter(|&e| {
                // with a healer selected, clicking another ally sets her heal focus (yellow arrow_heal) instead of selecting it
                let healers: Vec<Entity> = sel.iter().filter(|&h| h != e && units.get(h).is_ok_and(|(_, u, _)| u.healer)).collect();
                if healers.is_empty() { return true; }
                for h in healers { if let Ok((_, mut u, _)) = units.get_mut(h) { u.forced = Some(e); u.move_to = None; } }
                sfx.play("sfx_select");
                false
            }) {
                if !shift { for o in sel.iter() { cmd.entity(o).remove::<Selected>(); } }
                cmd.entity(e).insert(Selected);
                sfx.play("sfx_select");
            } else if !sel.is_empty() {
                order(p, &mut units, &sel);
            }
        }
    }
    if lr { run.ui_press = false; run.drag_start = None; }
    if mouse.just_pressed(MouseButton::Right) { order(p, &mut units, &sel); }
}

fn play(anims: &mut Query<&mut Anim>, c: Option<Entity>, r: Row, once: bool, flip: Option<bool>) {
    if let Some(c) = c { if let Ok(mut an) = anims.get_mut(c) { if !(an.once && !an.done && !once) { an.play(r, once); } if let Some(fl) = flip { an.flip = fl; } } }
}


#[allow(clippy::too_many_arguments)]
fn ai(mut cmd: Commands, time: Res<Time>, data: Res<GameData>, mut run: ResMut<Run>, a: Res<AssetServer>, f: Res<Fonts>, sheets: Res<Sheets>,
      mut q: Query<(Entity, &mut Unit, &mut Transform, &Children)>, mut anims: Query<&mut Anim>, mut sfx: ResMut<crate::audio::Sfx>) {
    if run.outro.is_some() { return; }
    let dt = time.delta_secs();
    if dt == 0.0 { return; }
    let snapshot: Vec<(Entity, Side, Vec2, bool, f32, f32, i32)> = q.iter().map(|(e, u, t, _)| (e, u.side, t.translation.truncate(), u.dead, u.hp, u.f.max_hp, u.hatred)).collect();
    let mut rng = run.rng.take().unwrap_or_default();
    let invis: Vec<Entity> = q.iter().filter(|(_, u, _, _)| u.st.invis > 0.0).map(|(e, _, _, _)| e).collect();
    let mut damage: Vec<(Entity, Entity, f32, bool)> = vec![]; // (target, source, amount, is_heal)
    let mut taunts: Vec<(Entity, Vec2, f32, f32)> = vec![];
    let mut buffs: Vec<(Side, Vec2, f32, (i32, i32, f32))> = vec![];
    let mut shots: Vec<(Entity, Entity, f32, f32, Vec2)> = vec![];
    let mut fire_box: Vec<(Entity, f32, f32)> = vec![];
    // status effects queued by skills: (target, kind, value, seconds)
    let mut states: Vec<(Entity, &'static str, f32, f32)> = vec![];
    let mut summons: Vec<(i32, Vec2, i32, f32)> = vec![];
    let mut revive: Vec<(Side, f32)> = vec![];
    for (e, mut u, mut t, ch) in q.iter_mut() {
        let anim = ch.iter().find(|c| anims.get(*c).is_ok());
        if u.dead { u.dead_t += dt; continue; }
        if u.hp <= 0.0 {
            u.dead = true; play(&mut anims, anim, Row::Dead, true, None);
            // no death sound: GORoleCB plays none (sfx_lose is only SceneGameOver, when the whole party is down)
            if u.side == Side::Enemy && u.enemy >= 0 {
                let en = data.enemy(u.enemy);
                let x = (u.level - 1).max(0) as f32;
                run.exp += (en.exp as f32 + eval_formula(en.formulas.exp, x)) as i64;
                run.money += (en.money as f32 + eval_formula(en.formulas.money, x.min(40.0))) as i64;
            }
            continue;
        }
        u.atk_cd -= dt;
        if u.buff.2 > 0.0 { u.buff.2 -= dt; if u.buff.2 <= 0.0 { u.buff = (0, 0, 0.0); } }
        if let Some((_, tt)) = &mut u.taunted_by { *tt -= dt; if *tt <= 0.0 { u.taunted_by = None; } }
        // ----- timed states -----
        if u.st.dot.1 > 0.0 { u.st.dot.1 -= dt; u.hp -= u.st.dot.0 * dt; }
        if u.st.hot.1 > 0.0 { u.st.hot.1 -= dt; u.hp = (u.hp + u.st.hot.0 * dt).min(u.f.max_hp); }
        if u.st.reduce.1 > 0.0 { u.st.reduce.1 -= dt; }
        if u.st.invis > 0.0 { u.st.invis -= dt; }
        if u.st.immune > 0.0 { u.st.immune -= dt; }
        if u.st.fire_arrow.3 > 0.0 { u.st.fire_arrow.3 -= dt; }
        let st = &mut u.st;
        for m in [&mut st.atk, &mut st.ac, &mut st.aspd] { if m.1 > 0.0 { m.1 -= dt; if m.1 <= 0.0 { *m = (0.0, 0.0); } } }
        if u.st.hpmax.1 > 0.0 { u.st.hpmax.1 -= dt; if u.st.hpmax.1 <= 0.0 { u.f.max_hp -= u.st.hpmax.0; u.hp = u.hp.min(u.f.max_hp); u.st.hpmax = (0.0, 0.0); } }
        if u.st.life > 0.0 { u.st.life -= dt; if u.st.life <= 0.0 { u.hp = 0.0; } }
        if u.st.stun > 0.0 {
            u.st.stun -= dt;
            for s in &mut u.skills { if s.left > 0.0 { s.left -= dt; } }
            play(&mut anims, anim, Row::Hurt, false, None);
            continue;
        }
        let pos = t.translation.truncate();
        let alive = |x: Entity| snapshot.iter().find(|s| s.0 == x && !s.3).map(|s| s.2);
        // ----- skill -----
        let mut cast_idx = None;
        // Skills fire only when the player taps them (the -1 marker set by `input`). The original's auto-battle flag
        // (DungeonHeroesCpp+0xf71) is only read by AIManager/GOHero target picking, never by skill code.
        for (k, s) in u.skills.iter_mut().enumerate() {
            if s.left > 0.0 { s.left = (s.left - dt).max(0.0); }
            else if s.left < 0.0 && cast_idx.is_none() { cast_idx = Some(k); s.left = s.cd; }
        }
        if let Some(k) = cast_idx {
            let s = u.skills[k].skill.clone();
            if std::env::var("HVSM_SKILLS").is_ok() || std::env::var("HVSM_CASTLOG").is_ok() { eprintln!("SKILLTEST cast slot{} {} attrs={} cd={:.1}", u.slot, s.name, s.attrs.len(), u.skills[k].cd); }
            play(&mut anims, anim, Row::Skill, true, None);
            let snd = s.effects.iter().map(|v| v.begin_sound.trim()).find(|n| !n.is_empty()).map(|n| n.trim_end_matches(".wav").to_string());
            sfx.play(match snd.as_deref() { Some("sfx_rage") => "sfx_rage", Some("sfx_heal") => "sfx_heal", Some("sfx_holyheal") => "sfx_holyheal", Some("sfx_shoot") => "sfx_shoot", _ => "sfx_skillstart" });
            // a skill aimed at a target (begin_pos 1/2) with no lock takes the nearest foe (own fallback: the original makes
            // the player tap a target, StateHelpGameClickAttack)
            if u.target.and_then(alive).is_none() && s.attrs.iter().any(|a| a.target == SkillTarget::Enemy && a.begin_pos_code >= 1) {
                u.target = snapshot.iter().filter(|o| o.1 != u.side && !o.3 && !invis.contains(&o.0)).min_by(|a, b| a.2.distance(pos).total_cmp(&b.2.distance(pos))).map(|o| o.0);
            }
            let center = u.target.and_then(alive).unwrap_or(pos);
            let hits = s.count.max(1) as usize;
            // a 1 s attr time is a tick; the skill's visual effect time (Iron Song / Battle Song 14 s) is the real duration
            let fx_dur = s.effects.iter().filter_map(|x| x.time_ms).filter(|&t| t > 0).max().map(|t| t as f32 / 1000.0).unwrap_or(0.0);
            let mut did = false;
            let lvl = u.level;
            for at in &s.attrs {
                let name = format!("{:?}", at.effect);
                let v = stats::attr_value(at, lvl);
                let secs = at.time_ms.max(0) as f32 / 1000.0;
                if at.prop.is_some_and(|p| p > 0.0 && p < 100.0 && rng.f32() * 100.0 >= p) { continue; } // prop = chance %
                // area: the attr's width × height box (original 480×320 units, ×2) around the target, or around the caster
                let (w, h) = (at.width.unwrap_or(0.0) * 2.0, at.height.unwrap_or(0.0) * 2.0);
                let at_self = at.target == SkillTarget::Caster || at.target == SkillTarget::Ally || at.begin_pos_code == 0;
                let c = if at_self { pos } else { center };
                let inside = |o: &(Entity, Side, Vec2, bool, f32, f32, i32)| if w > 0.0 { (o.2 - c).abs().cmple(Vec2::new(w, h.max(w * 0.6)) / 2.0 + 20.0).all() } else { o.2.distance(c) < 60.0 };
                let foes: Vec<Entity> = snapshot.iter().filter(|o| o.1 != u.side && !o.3 && inside(o)).map(|o| o.0).collect();
                let foes = if foes.is_empty() { u.target.into_iter().filter(|&x| alive(x).is_some() && snapshot.iter().any(|o| o.0 == x && o.1 != u.side)).collect() } else { foes };
                let allies: Vec<Entity> = match at.target {
                    SkillTarget::Caster => vec![e],
                    _ => snapshot.iter().filter(|o| o.1 == u.side && !o.3 && (w == 0.0 || inside(o) || at.target == SkillTarget::Ally)).map(|o| o.0).collect(),
                };
                if std::env::var("HVSM_SKILLS").is_ok() { eprintln!("SKILLTEST {} {} foes={} allies={} v={:.1} secs={:.1}", s.name, name, foes.len(), allies.len(), v, secs); }
                match name.as_str() {
                    "DamagePhysics" | "DamageFire" | "DamageCold" | "DamageLight" | "Normaldamage" | "Fireexplosive" | "HolyKill" => {
                        // GORoleCB::OnUpdate: normaldamage becomes damage_physics of (attack + weapon roll) × data%; the skill's
                        // count (Rapid Shot 4) is the number of hits
                        for k in 0..hits {
                            let (base, _) = roll_damage(&mut rng, &u.f);
                            let amt = (if at.value_kind == ValueKind::Percent { base * v.max(0.1) } else { v.max(1.0) }) + u.st.atk.0;
                            for &o in &foes {
                                if u.ranged && w == 0.0 {
                                    shots.push((e, o, amt, k as f32 * 0.18, pos));
                                } else { damage.push((o, e, amt, false)); }
                            }
                        }
                        did = true;
                    }
                    "DamagePoison" | "Onfire" => {
                        let dps = if secs > 0.0 { v.max(1.0) / secs.max(1.0) * if name == "Onfire" { secs.max(1.0) } else { 1.0 } } else { v };
                        for &o in &foes { states.push((o, "dot", dps.max(1.0), secs.max(3.0))); }
                        did = true;
                    }
                    "DizzyHeld" | "Stumble" | "StumbleSleep" | "Frozen" | "HolyHeld" | "SpecialHeld" => {
                        for &o in &foes { states.push((o, "stun", 0.0, if secs > 1.0 { secs } else if secs > 0.0 { secs.max(fx_dur) } else { v.max(2.0) })); }
                        did = true;
                    }
                    "Kill" => { for &o in &foes { damage.push((o, e, 99999.0, false)); } did = true; }
                    "RecoverHP" | "Selfheal" | "Addacheal" => {
                        let tg = if name == "Selfheal" { vec![e] } else { allies.clone() };
                        for o in tg { let mx = snapshot.iter().find(|x| x.0 == o).map(|x| x.5).unwrap_or(100.0);
                            damage.push((o, e, if at.value_kind == ValueKind::Percent { mx * v } else { v.max(5.0) }, true)); }
                        did = true;
                    }
                    "RecoverHPslow" => {
                        for &o in &allies { let mx = snapshot.iter().find(|x| x.0 == o).map(|x| x.5).unwrap_or(100.0);
                            let tot = if at.value_kind == ValueKind::Percent { mx * v } else { v.max(5.0) };
                            states.push((o, "hot", tot / secs.max(1.0), secs.max(1.0))); }
                        did = true;
                    }
                    "ResistDamage" | "Avoiddamage" if secs > 0.0 => {
                        let pct = if at.value_kind == ValueKind::Percent { v } else { v / 100.0 };
                        // the attr's time is the tick; the visual effect's time (e.g. Iron Song 14 s) is the real duration
                        let dur = s.effects.iter().filter_map(|x| x.time_ms).filter(|&t| t > 0).max().map(|t| t as f32 / 1000.0).unwrap_or(0.0).max(secs);
                        for &o in &allies { states.push((o, "reduce", pct.clamp(0.05, 0.9), dur)); }
                        did = true;
                    }
                    "Invisible" => { states.push((e, "invis", 0.0, secs.max(3.0))); did = true; }
                    "Dispel" => { for &o in &allies { states.push((o, "dispel", 0.0, 0.0)); } did = true; }
                    "Attracthatred" => { taunts.push((e, pos, w.max(400.0), secs.max(4.0))); did = true; }
                    "Resurrection" => { revive.push((u.side, v.clamp(0.1, 1.0))); did = true; }
                    "Animatedead" => {
                        // data = EnemyAttribute index of the summoned creature, time = lifetime
                        summons.push((at.data.unwrap_or(93.0) as i32, pos + Vec2::new(if u.side == Side::Hero { 60.0 } else { -60.0 }, -20.0), lvl, secs.max(10.0)));
                        did = true;
                    }
                    "Adddamage" | "Addac" | "Attackspeed" | "Addmaxhp" if secs > 0.0 => {
                        // timed modifier on the attr's targets: Caster = self, Ally = allies in the box, Enemy = foes in the box
                        let tg: Vec<Entity> = match at.target { SkillTarget::Enemy => foes.clone(), SkillTarget::Caster => vec![e], _ => allies.clone() };
                        let pct = at.value_kind == ValueKind::Percent;
                        let kind = match name.as_str() { "Adddamage" => if pct { "atk%" } else { "atk" }, "Addac" => if pct { "ac%" } else { "ac" },
                            "Attackspeed" => "aspd", _ => if pct { "hpmax%" } else { "hpmax" } };
                        let d = if secs <= 1.0 { secs.max(fx_dur) } else { secs };
                        for o in tg { states.push((o, kind, v, d)); }
                        did = true;
                    }
                    "ResistPhysics" | "ResistFire" | "ResistCold" | "ResistPoison" if secs > 0.0 => { states.push((e, "immune", 0.0, secs)); did = true; }
                    "SkillCD" => { for &o in &allies { states.push((o, "skillcd", v, 0.0)); } did = true; }
                    "Changefirearrow" => {
                        let g = |n: &str| s.attrs.iter().find(|a| format!("{:?}", a.effect) == n).and_then(|a| a.data).unwrap_or(60.0) * 2.0;
                        states.push((e, "firearrow", v, secs)); fire_box.push((e, g("ChangefirearrowWidth"), g("ChangefirearrowHeight")));
                        did = true;
                    }
                    n if secs > 0.0 && (n == "Speed" || n == "Powerofgiant" || n.starts_with("Spiritofhero")) => {
                        let b = if n.starts_with("Addac") || n.ends_with("Addac") { (v.max(2.0) as i32, 0, secs) } else { (0, v.max(2.0) as i32, secs) };
                        buffs.push((u.side, c, if at.target == SkillTarget::Caster { 1.0 } else { w.max(600.0) }, b)); did = true;
                    }
                    _ => {}
                }
            }
            if !did { if let Some(tp) = u.target { let (d, _) = roll_damage(&mut rng, &u.f); damage.push((tp, e, d * 2.0, false)); } }
            let lbl = cmd.spawn((Text2d::new(s.name.clone()), TextFont { font: f.main.clone().into(), font_size: bevy::text::FontSize::Px(20.0), ..default() },
                TextColor(Color::srgb(1.0, 0.85, 0.2)), Transform::from_translation((pos + Vec2::Y * 150.0).extend(65.0)), Scr, Floating(1.2))).id();
            let _ = lbl;
        }
        // AIManager::OnUpdate: an enemy (or, with auto-battle on, a hero) standing outside the screen's x bounds is sent to
        // a random x inside them (GORoleCB::SetTarget), keeping its y
        if u.move_to.is_none() && (u.side == Side::Enemy || run.auto) && (pos.x > FIELD_MAX.x || pos.x < FIELD_MIN.x) {
            u.move_to = Some(Vec2::new(FIELD_MIN.x + rng.f32() * (FIELD_MAX.x - FIELD_MIN.x), pos.y));
        }
        // ----- choose target -----
        let enemies_of = |side: Side| snapshot.iter().filter(move |o| o.1 != side && !o.3);
        if let Some(fe) = u.forced { if alive(fe).is_none() { u.forced = None; } }
        if let Some((te, _)) = u.taunted_by { if alive(te).is_some() { u.target = Some(te); } }
        let mut tgt = u.forced.or(u.target.filter(|&x| alive(x).is_some()));
        if u.healer {
            // As in the original (priest AttackType Buff; StateHelpGameHeal: drag the priest onto an ally → line_heal /
            // arrow_heal) the healer never attacks: she heals her focus ally, or (auto) the most hurt ally.
            let side = u.side;
            let ally = |x: Entity| snapshot.iter().find(|o| o.0 == x && o.1 == side && !o.3);
            if u.forced.is_some_and(|f| ally(f).is_none()) { u.forced = None; }
            let focus = u.forced.and_then(ally).filter(|o| o.4 < o.5).map(|o| o.0);
            // AIManager mode 1 (buff): the lowest hp/maxhp ally — for heroes only while auto-battle is on (GOHero::OnUpdate
            // does no idle search for buff heroes in manual mode)
            let auto = side == Side::Enemy || run.auto;
            tgt = focus.or_else(|| snapshot.iter().filter(|o| auto && o.1 == side && !o.3 && o.4 < o.5)
                .min_by(|a, b| (a.4 / a.5).total_cmp(&(b.4 / b.5))).map(|o| o.0));
        }
        if tgt.is_none() && u.move_to.is_none() && !u.healer {
            let foes: Vec<_> = enemies_of(u.side).filter(|o| !invis.contains(&o.0)).collect();
            tgt = if u.side == Side::Enemy || run.auto {
                // AIManager::OnUpdate: an attack-type unit (AttackType "attack", mode 0) without a lock locks a random foe;
                // for heroes this runs only while auto-battle (DungeonHeroesCpp+0xf71) is on
                if foes.is_empty() { None } else { Some(foes[rng.usize(..foes.len())].0) }
            } else {
                // GOHero::OnUpdate, auto off: an idle attack-type hero sends GetEnemyInTriggerRectEvent with its TriggerRect;
                // SceneMainGameMachine::FindEnemyInTriggerRect returns the first live enemy (list order) inside
                // x in [px-left, px+right], y in [py-down, py+up]
                let r = u.trig;
                foes.iter().find(|o| o.2.x >= pos.x - r.x && o.2.x <= pos.x + r.y && o.2.y >= pos.y - r.z && o.2.y <= pos.y + r.w).map(|o| o.0)
            };
        }
        u.target = tgt;
        let speed = (u.f.move_speed * 1.6).clamp(60.0, 220.0);
        // ----- move order -----
        if let Some(m) = u.move_to {
            let d = m - pos;
            if d.length() < 6.0 { u.move_to = None; play(&mut anims, anim, Row::Standby, false, None); }
            else if !anim.and_then(|c| anims.get(c).ok()).map(|a| a.once && !a.done).unwrap_or(false) {
                t.translation += (d.normalize() * (speed * dt).min(d.length())).extend(0.0); play(&mut anims, anim, Row::Move, false, Some(d.x < 0.0));
            }
            continue;
        }
        let Some(te) = tgt else {
            let busy = anim.and_then(|c| anims.get(c).ok()).map(|a| a.once && !a.done).unwrap_or(false);
            if busy {} else if u.side == Side::Enemy { t.translation.x -= speed * dt; play(&mut anims, anim, Row::Move, false, Some(true)); } else { play(&mut anims, anim, Row::Standby, false, None); }
            continue;
        };
        let tp = alive(te).unwrap_or(pos);
        let d = tp - pos;
        let face = Some(d.x < 0.0);
        let friendly = snapshot.iter().any(|o| o.0 == te && o.1 == u.side);
        // a locked unit walks to its lock (GORoleCB::MoveToLockObj), heroes included
        // GORoleCB::OnUpdate only calls MoveToLockObj when AttackMode (+0x184) is 0 (near); far and buff units
        // fire from where they stand whatever the distance
        // a unit stands still until its attack / skill animation has finished (GORoleCB only moves in the move state)
        let busy = anim.and_then(|c| anims.get(c).ok()).map(|a| a.once && !a.done).unwrap_or(false);
        if !u.ranged && !u.healer && d.length() > u.range {
            // melee approach to the side of the target; ranged walk until in range
            if !busy {
                let goal = if u.ranged { tp } else { tp + Vec2::new(if d.x > 0.0 { -50.0 } else { 50.0 }, 0.0) };
                let g = goal - pos;
                if g.length() > 2.0 { t.translation += (g.normalize() * (speed * dt).min(g.length())).extend(0.0); }
                play(&mut anims, anim, Row::Move, false, face);
            }
        } else if u.atk_cd <= 0.0 {
            u.atk_cd = (u.f.attack_interval * (1.0 + u.st.aspd.0)).max(0.4);
            play(&mut anims, anim, Row::Attack, true, face);
            if std::env::var("HVSM_ATKLOG").is_ok() && u.ranged { eprintln!("ATKLOG start {:?} t={:.3} hit_t={:.3}", e, time.elapsed_secs(), u.hit_t); }
            run.pending.push((u.hit_t, e, te, u.ranged || friendly, friendly));
        } else if !(anim.and_then(|c| anims.get(c).ok()).map(|a| a.once && !a.done).unwrap_or(false)) {
            play(&mut anims, anim, Row::Standby, false, face);
        }
        let _ = &a;
    }
    // attacks land on the animation's release/impact frame (AnimationType.xml attack_a index)
    let mut fire = vec![];
    run.pending.retain_mut(|p| { p.0 -= dt; if p.0 <= 0.0 { fire.push(*p); false } else { true } });
    for (_, e, te, shot, friendly) in fire {
        let Some(&(_, side, pos, dead, ..)) = snapshot.iter().find(|o| o.0 == e) else { continue };
        if dead || snapshot.iter().all(|o| o.0 != te || o.3) { continue; }
        let ranged = shot && !friendly;
        if std::env::var("HVSM_ATKLOG").is_ok() && ranged { eprintln!("ATKLOG fire {:?} t={:.3}", e, time.elapsed_secs()); }
        {
            if shot {
                let col = if friendly { Color::srgb(0.5, 1.0, 0.5) } else if side == Side::Hero { Color::srgb(1.0, 0.9, 0.4) } else { Color::srgb(1.0, 0.4, 0.3) };
                cmd.spawn((Sprite::from_color(col, Vec2::new(14.0, 5.0)), Transform::from_translation((pos + Vec2::Y * 60.0).extend(40.0)), Scr,
                    Projectile { from: e, to: te, speed: 600.0, heal: friendly, dmg: -1.0, delay: 0.0 }));
                sfx.play(if friendly { "sfx_heal" } else if ranged { "sfx_shoot" } else { "sfx_punch" });
            } else {
                damage.push((te, e, -1.0, false)); // -1 = roll a normal attack
                sfx.play(if rng.bool() { "sfx_chop1" } else { "sfx_punch" });
            }
        }
        let _ = side;
    }
    // separation + bounds
    let pos: Vec<(Entity, Vec2, bool)> = q.iter().map(|(e, u, t, _)| (e, t.translation.truncate(), u.dead)).collect();
    for (e, u, mut t, ch) in q.iter_mut() {
        if u.dead { continue; }
        let p = t.translation.truncate();
        let mut push = Vec2::ZERO;
        // no sliding while an attack / skill animation plays
        let busy = ch.iter().find_map(|c| anims.get(c).ok()).map(|a| a.once && !a.done).unwrap_or(false);
        for (o, op, od) in &pos { if *o != e && !od { let d = p - *op; let l = d.length(); if l < 52.0 && l > 0.01 { push += d / l * (52.0 - l); } } }
        let mut np = if busy { p } else { p + push * 0.35 };
        let minx = if u.side == Side::Enemy { FIELD_MIN.x } else { FIELD_MIN.x };
        let maxx = if u.side == Side::Enemy { 560.0 } else { FIELD_MAX.x };
        np = np.clamp(Vec2::new(minx, FIELD_MIN.y), Vec2::new(maxx, FIELD_MAX.y));
        t.translation.x = np.x; t.translation.y = np.y;
        t.translation.z = 10.0 - np.y * 0.01; // lower = in front
    }
    // apply effects
    for (caster, p, r, dur) in taunts { for (_, mut u, t, _) in q.iter_mut() { if u.side == Side::Enemy && !u.dead && t.translation.truncate().distance(p) < r { u.taunted_by = Some((caster, dur)); u.forced = None; } } }
    for (side, p, r, b) in buffs { for (_, mut u, t, _) in q.iter_mut() { if u.side == side && !u.dead && t.translation.truncate().distance(p) < r { u.buff = b; } } }
    for (from, to, amt, delay, p) in shots {
        cmd.spawn((Sprite::from_color(Color::srgb(1.0, 0.9, 0.4), Vec2::new(14.0, 5.0)), Transform::from_translation((p + Vec2::Y * 60.0).extend(40.0)), Scr,
            Visibility::Hidden, Projectile { from, to, speed: 600.0, heal: false, dmg: amt, delay }));
    }
    for (te, src, amt, heal) in damage { apply(&mut cmd, &f, &mut q, &mut rng, te, src, amt, heal, &mut anims); }
    for (te, kind, v, t) in states {
        let Ok((_, mut u, tr, _)) = q.get_mut(te) else { continue };
        if u.dead { continue; }
        match kind {
            _ if std::env::var("HVSM_SKILLS").is_ok() && { eprintln!("SKILLTEST state {kind} v={v:.1} t={t:.1}"); false } => {}
            "stun" => { u.st.stun = u.st.stun.max(t); spawn_float(&mut cmd, &f, tr.translation.truncate() + Vec2::Y * 30.0, "STUN".into(), Color::srgb(1.0, 1.0, 0.4)); }
            "dot" => u.st.dot = (v, t),
            "hot" => u.st.hot = (v, t),
            "reduce" => u.st.reduce = (v, t),
            "invis" => u.st.invis = t,
            "atk" => u.st.atk = (v, t), "atk%" => u.st.atk = (u.f.attack as f32 * v, t),
            "ac" => u.st.ac = (v, t), "ac%" => u.st.ac = (u.f.ac as f32 * v, t),
            "aspd" => u.st.aspd = (v, t),
            "hpmax" | "hpmax%" => { if u.st.hpmax.1 > 0.0 { u.f.max_hp -= u.st.hpmax.0; }
                let b = if kind == "hpmax%" { u.f.max_hp * v } else { v }; u.f.max_hp += b; u.hp = (u.hp + b.max(0.0)).min(u.f.max_hp); u.st.hpmax = (b, t); }
            "immune" => u.st.immune = t,
            "skillcd" => { for s in &mut u.skills { s.left = (s.left * (1.0 + v)).max(0.0); } }
            "firearrow" => { let (w, h) = fire_box.iter().find(|x| x.0 == te).map(|x| (x.1, x.2)).unwrap_or((180.0, 120.0)); u.st.fire_arrow = (v, w, h, t); }
            "dispel" => { u.st.stun = 0.0; u.st.dot = (0.0, 0.0); }
            _ => {}
        }
    }
    for (side, frac) in revive {
        if let Some((_, mut u, _, ch)) = q.iter_mut().find(|(_, u, _, _)| u.side == side && u.dead && u.st.life == 0.0 && u.dead_t < 30.0) {
            u.dead = false; u.dead_t = 0.0; u.hp = u.f.max_hp * frac;
            for c in ch.iter() { if let Ok(mut an) = anims.get_mut(c) { an.play(Row::Standby, false); an.once = false; an.done = false; } }
        }
    }
    for (id, p, lvl, life) in summons {
        // Cap (own design, original has no explicit limit visible): max 3 live summons per side; oldest expires.
        let mut live: Vec<(f32, Entity)> = q.iter().filter(|(_, u, _, _)| u.slot == 99 && !u.dead && u.side == Side::Hero).map(|(e, u, _, _)| (u.st.life, e)).collect();
        live.sort_by(|a, b| a.0.total_cmp(&b.0));
        while live.len() >= 3 { let (_, e) = live.remove(0); if let Ok((_, mut u, _, _)) = q.get_mut(e) { u.st.life = 0.01; } }
        let en = data.enemy(id);
        let ft = build_fighter(&en.stats, &en.formulas, lvl.max(1), (1, 3));
        let ranged = is_ranged(en.profession, en.attack_mode);
        let u = Unit { side: Side::Hero, trig: Vec4::ZERO, f: ft, ex: Default::default(), hp: ft.max_hp, atk_cd: 0.5, target: None, forced: None, move_to: None, roster: None, slot: 99, enemy: id,
            level: lvl, ranged, healer: false, range: if ranged { 300.0 } else { 70.0 }, hatred: en.hatred, taunted_by: None, buff: (0, 0, 0.0), skills: vec![],
            dead: false, dead_t: 0.0, team: 0, st: Status { life, ..default() }, hit_t: 0.5 };
        if std::env::var("HVSM_SKILLS").is_ok() { eprintln!("SKILLTEST summon enemy#{id} life={life:.0} live_before={}", live.len()); }
        spawn_unit(&mut cmd, &a, &data, &sheets, &enemy_key(&data, id), p.clamp(FIELD_MIN, FIELD_MAX), u);
    }
    run.rng = Some(rng);
}

#[allow(clippy::too_many_arguments)]
fn apply(cmd: &mut Commands, f: &Fonts, q: &mut Query<(Entity, &mut Unit, &mut Transform, &Children)>, rng: &mut fastrand::Rng, te: Entity, src: Entity, amt: f32, heal: bool, anims: &mut Query<&mut Anim>) {
    let atk = q.get(src).ok().map(|(_, u, _, _)| (u.f, u.buff.1 + u.st.atk.0.round() as i32));
    let Ok((_, mut u, t, ch)) = q.get_mut(te) else { return };
    if u.dead { return; }
    let pos = t.translation.truncate();
    let (txt, col) = if heal {
        let a = if amt < 0.0 { atk.map(|a| a.0.attack as f32 + 5.0).unwrap_or(10.0) } else { amt };
        u.hp = (u.hp + a).min(u.f.max_hp);
        (format!("+{:.0}", a), Color::srgb(0.4, 1.0, 0.4))
    } else {
        let mut d = amt;
        if d < 0.0 {
            let Some((af, bonus)) = atk else { return };
            let mut df = u.f; df.ac += u.st.ac.0.round() as i32;
            if hit_roll(rng, &af, &df, DefenseKind::Ac) == HitResult::Miss || rng.i32(0..100) < u.ex.avoid_pct {
                spawn_float(cmd, f, pos, "MISS".into(), Color::srgb(0.8, 0.8, 0.8)); return;
            }
            let (dd, crit) = roll_damage(rng, &af);
            d = dd + bonus as f32;
            if crit { d *= 1.0; spawn_float(cmd, f, pos + Vec2::Y * 20.0, "CRIT".into(), Color::srgb(1.0, 0.6, 0.1)); }
        }
        if u.st.immune > 0.0 { spawn_float(cmd, f, pos, "IMMUNE".into(), Color::srgb(0.8, 0.9, 1.0)); return; }
        let m = mitigate(d, u.f.resist_physics + u.buff.0, u.ex.resist_flat, 0.0).max(1.0) * if u.st.reduce.1 > 0.0 { 1.0 - u.st.reduce.0 } else { 1.0 };
        u.hp -= m;
        // AIManager: a unit keeps its lock until the target dies; being hit only gives an idle unit a target
        if u.side == Side::Enemy && u.forced.is_none() && u.taunted_by.is_none() && u.target.is_none() { u.target = Some(src); }
        // GOHero::OnGameEvent (event 3 = AttackEnemyEvent hit): an attack-type hero with no lock locks onto its attacker
        if u.side == Side::Hero && !u.healer && u.forced.is_none() && u.target.is_none() && u.move_to.is_none() { u.target = Some(src); }
        for c in ch.iter() { if let Ok(mut a) = anims.get_mut(c) { if a.row == Row::Standby || a.row == Row::Move { a.play(Row::Hurt, true); } } }
        (format!("{:.0}", m), if u.side == Side::Hero { Color::srgb(1.0, 0.35, 0.3) } else { Color::WHITE })
    };
    spawn_float(cmd, f, pos, txt, col);
}

fn spawn_float(cmd: &mut Commands, f: &Fonts, pos: Vec2, s: String, c: Color) {
    cmd.spawn((Text2d::new(s), TextFont { font: f.main.clone().into(), font_size: bevy::text::FontSize::Px(20.0), ..default() }, TextColor(c),
        Transform::from_translation((pos + Vec2::new(0.0, 110.0)).extend(66.0)), Scr, Floating(0.9)));
}

fn projectiles(mut cmd: Commands, time: Res<Time>, f: Res<Fonts>, mut run: ResMut<Run>, mut p: Query<(Entity, &mut Projectile, &mut Transform), Without<Unit>>,
               mut q: Query<(Entity, &mut Unit, &mut Transform, &Children)>, mut anims: Query<&mut Anim>) {
    let mut rng = run.rng.take().unwrap_or_default();
    let dt = time.delta_secs();
    for (e, mut pr, mut t) in p.iter_mut() {
        if pr.delay > 0.0 { pr.delay -= dt; if pr.delay <= 0.0 { cmd.entity(e).insert(Visibility::Inherited); } continue; }
        let Ok((_, u, tt, _)) = q.get(pr.to) else { cmd.entity(e).despawn(); continue };
        if u.dead { cmd.entity(e).despawn(); continue; }
        let goal = tt.translation.truncate() + Vec2::Y * 50.0;
        let d = goal - t.translation.truncate();
        let step = pr.speed * time.delta_secs();
        t.rotation = Quat::from_rotation_z(d.y.atan2(d.x));
        if d.length() <= step {
            cmd.entity(e).despawn();
            apply(&mut cmd, &f, &mut q, &mut rng, pr.to, pr.from, pr.dmg, pr.heal, &mut anims);
            if pr.dmg >= 0.0 && std::env::var("HVSM_SKILLS").is_ok() { eprintln!("SKILLTEST skill arrow hit dmg={:.1}", pr.dmg); }
            // Flaming Arrows: each arrow also deals its fire damage to the shooter's foes in a box around the hit
            let fa = q.get(pr.from).ok().filter(|x| x.1.st.fire_arrow.3 > 0.0).map(|x| (x.1.st.fire_arrow, x.1.side));
            if let (Some(((dmg, w, h, _), side)), false) = (fa, pr.heal) {
                let c = goal - Vec2::Y * 50.0;
                let hit: Vec<Entity> = q.iter().filter(|x| x.1.side != side && !x.1.dead && (x.2.translation.truncate() - c).abs().cmple(Vec2::new(w, h) / 2.0).all()).map(|x| x.0).collect();
                for o in hit { apply(&mut cmd, &f, &mut q, &mut rng, o, pr.from, dmg.max(1.0), false, &mut anims); }
            }
        }
        else { t.translation += (d.normalize() * step).extend(0.0); }
    }
    run.rng = Some(rng);
}

fn effects(mut cmd: Commands, time: Res<Time>, mut fl: Query<(Entity, &mut Floating, &mut Transform)>, units: Query<(Entity, &Unit)>,
           mut fills: Query<(&HpFill, &mut Sprite, &mut Transform), Without<Floating>>, mut rings: Query<(&Ring, &mut Visibility)>, sel: Query<(), With<Selected>>) {
    let dt = time.delta_secs();
    for (e, mut f, mut t) in fl.iter_mut() {
        if f.0 < 0.0 { f.0 += dt; if f.0 >= 0.0 { cmd.entity(e).despawn(); } continue; } // banners: fixed, negative ttl
        f.0 -= dt; t.translation.y += 40.0 * dt;
        if f.0 <= 0.0 { cmd.entity(e).despawn(); }
    }
    for (h, mut s, mut t) in fills.iter_mut() {
        if let Ok((_, u)) = units.get(h.unit) {
            let r = (u.hp / u.f.max_hp).clamp(0.0, 1.0);
            s.custom_size = Some(Vec2::new(h.w * r, 6.0));
            t.translation.x = -h.w * (1.0 - r) / 2.0;
            if u.dead { s.custom_size = Some(Vec2::ZERO); }
        }
    }
    for (r, mut v) in rings.iter_mut() {
        let on = sel.get(r.0).is_ok() && units.get(r.0).map(|(_, u)| !u.dead).unwrap_or(false);
        *v = if on { Visibility::Inherited } else { Visibility::Hidden };
    }
    // fade out dead enemies
    for (e, u) in units.iter() { if u.side == Side::Enemy && u.dead && u.dead_t > 2.5 { cmd.entity(e).despawn(); } }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn hud(run: Res<Run>, setup: Res<BattleSetup>, data: Res<GameData>,
       (mut ov, mut at, save, sel, mut icon, mut frames, a): (
           Query<(&CdOverlay, &mut Sprite), (Without<SelIcon>, Without<PartyHpFill>)>,
           Query<&mut Text2d, With<AutoText>>, Res<Save>, Query<&Unit, With<Selected>>,
           Query<(&mut SelIcon, &mut Sprite), (Without<CdOverlay>, Without<PartyHpFill>)>,
           Query<(&SkillFrame, &mut Visibility)>, Res<AssetServer>),
       (mut wave_t, mut party_t, mut party_hp, units): (
           Query<&mut Text2d, (With<WaveHud>, Without<AutoText>, Without<PartyHud>)>,
           Query<(&PartyHud, &mut Text2d), (Without<AutoText>, Without<WaveHud>)>,
           Query<(&PartyHpFill, &mut Sprite, &mut Transform), (Without<SelIcon>, Without<CdOverlay>)>,
           Query<&Unit>)) {
    let su = sel.iter().next().filter(|u| u.side == Side::Hero);
    let n = su.map(|u| u.skills.len()).unwrap_or(0);
    for (mut si, mut sp) in icon.iter_mut() {
        let p = su.and_then(|u| u.skills.get(si.0)).map(|s| format!("icons/{}.png", s.skill.icon)).filter(|p| crate::data::asset_exists(p)).unwrap_or("ui/item.png".into());
        if si.1 != p { sp.image = a.load(&p); si.1 = p; }
        sp.color = if si.0 < n.max(1) { Color::WHITE } else { Color::NONE };
    }
    for (fr, mut v) in frames.iter_mut() { let want = if fr.0 < n.max(1) { Visibility::Inherited } else { Visibility::Hidden }; if *v != want { *v = want; } }
    for (c, mut s) in ov.iter_mut() {
        let r = su.map(|u| if u.dead { 1.0 } else { u.skills.get(c.0).map(|s| (s.left / s.cd).clamp(0.0, 1.0)).unwrap_or(1.0) }).unwrap_or(1.0);
        s.custom_size = Some(Vec2::new(66.0, 66.0 * r));
    }
    if let Ok(mut t) = at.single_mut() { let s = format!("x{}", save.battle_speed.clamp(1, 3)); if t.0 != s { t.0 = s; } }
    // wave / enemy counter
    if let Ok(mut t) = wave_t.single_mut() {
        let alive = units.iter().filter(|u| u.side == Side::Enemy && !u.dead).count();
        let s = if setup.survival {
            format!("Wave {}  Enemies {}", run.surv_waves.max(0) + 1, alive)
        } else if let Some(mw) = data.waves.maps.get(&setup.map) {
            let total = mw.teams.len().max(1);
            let cur = (run.team + 1).min(total);
            format!("Wave {cur}/{total}  Enemies {alive}")
        } else {
            format!("Enemies {alive}")
        };
        if t.0 != s { t.0 = s; }
    }
    // party HP fills + dead greying of names
    for (ph, mut tx) in party_t.iter_mut() {
        let u = units.iter().find(|u| u.side == Side::Hero && u.slot == ph.0);
        if let Some(u) = u {
            let name = save.party.get(ph.0).and_then(|&r| save.heroes.get(r)).map(|oh| data.hero(oh.hero).class_name.clone()).unwrap_or_default();
            let s = if u.dead { format!("{name}  KO") } else { format!("{name}  {:.0}/{:.0}", u.hp, u.f.max_hp) };
            if tx.0 != s { tx.0 = s; }
        }
    }
    for (ph, mut sp, mut tr) in party_hp.iter_mut() {
        let u = units.iter().find(|u| u.side == Side::Hero && u.slot == ph.0);
        let r = u.map(|u| if u.dead { 0.0 } else { (u.hp / u.f.max_hp).clamp(0.0, 1.0) }).unwrap_or(0.0);
        let w = 120.0 * r;
        sp.custom_size = Some(Vec2::new(w, 10.0));
        // keep the fill left-aligned inside the 120px bg centred at -370
        tr.translation.x = -370.0 - (120.0 - w) / 2.0;
    }
}

fn end_check(mut run: ResMut<Run>, units: Query<&Unit>, ents: Query<(Entity, &Unit)>, mut save: ResMut<Save>, data: Res<GameData>, setup: Res<BattleSetup>, mut next: ResMut<NextState<AppState>>,
             mut out: ResMut<crate::result::Outcome>, time: Res<Time>, mut wait: Local<f32>) {
    let heroes_dead = units.iter().filter(|u| u.side == Side::Hero && u.roster.is_some()).all(|u| u.dead);
    if run.outro.is_some() || !(run.won || heroes_dead) { *wait = 0.0; return; }
    *wait += time.delta_secs();
    if *wait < 1.5 { return; }
    *wait = 0.0;
    let won = run.won && !heroes_dead;
    let mut lines = vec![];
    let mut surv = None;
    if setup.survival {
        // StateMainGameChallangeOver: kills, waves complete, rank Wave1-8 / 9-18 / 19-30 / 31-44 / 45+, and one random item
        // from the highest <Award> whose teamNum the run reached, added to the bag
        let waves = run.surv_waves;
        let kills = run.kills + run.dead_in_team as i32;
        let tier = match waves { ..=8 => 0, 9..=18 => 1, 19..=30 => 2, 31..=44 => 3, _ => 4 };
        let item = data.survival.award.iter().filter(|(n, _)| waves >= *n).last()
            .and_then(|(_, l)| if l.is_empty() { None } else { Some(l[fastrand::usize(..l.len())]) })
            .filter(|&i| data.item(i).is_some());
        if let Some(i) = item { if let Some(s) = save.first_free_stash() { save.stash[s] = Some(i); } }
        save.survival_best = save.survival_best.max(waves);
        surv = Some((kills, waves, item, tier));
    }
    if setup.arena.is_some() {
        // PVP_Reward lobby "Low": medal 4, score a=5; a loss gives half
        let (m, sc) = if won { (4, 5) } else { (2, 2) };
        save.medals += m; save.arena_score += sc;
        lines.push(format!("Arena: +{m} medals, +{sc} score"));
        run.money = run.money.max(if won { 50 } else { 0 });
    }
    if let (Some(ci), true) = (setup.siege, won) {
        if let Some(c) = data.siege.castles.get(ci) {
            let (s, x) = (&data.siege, c.level as f32);
            let ev = crate::data::SiegeTable::eval;
            let (st, bd) = (ev(s.stars, x), ev(s.badges, x));
            save.stars += st; save.badges += bd;
            run.money += ev(s.money, x); run.exp += ev(s.exp, x);
            lines.push(format!("Victory! Guardian Stars +{st}  Badges +{bd}"));
            if !save.castles.contains(&ci) {
                save.castles.push(ci);
                save.pets.push((c.boss, c.level));
                if save.active_pet.is_none() { save.active_pet = Some(save.pets.len() - 1); }
                lines.push(format!("{} Castle captured! {} joins you as a minion", c.name, c.name));
            }
        }
    }
    let mut loot = vec![];
    if setup.tutorial && won { // GuideConfig.xml FirstFightReward: money 1000, exp 100, equip 504
        save.tutorial_done = true;
        if let (Some(it), Some(s)) = (data.item(504), save.first_free_stash()) { save.stash[s] = Some(504); loot.push(504); lines.push(format!("Reward: {}", it.name)); }
    }
    let mut outro = if setup.survival || setup.arena.is_some() || setup.siege.is_some() { None } else {
        let mut bars = vec![];
        for (slot, &r) in save.party.iter().enumerate() {
            let Some(e) = ents.iter().find(|(_, u)| u.roster == Some(r)).map(|(e, _)| e) else { continue };
            let oh = &save.heroes[r];
            bars.push(OutroBar { roster: r, slot, unit: Some(e), from: (oh.exp as f32 / stats::exp_to_next(&data, oh).max(1) as f32).clamp(0.0, 1.0), to: 0.0, levels: 0 });
        }
        Some(Outro { bars, loot: loot.clone(), exp: if won { run.exp } else { 0 }, ..default() })
    };
    save.add_money(run.money);
    if won {
        let party = save.party.clone();
        for r in party {
            let oh = &mut save.heroes[r];
            oh.exp += run.exp;
            loop {
                let need = stats::exp_to_next(&data, oh);
                if oh.exp < need { break; }
                oh.exp -= need; oh.level += 1;
                if let Some(b) = outro.as_mut().and_then(|o| o.bars.iter_mut().find(|b| b.roster == r)) { b.levels += 1; }
                lines.push(format!("{} reached Lv{} (+1 skill point)", data.hero(oh.hero).class_name, oh.level));
            }
        }
        // as in the original, beating an area's boss opens the next area (BossInfo / unlock chain)
        if setup.boss && setup.node == save.unlocked.max(1) && (save.unlocked as usize) + 1 < data.world.nodes.len() {
            save.unlocked = save.unlocked.max(1) + 1;
            // own rule (user request): the first boss opens the 4th party slot; the original sold it (BUY_4player, 50,000 gold)
            if save.team_size < 4 { save.team_size = 4; lines.push("A 4th hero can now join your party".into()); }
            lines.push(format!("New area unlocked! (node {})", save.unlocked));
        }
        // hero camps (EnemyRules <FallHero>): weighted pick, a win adds the hero to the hire list (addHeroToUnHire)
        if setup.arena.is_none() && setup.siege.is_none() && !setup.tutorial {
            if let Some((ids, w)) = data.fall_hero.get(&setup.map) {
                if let Some(h) = pick_weighted(ids, w) {
                    if !save.found.contains(&h) { save.found.push(h); }
                    if let Some(o) = outro.as_mut() { o.new_hero = Some(h); }
                    lines.push(format!("Hero Encountered! {} can now be hired at the Tavern", data.hero(h).class_name));
                }
            }
        }
        crate::store::refresh_shop(&data, &mut save);
        lines.push("The store has been restocked".into());
    }
    save.write();
    if let Some(mut o) = outro.take() {
        o.won = won; o.money = run.money;
        for b in o.bars.iter_mut() { let oh = &save.heroes[b.roster]; b.to = (oh.exp as f32 / stats::exp_to_next(&data, oh).max(1) as f32).clamp(0.0, 1.0); }
        run.outro = Some(o);
        run.won = false; run.done = true;
        return;
    }
    *out = crate::result::Outcome { won, money: run.money, exp: if won { run.exp } else { 0 }, lines, survival: surv };
    run.won = false;
    next.set(AppState::Result);
}

/// SceneHelpGame / StateHelpGame*: the guided first battle (warrior + cleric on map_00) with the original English prompts.
/// Each step waits for the player's order. Own design: steps advance on the order itself (the original also froze the
/// field and showed a moving finger), heroes cannot drop below 1 HP, and "upper left" became "below" for our skill bar.
#[allow(clippy::too_many_arguments)]
fn tutorial(mut cmd: Commands, a: Res<AssetServer>, data: Res<GameData>, sheets: Res<Sheets>, setup: Res<BattleSetup>, mut run: ResMut<Run>, time: Res<Time>,
            mut q: Query<(Entity, &mut Unit, &Transform)>, mut txt: Query<&mut Text2d, With<TutText>>) {
    if !setup.tutorial || run.done { return; }
    run.tut_t += time.delta_secs();
    let heroes: Vec<(Entity, bool, Vec2)> = q.iter().filter(|(_, u, _)| u.side == Side::Hero && u.slot < 10).map(|(e, u, t)| (e, u.healer, t.translation.truncate())).collect();
    let Some(&(w, _, _)) = heroes.iter().find(|h| !h.1) else { return };
    let cl = heroes.iter().find(|h| h.1).map(|h| (h.0, h.2));
    // nobody dies before the lesson they are needed for (heroes never; the first monster not before step 6)
    let keep_foe = run.tut < 6;
    for (_, mut u, _) in q.iter_mut() { if (u.side == Side::Hero || keep_foe) && u.hp < 1.0 && !u.dead { u.hp = 1.0; } }
    let enemies: Vec<Entity> = q.iter().filter(|(_, u, _)| u.side == Side::Enemy && !u.dead).map(|(e, _, _)| e).collect();
    let foe_id = data.world.nodes.first().and_then(|n| n.maps.first()).and_then(|m| data.waves.maps.get(m))
        .and_then(|mw| mw.teams.first()).and_then(|t| t.spawns.first()).and_then(|s| s.enemy_choices.first().copied()).unwrap_or(0);
    let spawn = |cmd: &mut Commands, pos: Vec2, target: Option<Entity>, walk: Option<Vec2>| {
        let e = data.enemy(foe_id);
        let ft = build_fighter(&e.stats, &e.formulas, 1, (1, 3));
        let ranged = is_ranged(e.profession, e.attack_mode);
        let u = Unit { side: Side::Enemy, trig: Vec4::ZERO, f: ft, ex: Default::default(), hp: ft.max_hp, atk_cd: 1.0, target, forced: target, move_to: walk, roster: None, slot: 0, enemy: foe_id,
            level: 1, ranged, healer: false, range: if ranged { 300.0 } else { 70.0 }, hatred: e.hatred, taunted_by: None, buff: (0, 0, 0.0), skills: vec![],
            dead: false, dead_t: 0.0, team: 0, st: Status::default(), hit_t: 0.5 };
        spawn_unit(cmd, &a, &data, &sheets, &enemy_key(&data, foe_id), pos, u);
    };
    let get = |q: &Query<(Entity, &mut Unit, &Transform)>, e: Entity| q.get(e).ok().map(|(_, u, _)| (u.forced, u.move_to, u.skills.first().map(|s| s.left)));
    let step = run.tut;
    let next = match step {
        0 => get(&q, w).is_some_and(|x| x.1.is_some()) || cl.is_some_and(|c| get(&q, c.0).is_some_and(|x| x.1.is_some())),
        1 => enemies.iter().any(|&e| get(&q, w).is_some_and(|x| x.0 == Some(e))),
        2 => cl.is_some_and(|c| get(&q, c.0).is_some_and(|x| x.0 == Some(w))),
        3 => cl.is_some_and(|c| get(&q, c.0).is_some_and(|x| x.0 == Some(c.0))),
        4 => cl.is_some_and(|c| get(&q, c.0).is_some_and(|x| x.2.is_some_and(|l| l > 0.0))),
        5 => cl.is_some_and(|c| get(&q, c.0).is_some_and(|x| x.1.is_some())),
        6 => enemies.iter().any(|&e| get(&q, w).is_some_and(|x| x.0 == Some(e))),
        7 => enemies.is_empty(),
        _ => run.tut_t > 3.0,
    };
    let skip = cl.is_none() && matches!(step, 2..=5); // no cleric hired: skip the healing lessons
    if next || skip || run.tut_t > 0.0 && step == u8::MAX {
        run.tut += 1; run.tut_t = 0.0;
        match run.tut {
            // StateHelpGameHalfDead / ClickAttackSelect place the monster at (300,-13) and walk it to (150,-13); here it enters from
            // behind the right frame (like every wave) and walks onto the field (own choice so it never pops into view).
            1 => { let p = cl.map(|c| c.1).unwrap_or(Vec2::new(-300.0, -120.0)); spawn(&mut cmd, Vec2::new(540.0, p.y), cl.map(|c| c.0), None); }
            2 => if let Ok((_, mut u, _)) = q.get_mut(w) { u.hp = u.hp.min(u.f.max_hp * 0.55); },
            3 => if let Some(c) = cl { if let Ok((_, mut u, _)) = q.get_mut(c.0) { u.hp = u.hp.min(u.f.max_hp * 0.5); u.forced = None; } },
            4 => if let Some(c) = cl { if let Ok((_, mut u, _)) = q.get_mut(c.0) { if let Some(s) = u.skills.first_mut() { s.left = 0.0; } } },
            6 => { spawn(&mut cmd, Vec2::new(540.0, -13.0), None, Some(Vec2::new(150.0, -13.0))); spawn(&mut cmd, Vec2::new(580.0, -150.0), None, Some(Vec2::new(200.0, -150.0))); }
            9 => { run.done = true; run.won = true; run.money = 1000; run.exp = 100; }
            _ => {}
        }
    }
    let msg = match run.tut {
        0 => "Drag your characters into battle formation!",
        1 => "Look out! Your cleric is under attack!\nProtect your cleric by dragging your warrior\ntoward the enemy!",
        2 => "Your warrior is hurt!\nHurry up and have your cleric heal him!",
        3 => "Now your cleric is hurt too.\nDrag to make her target herself!",
        4 => "Tap to select a character and use skills.\nTap on the heal skill icon below to\nuse it!",
        5 => "Tap anywhere on the battlefield to move her\nout of danger.",
        6 => "Now choose your valiant warrior once more!\nThere is no need to drag him to an enemy to\nattack; just tap on the enemy directly!",
        7 => "Defeat the monsters!",
        _ => "Nice work!\nThe battles to come will be far more intense!",
    };
    if let Ok(mut t) = txt.single_mut() { if t.0 != msg { t.0 = msg.into(); } }
}

/// SceneMainGameMachine::initEnemyRules: r = lrand48() % sum(weights), walk the weights.
pub fn pick_weighted(ids: &[i32], w: &[i32]) -> Option<i32> {
    let sum: i32 = w.iter().sum();
    if sum < 1 || ids.is_empty() { return None; }
    let mut r = fastrand::i32(0..sum);
    for (i, &x) in w.iter().enumerate() { if r < x { return ids.get(i).copied(); } r -= x; }
    ids.last().copied()
}

/// SceneMainGameMachine::DeleteEnemy, survival branch: if (waves+1-baseTeam) % addTeam == 0 and waves > 0 the next team is
/// the next BOSS team (in order, wrapping); otherwise a random <Rules> team whose every enemy is drawn from the
/// <Enemy enemyIndex> pool. Every enemy gets the current level. BOSS enemies: 10% get their x negated.
pub fn surv_team(data: &GameData, run: &mut Run, rng: &mut fastrand::Rng) -> Team {
    let sv = &data.survival;
    let w = run.surv_waves;
    let boss = w > 0 && sv.boss_add > 0 && (w + 1 - sv.boss_base).rem_euclid(sv.boss_add) == 0 && !sv.boss.is_empty();
    let spawns = if boss {
        let t = &sv.boss[run.surv_boss % sv.boss.len()];
        run.surv_boss += 1;
        t.iter().map(|&(e, ld, dt, x)| crate::types::Spawn { enemy_choices: vec![e], level: run.surv_level, after_dead: ld, delay_ms: dt, x: if rng.i32(0..100) < 10 { -x } else { x } }).collect()
    } else {
        let t = &sv.rules[rng.usize(..sv.rules.len().max(1)).min(sv.rules.len().saturating_sub(1))];
        t.iter().map(|&(ld, dt, x)| crate::types::Spawn { enemy_choices: vec![sv.pool[rng.usize(..sv.pool.len())]], level: run.surv_level, after_dead: ld, delay_ms: dt, x }).collect()
    };
    Team { index: w, spawns }
}

#[derive(Clone, Default)]
pub struct OutroBar { pub roster: usize, pub slot: usize, pub unit: Option<Entity>, pub from: f32, pub to: f32, pub levels: u32 }
#[derive(Clone, Default)]
pub struct Outro { pub t: f32, pub won: bool, pub money: i64, pub exp: i64, pub bars: Vec<OutroBar>, pub loot: Vec<i32>, pub new_hero: Option<i32>, pub stage: u8, pub lv_played: u32 }
#[derive(Component)]
struct OutroEl(u8);
#[derive(Component)]
struct OutroFill(usize);

/// SceneGameOver (STAGECLEAR / GAMEOVER) played over the battle like the original: the party lines up in formation,
/// the banner shows, then the Results panel (results / fengetiao / gold / loot / newhero textures at the decoded
/// positions), then initExpInfo: "+exp" over each hero and the S02/S01/S03 bar under the feet, then the party walks off.
/// Positions are the original 480x320 values x2. Own design: the stage timings (taken from the gameplay video).
fn outro(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, data: Res<GameData>, sheets: Res<Sheets>, save: Res<Save>, time: Res<Time<Real>>,
         mut run: ResMut<Run>, mut units: Query<(&Unit, &mut Transform, Option<&Children>)>, mut anims: Query<&mut Anim>,
         mut fills: Query<(&OutroFill, &mut Sprite, &mut Transform), Without<Unit>>, els: Query<(Entity, &OutroEl)>,
         mut sfx: ResMut<crate::audio::Sfx>, mut next: ResMut<NextState<AppState>>, mut notice: ResMut<NewHeroNotice>) {
    let Some(mut o) = run.outro.take() else { return };
    let dt = time.delta_secs();
    o.t += dt;
    let slot_pos = |s: usize| [Vec2::new(80.0, -53.0), Vec2::new(0.0, -3.0), Vec2::new(-80.0, -53.0), Vec2::new(0.0, -103.0)][s.min(3)] * 2.0;
    let walk_off = o.stage >= 4;
    for b in &o.bars {
        let Some(e) = b.unit else { continue };
        let Ok((u, mut t, ch)) = units.get_mut(e) else { continue };
        if u.dead { continue; }
        let goal = if walk_off { Vec2::new(800.0, slot_pos(b.slot).y) } else { slot_pos(b.slot) };
        let g = goal - t.translation.truncate();
        let rig = ch.and_then(|c| c.iter().find(|c| anims.get(*c).is_ok()));
        if g.length() > 3.0 {
            t.translation += (g.normalize() * (160.0 * dt).min(g.length())).extend(0.0);
            play(&mut anims, rig, Row::Move, false, Some(g.x < 0.0));
        } else { play(&mut anims, rig, Row::Standby, false, Some(false)); }
    }
    let clear = |cmd: &mut Commands, st: u8| for (e, el) in &els { if el.0 == st { cmd.entity(e).despawn(); } };
    let z = 60.0;
    // stage times: banner 0.5, results 3.0, exp 5.0, walk off 7.5, leave 9.5 (a loss: banner then leave at 4.0)
    let advance = match o.stage { 0 => 0.5, 1 => if o.won { 3.0 } else { 4.0 }, 2 => 5.0, 3 => 7.5, _ => 9.5 };
    if o.t >= advance {
        match o.stage {
            0 => {
                sfx.play(if o.won { "sfx_win" } else { "sfx_lose" });
                let e = image(&mut cmd, &a, if o.won { "ui/STAGECLEAR.png" } else { "ui/GAMEOVER.png" }, Vec2::new(0.0, 60.0), None, z);
                cmd.entity(e).insert(OutroEl(1));
            }
            1 if !o.won => { next.set(AppState::WorldMap); }
            1 => {
                clear(&mut cmd, 1);
                let mut put = |cmd: &mut Commands, e: Entity| { cmd.entity(e).insert(OutroEl(2)); };
                let has_loot = !o.loot.is_empty();
                let im = |cmd: &mut Commands, p: &str, y: f32| image(cmd, &a, p, Vec2::new(0.0, y * 2.0), None, z);
                // write (8x8 white) stretched to 480x320 and tinted 0x78000000: black at 47% behind the Results panel
                let mut w = Sprite::from_image(a.load("ui/write.png")); w.custom_size = Some(SCREEN * 1.2); w.color = Color::srgba(0.0, 0.0, 0.0, 120.0 / 255.0);
                let e = cmd.spawn((w, Transform::from_xyz(0.0, 0.0, z - 1.0), Scr)).id(); put(&mut cmd, e);
                if o.new_hero.is_some() { notice.0 = true; }
                let e = im(&mut cmd, "ui/results.png", 130.0); put(&mut cmd, e);
                let (f1, gl, gv, f2) = if has_loot { (100.0, 80.0, 60.0, 40.0) } else { (40.0, 20.0, 0.0, -20.0) };
                let e = im(&mut cmd, "ui/fengetiao.png", f1); put(&mut cmd, e);
                let e = im(&mut cmd, "ui/gold.png", gl); put(&mut cmd, e);
                let e = text(&mut cmd, &f, format!("{}", o.money), Vec2::new(0.0, gv * 2.0), 30.0, Color::srgb(1.0, 0.62, 0.12), z); put(&mut cmd, e);
                let e = im(&mut cmd, "ui/fengetiao.png", f2); put(&mut cmd, e);
                if has_loot {
                    let e = im(&mut cmd, "ui/loot.png", 20.0); put(&mut cmd, e);
                    let n = o.loot.len() as f32;
                    for (i, &it) in o.loot.iter().enumerate() {
                        if let Some(p) = crate::common::item_icon_path(&data, it) {
                            let e = image(&mut cmd, &a, &p, Vec2::new((i as f32 - (n - 1.0) * 0.5) * 80.0, -20.0), Some(Vec2::new(74.0, 70.0)), z); put(&mut cmd, e);
                        }
                    }
                    let e = im(&mut cmd, "ui/fengetiao.png", -40.0); put(&mut cmd, e);
                }
                if let Some(h) = o.new_hero {
                    let e = im(&mut cmd, "ui/newhero.png", -60.0); put(&mut cmd, e);
                    let oh = Save::new_hero(h, None);
                    let e = hero_figure(&mut cmd, &data, &sheets, &oh, Vec2::new(0.0, -300.0), 130.0, z); put(&mut cmd, e);
                }
            }
            2 => {
                clear(&mut cmd, 2);
                for (i, b) in o.bars.iter().enumerate() {
                    let alive = b.unit.and_then(|e| units.get(e).ok()).map(|(u, _, _)| !u.dead).unwrap_or(false);
                    if !alive { continue; }
                    let p = slot_pos(b.slot);
                    let e = text(&mut cmd, &f, format!("+{}", o.exp), p + Vec2::new(0.0, 160.0), 28.0, Color::WHITE, z); cmd.entity(e).insert(OutroEl(3));
                    let e = image(&mut cmd, &a, "ui/S02.png", p, Some(Vec2::new(150.0, 42.0)), z); cmd.entity(e).insert(OutroEl(3));
                    let e = image(&mut cmd, &a, "ui/S01.png", p, Some(Vec2::new(0.0, 26.0)), z + 0.1); cmd.entity(e).insert((OutroEl(3), OutroFill(i)));
                    let e = image(&mut cmd, &a, "ui/S03.png", p, Some(Vec2::new(150.0, 42.0)), z + 0.2); cmd.entity(e).insert(OutroEl(3));
                }
            }
            3 => { clear(&mut cmd, 3); }
            _ => { next.set(AppState::WorldMap); }
        }
        o.stage += 1;
    }
    // exp fill: from the old ratio to the new one over 1.5 s, wrapping once per level gained (sfx_levelup each wrap)
    if o.stage == 3 {
        let k = ((o.t - 5.0) / 1.5).clamp(0.0, 1.0);
        for (fi, mut s, mut t) in &mut fills {
            let Some(b) = o.bars.get(fi.0) else { continue };
            let total = b.levels as f32 + b.to - b.from;
            let v = b.from + total * k;
            let wraps = v.floor().max(0.0) as u32;
            if wraps > o.lv_played { o.lv_played = wraps; sfx.play("sfx_levelup"); }
            let r = if b.levels > 0 && k >= 1.0 { b.to } else { v - v.floor() };
            let p = slot_pos(b.slot);
            s.custom_size = Some(Vec2::new(124.0 * r, 26.0));
            t.translation.x = p.x + (124.0 * r - 124.0) * 0.5;
        }
    }
    let _ = &save;
    run.outro = Some(o);
}

/// Set by the outro when a camp hero was found; the world map then shows the original notice
/// "A new hero has joined your cause! Go to the Team to use him!" (UserNoticeFrameGameObject: write + tanchuang01 + button_ok).
#[derive(Resource, Default)]
pub struct NewHeroNotice(pub bool);

/// SceneGameOver replaces the battle UI: everything but the field, the units' bodies and the outro's own sprites is hidden
/// (HUD, portraits, skill buttons, speed/pause, HP bars, names, selection rings, order marks).
fn outro_hide(run: Res<Run>, mut top: Query<(&mut Visibility, &Transform), (With<Scr>, Without<Unit>, Without<OutroEl>, Without<ChildOf>)>,
              units: Query<&Children, With<Unit>>, mut kids: Query<(&mut Visibility, &Transform), (Without<Scr>, Without<OutroEl>, Without<Anim>, Without<Unit>)>) {
    if run.outro.is_none() { return; }
    for (mut v, t) in &mut top { if t.translation.z > 0.5 && *v != Visibility::Hidden { *v = Visibility::Hidden; } }
    for ch in &units { for c in ch.iter() { if let Ok((mut v, _)) = kids.get_mut(c) { if *v != Visibility::Hidden { *v = Visibility::Hidden; } } } }
}
