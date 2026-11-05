//! SceneSkillList — skill tree: portrait strip, BACK, LEVEL/POINTS banner, skill_bg panel with the hero's tree
//! (heroes.ron skill_tree: row = vertical 1..4, line = column 1..8), x/max badges (skillnum_green when maxed,
//! yellow when partly filled, white otherwise), skillToSkill arrows from prerequisites, dimmed locked skills,
//! description box and RESET (O_RESET). 1 point per hero level. Click a skill to describe it, click again to learn.
use crate::{anim::Sheets, common::*, data::GameData, save::Save, stats, types::SkillTreeNode, ui::*, AppState, Selected};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(AppState::Skills), enter)
        .add_systems(Update, logic.run_if(in_state(AppState::Skills)))
        .add_systems(OnExit(AppState::Skills), despawn_screen);
}

const PANEL: Vec2 = Vec2::new(0.0, 10.0);
fn node_pos(n: &SkillTreeNode) -> Vec2 { PANEL + Vec2::new(-367.0 + 105.0 * (n.line - 1) as f32, 135.0 - 85.0 * (n.row - 1) as f32) }

#[derive(Component)]
struct Tree;
#[derive(Resource, Default)]
struct SkSel(Option<i32>);

fn enter(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, data: Res<GameData>, sheets: Res<Sheets>, save: Res<Save>, mut sel: ResMut<Selected>) {
    if sel.0 >= save.heroes.len() { sel.0 = 0; }
    image(&mut cmd, &a, "ui/team_bg.png", Vec2::ZERO, Some(SCREEN), 0.0);
    let list: Vec<usize> = (0..save.heroes.len()).collect();
    portrait_strip(&mut cmd, &a, &data, &sheets, &save, sel.0, &list);
    back_and_gold(&mut cmd, &a, &f, &save, AppState::Tavern);
    image(&mut cmd, &a, "ui/skill_bg.png", PANEL, None, 1.0);
    image(&mut cmd, &a, "ui/button_level.png", Vec2::new(228.0, 250.0), Some(Vec2::new(150.0, 58.0)), 5.0); // SceneSkillList button_level scroll
    desc_box(&mut cmd, &a, &f, Vec2::new(-110.0, -245.0));
    button(&mut cmd, &a, "ui/resetskill.png", Some("ui/resetskill_0.png"), Vec2::new(400.0, -205.0), Vec2::new(102.0, 80.0), 5.0, Act::Custom(2, 0));
    cmd.insert_resource(SkSel(None));
    cmd.insert_resource(crate::store::Rebuild(true));
}

pub fn node_state(data: &GameData, oh: &crate::save::OwnedHero, n: &SkillTreeNode) -> (i32, bool) {
    let pts = oh.skills.get(&n.skill).copied().unwrap_or(0);
    let spent: i32 = oh.skills.values().sum();
    let tree = &data.hero(oh.hero).skill_tree;
    let mut ok = n.need_had_points < 0 || spent >= n.need_had_points;
    if n.before >= 0 { ok &= oh.skills.get(&n.before).copied().unwrap_or(0) >= n.need_before_points.max(1); }
    if let Some(fl) = n.exclusion_flag {
        ok &= !tree.iter().any(|o| o.skill != n.skill && o.exclusion_flag == Some(fl) && oh.skills.get(&o.skill).copied().unwrap_or(0) > 0);
    }
    (pts, ok)
}

#[allow(clippy::too_many_arguments)]
fn logic(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, data: Res<GameData>, mut save: ResMut<Save>, sel: Res<Selected>, mut sk: ResMut<SkSel>,
         mut clicked: ResMut<Clicked>, mut rb: ResMut<crate::store::Rebuild>, tree: Query<Entity, With<Tree>>, mut sfx: ResMut<crate::audio::Sfx>, mut toast: ResMut<Toast>,
         (mut qt, mut qb): (Query<&mut Text2d, (With<DescTitle>, Without<DescBody>)>, Query<&mut Text2d, (With<DescBody>, Without<DescTitle>)>),
         (k, mut next): (Res<ButtonInput<KeyCode>>, ResMut<NextState<AppState>>)) {
    if k.just_pressed(KeyCode::Escape) { next.set(AppState::Tavern); }
    let r = sel.0;
    let hero = save.heroes[r].hero;
    for act in clicked.0.drain(..) {
        match act {
            Act::Custom(PORTRAIT_ACT, h) => { cmd.insert_resource(Selected(h as usize)); next.set(AppState::Skills); }
            Act::Custom(2, _) => { save.heroes[r].skills.clear(); sfx.play("sfx_back"); save.write(); toast.0 = Some(("Skill points reset".into(), 2.0)); }
            Act::Custom(1, s) => {
                let n = data.hero(hero).skill_tree.iter().find(|n| n.skill == s).unwrap().clone();
                let (pts, ok) = node_state(&data, &save.heroes[r], &n);
                if sk.0 == Some(s) {
                    if !ok { toast.0 = Some(("Requirements not met".into(), 2.0)); sfx.play("sfx_wrong"); }
                    else if pts >= n.max_points { toast.0 = Some(("Skill is maxed".into(), 2.0)); }
                    else if stats::points_left(&save.heroes[r]) <= 0 { toast.0 = Some(("No skill points left — level up".into(), 2.0)); sfx.play("sfx_wrong"); }
                    else { *save.heroes[r].skills.entry(s).or_insert(0) += 1; sfx.play("sfx_levelup"); save.write(); }
                }
                sk.0 = Some(s);
                let (pts, _) = node_state(&data, &save.heroes[r], &n);
                if let Some(d) = stats::skill_at_points(&data, s, pts.max(1)) {
                    // [NOW]/[NEXT] are the original's markers for this rank / next rank
                    let mut body = d.introduction.replace("\\n", "\n").replace("[NOW]", "").replace("[NEXT]", "\nNext rank: ");
                    if let Some(cd) = d.cd_ms.filter(|&c| c > 0) { body += &format!("\nCooldown {:.0}s", cd as f32 / 1000.0); }
                    if n.need_had_points > 0 { body += &format!("  Requires {} points spent", n.need_had_points); }
                    let parent = data.skill(n.before).filter(|p| p.kind == crate::types::SkillKind::Active && d.kind == crate::types::SkillKind::Passive);
                    body += match (d.kind, parent) {
                        (crate::types::SkillKind::Active, _) => "\nActive: shows in the battle skill bar (top-left) once learned; tap it to cast.",
                        (_, Some(_)) => "\nUpgrade: improves its skill automatically, no button.",
                        _ => "\nPassive: always on.",
                    };
                    body += "  Tap again to learn.";
                    let kind = match parent { Some(p) => format!("Upgrade of {}", p.name), None if d.kind == crate::types::SkillKind::Active => "Active".into(), None => "Passive".into() };
                    set_desc(format!("{}  {}/{}  {}", d.name, pts, n.max_points, kind), body, &mut qt, &mut qb);
                }
            }
            _ => {}
        }
        rb.0 = true;
    }
    if !rb.0 { return; }
    rb.0 = false;
    if sk.0.is_none() {
        set_desc("Skills", "Tap a skill to read it, tap it again to spend a point (1 point per level).\nActive skills appear in the battle skill bar (top-left); passives and upgrades work on their own.", &mut qt, &mut qb);
    }
    for e in tree.iter() { cmd.entity(e).despawn(); }
    let oh = &save.heroes[r];
    let spawn = |cmd: &mut Commands, e: Entity| { cmd.entity(e).insert(Tree); };
    let t = text(&mut cmd, &f, format!("LEVEL: {}\nPOINTS: {}", oh.level, stats::points_left(oh)), Vec2::new(228.0, 250.0), 17.0, Color::srgb(0.3, 0.15, 0.05), 6.0);
    spawn(&mut cmd, t);
    for n in &data.hero(hero).skill_tree {
        let p = node_pos(n);
        let (pts, ok) = node_state(&data, oh, n);
        let Some(s) = data.skill(n.skill) else { continue };
        let icon = format!("icons/{}.png", s.icon);
        let path = if crate::data::asset_exists(&icon) { icon } else { "ui/item.png".into() };
        let mut sp = Sprite::from_image(a.load(path));
        sp.custom_size = Some(Vec2::splat(64.0));
        if !ok && pts == 0 { sp.color = Color::srgb(0.35, 0.35, 0.35); }
        let e = cmd.spawn((sp, Transform::from_translation(p.extend(3.0)), Scr, Hot { size: Vec2::splat(70.0), act: Act::Custom(1, n.skill), normal: None, pressed: None })).id();
        spawn(&mut cmd, e);
        if sk.0 == Some(n.skill) { let e = image(&mut cmd, &a, "ui/skill_selected.png", p, Some(Vec2::splat(80.0)), 3.5); spawn(&mut cmd, e); }
        let badge = if pts >= n.max_points { "ui/skillnum_green.png" } else if pts > 0 { "ui/skillnum_yellow.png" } else { "ui/skillnum_white.png" };
        let b = image(&mut cmd, &a, badge, p + Vec2::new(18.0, -34.0), None, 4.0); spawn(&mut cmd, b);
        let bt = text(&mut cmd, &f, format!("{pts}/{}", n.max_points), p + Vec2::new(18.0, -34.0), 15.0, Color::BLACK, 4.5); spawn(&mut cmd, bt);
        if n.before >= 0 {
            if let Some(pn) = data.hero(hero).skill_tree.iter().find(|o| o.skill == n.before) {
                let from = node_pos(pn); let d = p - from;
                let mut s = Sprite::from_image(a.load("ui/skillToSkill.png"));
                s.custom_size = Some(Vec2::new((d.length() - 70.0).max(14.0), 12.0));
                let e = cmd.spawn((s, Transform::from_translation(((from + p) / 2.0).extend(2.5)).with_rotation(Quat::from_rotation_z(d.y.atan2(d.x))), Scr)).id();
                spawn(&mut cmd, e);
            }
        }
    }
}
