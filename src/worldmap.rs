//! SceneMoveMap — the world map (worldmap_0..3 stitched → 1025×623, drawn at 2×), nodes from WorldMoveMap.xml
//! (position, BOSS_dian_* icon, unlock order, level range, map list). Free roam: the party walks wherever you click;
//! inside an area (radius) random encounters fire after `begindis` px at `propperpixel`‰ per px, the area centre
//! (bossradius) is its boss camp, and <Task>/<RefreshHero> mission and hero-camp markers start their own battles. BOSS/ARENA/SIEGE top-right,
//! SUMMON/INN/STORE/SKILLS/TEAM/STASH bottom bar (button_*0/1 textures).
use crate::{anim::*, battle::BattleSetup, data::GameData, save::Save, ui::*, AppState};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.init_resource::<Roam>().add_systems(OnEnter(AppState::WorldMap), enter)
        .add_systems(Update, (popup_close, actions, walk, camera_pan, fade, mark_anim).chain().run_if(in_state(AppState::WorldMap)))
        .add_systems(OnExit(AppState::WorldMap), despawn_screen);
}

#[derive(Component)]
struct MapRoot;
/// Node caption with its unclamped map-space position; kept inside the visible frame while its node is on screen.
#[derive(Component)]
struct NodeLabel(Vec2);
#[derive(Component)]
struct Walker { dest: Option<Vec2> }

fn node_pos(data: &GameData, i: u32) -> Vec2 {
    let n = &data.world.nodes[i as usize];
    Vec2::new((n.x - data.world.width / 2.0) * 2.0, (data.world.height / 2.0 - n.y) * 2.0)
}

fn enter(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, data: Res<GameData>, sheets: Res<Sheets>, save: Res<Save>, mut roam: ResMut<Roam>, mut notice: ResMut<crate::battle::NewHeroNotice>) {
    if std::mem::take(&mut notice.0) { new_hero_popup(&mut cmd, &a, &f); }
    let root = cmd.spawn((Transform::default(), Visibility::default(), Scr, MapRoot)).id();
    let mut s = Sprite::from_image(a.load("ui/worldmap.jpg"));
    s.custom_size = Some(Vec2::new(data.world.width, data.world.height) * 2.0);
    let bg = cmd.spawn((s, Transform::from_xyz(0.0, 0.0, 0.0))).id();
    cmd.entity(root).add_child(bg);
    // fog: mapcover_NNN (BossInfo coverImg) is the map darkened except for the areas opened up to node NNN
    let cover = save.unlocked.max(1).min(13);
    let mut fog = Sprite::from_image(a.load(format!("ui/mapcover_{cover:03}.png")));
    fog.custom_size = Some(Vec2::new(data.world.width, data.world.height) * 2.0);
    let fe = cmd.spawn((fog, Transform::from_xyz(0.0, 0.0, 0.5))).id();
    cmd.entity(root).add_child(fe);
    for n in &data.world.nodes {
        let p = node_pos(&data, n.index);
        let unlocked = open(&save, n.index);
        let icon = if a.get_handle::<Image>(format!("ui/{}.png", n.icon)).is_some() || crate::data::asset_exists(format!("ui/{}.png", n.icon)) { format!("ui/{}.png", n.icon) } else { "ui/BOSS_dian_001.png".into() };
        let mut s = Sprite::from_image(a.load(icon));
        s.custom_size = Some(Vec2::new(110.0, 96.0));
        if !unlocked { s.color = Color::srgb(0.35, 0.35, 0.35); }
        let e = cmd.spawn((s, Transform::from_translation((p + Vec2::Y * 30.0).extend(1.0)),
            Hot { size: Vec2::new(110.0, 100.0), act: Act::Custom(1, n.index as i32), normal: None, pressed: None })).id();
        cmd.entity(root).add_child(e);
        // unlocked: level range under the node. locked: original map2_locked padlock over the icon (no "Locked" text —
        // that label overlapped neighbouring nodes).
        if unlocked {
            let lvl = if n.max_level > 0 { format!("Lv{}-{}", n.min_level, n.max_level) } else { "Lv1".into() };
            let t = cmd.spawn((Text2d::new(lvl), TextFont { font: f.main.clone().into(), font_size: bevy::text::FontSize::Px(18.0), ..default() },
                TextColor(Color::WHITE), Transform::from_translation((p + Vec2::new(0.0, -28.0)).extend(2.0)), NodeLabel(p + Vec2::new(0.0, -28.0)))).id();
            cmd.entity(root).add_child(t);
        } else {
            let mut lk = Sprite::from_image(a.load("ui/map2_locked.png"));
            lk.custom_size = Some(Vec2::new(48.0, 54.0));
            let t = cmd.spawn((lk, Transform::from_translation((p + Vec2::Y * 30.0).extend(1.5)), NodeLabel(p + Vec2::Y * 30.0))).id();
            cmd.entity(root).add_child(t);
        }
    }
    // hero token: the party leader, walking
    let lead = save.party.first().map(|&r| save.heroes[r].hero).unwrap_or(0);
    let start = roam.pos.unwrap_or_else(|| { let (x, y) = data.world.begin; if x > 0.0 { to_world(&data, x, y) } else { node_pos(&data, 0) } });
    roam.pos = Some(start);
    roam.pending = None;
    refill_marks(&data, &save, &mut roam);
    for (p, kind, _) in &roam.marks {
        // UserMoveMapTaskNode::initNode: just the looping CompAnimObj (WorldMoveMap.xml animation = Renwu_mark skull for <Task>,
        // Yinxiong_mark tent for <RefreshHero>); no glow or label
        let (img, w, h) = if *kind == MarkKind::Task { ("ui/Renwu_mark_sheet.png", 74.0, 64.0) } else { ("ui/Yinxiong_mark_sheet.png", 82.0, 76.0) };
        let mut sp = Sprite::from_image(a.load(img)); sp.rect = Some(Rect::new(0.0, 0.0, w, h));
        let m = cmd.spawn((sp, Transform::from_translation(p.extend(3.5)), MarkSprite, MarkAnim(0.0, w, h))).id();
        cmd.entity(root).add_child(m);
    }
    let w = cmd.spawn((Transform::from_translation(start.extend(5.0)), Visibility::default(), Walker { dest: None })).id();
    cmd.entity(root).add_child(w);
    match save.party.first().map(|&r| &save.heroes[r]) {
        Some(oh) => { crate::anim::attach_rig(&mut cmd, w, &hero_key(&data, lead), crate::anim::hero_look(&data, oh.hero, Some(&oh.equip)), 1.0, 0.0); }
        None => { attach_sheet(&mut cmd, &data, &sheets, w, &hero_key(&data, lead), 1.0, 0.0); }
    }
    // HUD (fixed on screen)
    // BOSS / ARENA / SIEGE in a row top-right, as in the original's SceneMoveMap
    // Mode row: skip a text label when the texture already bakes the word in (bar_map2 = SIEGE).
    let hud = [("ui/BOSS_dian_015.png", Vec2::new(200.0, 270.0), Act::Custom(2, 0), "BOSS"),
               ("ui/arena_icon.png", Vec2::new(305.0, 270.0), Act::Goto(AppState::Arena), "ARENA"),
               ("ui/bar_map2.png", Vec2::new(410.0, 270.0), Act::Goto(AppState::Siege), ""),
               ("ui/worldmap_spot_9.png", Vec2::new(95.0, 270.0), Act::Custom(7, 0), "SURVIVAL")];
    for (img, p, act, lbl) in hud {
        button(&mut cmd, &a, img, None, p, Vec2::new(72.0, 60.0), 50.0, act);
        if !lbl.is_empty() {
            text(&mut cmd, &f, lbl, p + Vec2::new(0.0, -40.0), 13.0, Color::srgb(1.0, 0.95, 0.8), 51.0);
        }
    }
    let bar = [("summon", Act::Goto(AppState::Summon)), ("inn", Act::Goto(AppState::Tavern)), ("shop", Act::Goto(AppState::Store)),
               ("skill", Act::Goto(AppState::Skills)), ("team", Act::Goto(AppState::Equip)), ("stash", Act::Goto(AppState::Equip))];
    for (i, (n, act)) in bar.into_iter().enumerate() {
        let p = Vec2::new(-325.0 + 130.0 * i as f32, -262.0);
        button(&mut cmd, &a, &format!("ui/button_{n}0.png"), Some(&format!("ui/button_{n}1.png")), p, Vec2::new(115.0, 114.0), 50.0, act);
    }
    // the scroll top-left: quests in the original — here returns to the main menu
    button(&mut cmd, &a, "ui/bar_menu.png", Some("ui/bar_menu_0.png"), Vec2::new(-400.0, 280.0), Vec2::new(146.0, 78.0), 50.0, Act::Goto(AppState::Menu));
    image(&mut cmd, &a, "ui/coin.png", Vec2::new(-280.0, 280.0), None, 50.0);
    let g = text(&mut cmd, &f, "", Vec2::new(-220.0, 280.0), 24.0, Color::WHITE, 50.0);
    cmd.entity(g).insert(crate::common::GoldText);
}

pub fn open(save: &Save, i: u32) -> bool { i <= save.unlocked.max(1) }

/// Map-image pixel (y down) → map-root space (2×, centre origin, y up), and back.
fn to_world(data: &GameData, x: f32, y: f32) -> Vec2 { Vec2::new((x - data.world.width / 2.0) * 2.0, (data.world.height / 2.0 - y) * 2.0) }
fn to_px(data: &GameData, p: Vec2) -> Vec2 { Vec2::new(p.x / 2.0 + data.world.width / 2.0, data.world.height / 2.0 - p.y / 2.0) }

/// Free-roam state of SceneMoveMap, kept while the game runs (position, walked distance, mission/hero markers).
#[derive(Resource, Default)]
pub struct Roam { pub pos: Option<Vec2>, walked: f32, /// last whole-pixel excess used for the d100 check (UserMoveMapNode::OnUpdate)
    walk_check: i32, boss_armed: bool, marks: Vec<(Vec2, MarkKind, u32)>, pending: Option<(f32, BattleSetup, String)>,
    /// SceneMoveMap fight_retreat prompt: the battle on offer and the marker to put back on RETREAT
    prompt: Option<(BattleSetup, Option<(Vec2, MarkKind, u32)>)> }
#[derive(Component)]
struct PromptEl;
#[derive(Clone, Copy, PartialEq)]
enum MarkKind { Task, Hero }
#[derive(Component)]
struct MarkSprite;
#[derive(Component)]
struct MarkAnim(f32, f32, f32);
/// the 21-frame mark animations, looped (own design: 15 fps)
fn mark_anim(time: Res<Time>, mut q: Query<(&mut MarkAnim, &mut Sprite)>) {
    for (mut m, mut s) in &mut q {
        m.0 += time.delta_secs();
        let f = ((m.0 * 15.0) as u32 % 21) as f32;
        s.rect = Some(Rect::new(f * m.1, 0.0, (f + 1.0) * m.1, m.2));
    }
}
#[derive(Component)]
struct Banner;
#[derive(Component)]
struct Fade(f32);

fn fade(mut cmd: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Fade, &mut Sprite)>) {
    for (e, mut f, mut s) in q.iter_mut() { f.0 -= time.delta_secs(); s.color = s.color.with_alpha((f.0 / 0.6).clamp(0.0, 1.0)); if f.0 <= 0.0 { cmd.entity(e).despawn(); } }
}

/// Area (node) whose encounter radius contains map pixel `px`.
fn area_at<'a>(data: &'a GameData, px: Vec2) -> Option<&'a crate::data::MapNode> {
    data.world.nodes.iter().filter(|n| n.radius > 0.0 && !n.maps.is_empty() && px.distance(Vec2::new(n.x, n.y)) <= n.radius)
        .min_by(|a, b| px.distance(Vec2::new(a.x, a.y)).total_cmp(&px.distance(Vec2::new(b.x, b.y))))
}

/// Refill mission (<Task>, tent) and hero-camp (<RefreshHero>) markers: random points inside unlocked areas near the party.
fn refill_marks(data: &GameData, save: &Save, roam: &mut Roam) {
    let here = to_px(data, roam.pos.unwrap_or_default());
    for (kind, maps, dist, want) in [(MarkKind::Task, &data.world.task_maps, data.world.task_dist, 1), (MarkKind::Hero, &data.world.hero_maps, data.world.hero_dist, 1)] {
        if maps.is_empty() { continue; }
        let mut tries = 0;
        while roam.marks.iter().filter(|m| m.1 == kind).count() < want && tries < 200 {
            tries += 1;
            // UserMoveMapInfoGameObject: random angle, distance 25 + rand(maxdisance) from the party
            let ang = fastrand::f32() * std::f32::consts::TAU;
            let c = here + Vec2::new(ang.cos(), ang.sin()) * (25.0 + fastrand::f32() * dist);
            let Some(n) = area_at(data, c) else { continue };
            if !open(save, n.index) || c.distance(Vec2::new(n.x, n.y)) < 45.0 { continue; }
            // GuideConfig <FirstRefreshHero mapindex=113 level=1>: the first hero camp is map 113 (drops the Archer)
            let first = kind == MarkKind::Hero && save.heroes.iter().all(|o| o.hero != 5);
            roam.marks.push((to_world(data, c.x, c.y), kind, if first { 113 } else { maps[fastrand::usize(..maps.len())] }));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn actions(mut cmd: Commands, a: Res<AssetServer>, mut clicked: ResMut<Clicked>, data: Res<GameData>, save: Res<Save>, mut setup: ResMut<BattleSetup>,
           mut next: ResMut<NextState<AppState>>, mut q: Query<&mut Walker>, mut toast: ResMut<Toast>, k: Res<ButtonInput<KeyCode>>,
           mouse: Res<ButtonInput<MouseButton>>, cur: Res<Cursor>, hots: Query<(&Hot, &GlobalTransform, &InheritedVisibility)>,
           root: Query<&Transform, With<MapRoot>>, mut roam: ResMut<Roam>, (time, prompt_q): (Res<Time>, Query<Entity, With<PromptEl>>)) {
    // "Monsters encountered!" banner, then the battle
    // fight_retreat prompt: FIGHT starts the battle, RETREAT (or Esc) closes it and leaves the party where it is
    if roam.prompt.is_some() {
        let mut choice = None;
        for act in clicked.0.drain(..) { match act { Act::Custom(20, _) => choice = Some(true), Act::Custom(21, _) => choice = Some(false), _ => {} } }
        if k.just_pressed(KeyCode::Escape) { choice = Some(false); }
        if let Some(fight) = choice {
            for e in prompt_q.iter() { cmd.entity(e).despawn(); }
            let (s, mark) = roam.prompt.take().unwrap();
            if fight { *setup = s; next.set(AppState::Battle); }
            else { roam.walked = 0.0; roam.walk_check = 0; if let Some(m) = mark { roam.marks.push(m); } }
        }
        return;
    }
    if k.just_pressed(KeyCode::Escape) { next.set(AppState::Tavern); }
    if let Some((t, s, _)) = &mut roam.pending {
        *t -= time.delta_secs();
        if *t <= 0.0 { *setup = s.clone(); roam.pending = None; next.set(AppState::Battle); }
        clicked.0.clear();
        return;
    }
    let Ok(mut w) = q.single_mut() else { return };
    for act in clicked.0.drain(..) {
        match act {
            Act::Custom(1, i) => {
                let i = i as u32;
                if !open(&save, i) { toast.0 = Some(("Locked - defeat the previous area first".into(), 2.0)); continue; }
                let n = &data.world.nodes[i as usize];
                w.dest = Some(to_world(&data, n.x, n.y));
            }
            Act::Custom(7, _) => {
                start_battle(&mut roam, BattleSetup { node: 0, map: data.world.nodes.get(1).map(|n| n.maps[0]).unwrap_or(0), level: (0, 0), boss: false, arena: None, tutorial: false, siege: None, survival: true }, "SurvivalMode", &mut cmd, &a);
            }
            Act::Custom(2, _) => {
                let px = to_px(&data, roam.pos.unwrap_or_default());
                match area_at(&data, px).filter(|n| open(&save, n.index) && n.boss_level > 0) {
                    Some(n) => w.dest = Some(to_world(&data, n.x, n.y)),
                    None => toast.0 = Some(("Walk into an area to find its boss".into(), 2.0)),
                }
            }
            _ => {}
        }
    }
    // click on open ground: walk there (free roam inside the map's walkable rectangle)
    if mouse.just_pressed(MouseButton::Left) {
        if let (Some(p), Ok(r)) = (cur.world, root.single()) {
            let over_ui = hots.iter().any(|(h, gt, v)| v.get() && (p - gt.translation().truncate()).abs().cmple(h.size / 2.0).all());
            if !over_ui {
                let (x0, y0, x1, y1) = data.world.bounds;
                let px = to_px(&data, p - r.translation.truncate());
                let px = if x1 > x0 { Vec2::new(px.x.clamp(x0, x1), px.y.clamp(y0, y1)) } else { px };
                w.dest = Some(to_world(&data, px.x, px.y));
                let m = cmd.spawn((Sprite { image: a.load("ui/arrow_move.png"), custom_size: Some(Vec2::new(60.0, 30.0)), ..default() },
                    Transform::from_translation((p).extend(40.0)), Scr, Fade(0.6))).id();
                let _ = m;
            }
        }
    }
}

/// Pre-battle panel: the area's normalShow parchment (Monsters_encountered "Average level:", Hero_Encountered "Enemy Level:",
/// Boss_Encountered_N "Boss Level:") with button_fight / button_RETREAT (SceneMoveMap::OnLoad creates both buttons).
fn offer_battle(roam: &mut Roam, setup: BattleSetup, mark: Option<(Vec2, MarkKind, u32)>, banner: &str, cmd: &mut Commands, a: &AssetServer, f: &Fonts, data: &GameData) {
    roam.walked = 0.0; roam.walk_check = 0;
    let avg = (setup.level.0 + setup.level.1) as f32 / 2.0;
    let dim = cmd.spawn((Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.5), SCREEN * 1.3), Transform::from_xyz(0.0, 0.0, 82.0), Scr, PromptEl)).id();
    let _ = dim;
    let c = Vec2::new(0.0, 62.0);
    let p = image(cmd, a, &format!("ui/{banner}.png"), c, None, 83.0);
    cmd.entity(p).insert(PromptEl);
    // the level goes after the panel's "...Level:" caption (measured on the 358×374 textures)
    let dx = if banner.starts_with("Hero") { 50.0 } else if banner.starts_with("Boss") { 40.0 } else { 48.0 };
    let lv = if avg.fract() == 0.0 { format!("{avg:.0}") } else { format!("{avg:.1}") };
    let t = text(cmd, f, lv, c + Vec2::new(dx, 126.0), 24.0, Color::srgb(1.0, 0.95, 0.75), 84.0);
    cmd.entity(t).insert((PromptEl, bevy::sprite::Anchor::CENTER_LEFT));
    // Usually Drops: the parchment bakes in "Usually Drops: ???" + a ? box; overlay real icons from
    // WorldMoveMap.xml <FallDown> (normalEquipIndex / bossEquipIndex) over that row.
    let node = data.world.nodes.get(setup.node as usize);
    let drops: Vec<i32> = node.map(|n| {
        let src = if setup.boss || banner.starts_with("Boss") { &n.boss_drops } else { &n.normal_drops };
        src.iter().copied().filter(|&id| data.item(id).is_some()).collect()
    }).unwrap_or_default();
    // pick up to 5 distinct icons; centre them on the parchment's drops row (~y = c.y - 10)
    let mut shown = Vec::new();
    for id in drops {
        if shown.contains(&id) { continue; }
        shown.push(id);
        if shown.len() == 5 { break; }
    }
    if !shown.is_empty() {
        let n = shown.len() as f32;
        let base = c + Vec2::new(20.0, 18.0);
        // cover the parchment's baked-in "???" + ? box so only the real icons show
        let blot = cmd.spawn((Sprite::from_color(Color::srgb(0.72, 0.55, 0.32), Vec2::new(n * 58.0 + 40.0, 56.0)), Transform::from_translation(base.extend(84.2)), Scr, PromptEl)).id();
        let _ = blot;
        for (i, id) in shown.into_iter().enumerate() {
            if let Some(path) = crate::common::item_icon_path(data, id) {
                let x = base.x + (i as f32 - (n - 1.0) / 2.0) * 58.0;
                let mut sp = Sprite::from_image(a.load(path));
                sp.custom_size = Some(Vec2::splat(48.0));
                let e = cmd.spawn((sp, Transform::from_translation(Vec3::new(x, base.y, 84.5)), Scr, PromptEl)).id();
                let _ = e;
            }
        }
    }
    for (img, x, act) in [("button_fight", -80.0, 20), ("button_RETREAT", 80.0, 21)] {
        let b = button(cmd, a, &format!("ui/{img}.png"), Some(&format!("ui/{img}_0.png")), Vec2::new(x, -160.0), Vec2::new(140.0, 60.0), 84.0, Act::Custom(act, 0));
        cmd.entity(b).insert(PromptEl);
    }
    roam.prompt = Some((setup, mark));
}

/// Average level of the battle party (the base for mission and hero-camp levels, see `walk`).
fn party_level(save: &Save) -> i32 {
    let v: Vec<i32> = save.party.iter().filter_map(|&r| save.heroes.get(r)).map(|o| o.level).collect();
    if v.is_empty() { 1 } else { (v.iter().sum::<i32>() as f32 / v.len() as f32).round() as i32 }
}

fn start_battle(roam: &mut Roam, setup: BattleSetup, banner: &str, cmd: &mut Commands, a: &AssetServer) {
    roam.walked = 0.0; roam.walk_check = 0;
    roam.pending = Some((1.3, setup, banner.into()));
    cmd.spawn((Sprite::from_image(a.load(format!("ui/{banner}.png"))), Transform::from_xyz(0.0, 60.0, 80.0), Scr, Banner));
}

#[allow(clippy::too_many_arguments)]
fn walk(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, time: Res<Time>, data: Res<GameData>, mut save: ResMut<Save>, mut roam: ResMut<Roam>,
        mut q: Query<(&mut Walker, &mut Transform, &Children)>, mut an: Query<&mut Anim>, mut toast: ResMut<Toast>,
        marks: Query<Entity, With<MarkSprite>>, root: Query<Entity, With<MapRoot>>) {
    if roam.pending.is_some() || roam.prompt.is_some() { return; }
    let Ok((mut w, mut t, ch)) = q.single_mut() else { return };
    let set_anim = |an: &mut Query<&mut Anim>, row: Row, flip: Option<bool>| for c in ch.iter() { if let Ok(mut a) = an.get_mut(c) { a.play(row, false); if let Some(f) = flip { a.flip = f; } } };
    let Some(dest) = w.dest else { set_anim(&mut an, Row::Standby, None); return };
    let pos = t.translation.truncate();
    let d = dest - pos;
    // Map movespeed="20": taken as 2×movespeed map pixels per second (×2 to map-root space), so the
    // walk cycle (12 fps) roughly matches the ground covered; the old 260/s was about 3× too fast.
    let step = data.world.move_speed.max(5.0) * 2.0 * 2.0 * time.delta_secs();
    set_anim(&mut an, Row::Move, if d.x.abs() > 1.0 { Some(d.x < 0.0) } else { None });
    let np = if d.length() <= step { w.dest = None; dest } else { pos + d.normalize() * step };
    let px = to_px(&data, np);
    // locked areas are closed off
    if let Some(n) = data.world.nodes.iter().find(|n| n.radius > 0.0 && !open(&save, n.index) && px.distance(Vec2::new(n.x, n.y)) <= n.radius * 0.8) {
        w.dest = None; toast.0 = Some((format!("Locked area (Lv{}-{})", n.min_level, n.max_level), 2.0)); let _ = n; return;
    }
    let moved = (np - pos).length() / 2.0; // in map pixels
    t.translation = np.extend(5.0);
    roam.pos = Some(np);
    let area = area_at(&data, px);
    if let Some(n) = area { if save.current_node != n.index { save.current_node = n.index; } }
    // boss camp: reaching the area centre (bossradius) fights its boss
    let at_boss = area.filter(|n| n.boss_level > 0 && px.distance(Vec2::new(n.x, n.y)) <= n.boss_radius.max(12.0));
    match at_boss {
        Some(n) if roam.boss_armed => {
            roam.boss_armed = false; w.dest = None;
            let banner = if data.world.nodes.len() > 15 && n.index == 15 { "Hero_Encountered".to_string() } else { format!("Boss_Encountered_{}", n.index.clamp(1, 13)) };
            offer_battle(&mut roam, BattleSetup { node: n.index, map: n.boss_map, level: (n.boss_level, n.boss_level), boss: true, arena: None, tutorial: false, siege: None , survival: false }, None, &banner, &mut cmd, &a, &f, &data);
            save.write();
            return;
        }
        None => roam.boss_armed = true,
        _ => {}
    }
    // mission / hero-camp markers
    if let Some(k) = roam.marks.iter().position(|m| m.0.distance(np) <= 40.0) {
        let mark = roam.marks.remove(k);
        let (_, kind, map) = mark;
        let n = area.unwrap_or(&data.world.nodes[1]);
        // UserMoveMapInfoGameObject::OnGameEvent: a mission (control 10) is fought at base − levelchange + rand(2·levelchange + 1),
        // a hero camp (control 11) at base + levelchange (−3), both at least 1. The base is a runtime GameData value I could not
        // trace to its writer; it is taken here as the party's average level so both kinds keep pace with the party.
        let base = party_level(&save);
        let lc = if kind == MarkKind::Task { data.world.task_level } else { data.world.hero_level };
        let lvl = if kind == MarkKind::Task { base - lc + fastrand::i32(0..=2 * lc.abs()) } else { base + lc }.max(1);
        let first = map == 113;
        let map = if data.waves.maps.contains_key(&map) { map } else { n.maps[0] };
        w.dest = None;
        for e in marks.iter() { cmd.entity(e).despawn(); }
        offer_battle(&mut roam, BattleSetup { node: n.index, map, level: if first { (1, 1) } else { (lvl, lvl) }, boss: false, arena: None, tutorial: false, siege: None , survival: false },
            Some(mark), if kind == MarkKind::Task { "Monsters_encountered" } else { "Hero_Encountered" }, &mut cmd, &a, &f, &data);
        save.write();
        return;
    }
    // random encounters — UserMoveMapNode::OnUpdate: accumulate walk while inside an unlocked area; once walked >
    // begindis, each further whole pixel raises a d100 check by propperpixel (score = excess·propperpixel; fire when
    // score > lrand48()%100). Failed rolls keep the accumulator so the chance grows until a hit; a hit resets it.
    if let Some(n) = area.filter(|n| open(&save, n.index)) {
        roam.walked += moved;
        if roam.walked > n.begin_dis {
            let excess = (roam.walked - n.begin_dis).floor() as i32;
            if excess > 0 && roam.walk_check != excess {
                roam.walk_check = excess;
                let score = excess.saturating_mul(n.per_pixel as i32);
                if score > fastrand::i32(0..100) {
                    let maps: Vec<u32> = n.maps.iter().copied().filter(|m| data.waves.maps.contains_key(m)).collect();
                    if !maps.is_empty() {
                        roam.walked = 0.0; roam.walk_check = 0;
                        w.dest = None;
                        offer_battle(&mut roam, BattleSetup { node: n.index, map: maps[fastrand::usize(..maps.len())], level: (n.min_level, n.max_level), boss: false, arena: None, tutorial: false, siege: None , survival: false },
                            None, "Monsters_encountered", &mut cmd, &a, &f, &data);
                        save.write();
                    }
                }
            }
        }
    } else { roam.walked = 0.0; roam.walk_check = 0; }
    let _ = root;
}

/// Keep the walker centred, clamped to the map edges.
fn camera_pan(walker: Query<&Transform, (With<Walker>, Without<MapRoot>, Without<NodeLabel>)>, mut root: Query<&mut Transform, (With<MapRoot>, Without<NodeLabel>)>,
              mut labels: Query<(&NodeLabel, &mut Transform), (Without<MapRoot>, Without<Walker>)>, data: Res<GameData>) {
    let (Ok(w), Ok(mut r)) = (walker.single(), root.single_mut()) else { return };
    let half = Vec2::new(data.world.width, data.world.height) - SCREEN / 2.0; // map half-size(×2/2) − screen half
    let want = (-w.translation.truncate()).clamp(-half, half);
    r.translation = r.translation.truncate().lerp(want, 0.15).extend(0.0);
    let off = r.translation.truncate();
    for (l, mut t) in labels.iter_mut() {
        let g = l.0 + off;
        let edge = SCREEN.x / 2.0 - 55.0;
        let x = if g.x.abs() < SCREEN.x / 2.0 + 60.0 { g.x.clamp(-edge, edge) } else { g.x };
        let y = if g.y.abs() < SCREEN.y / 2.0 + 40.0 { g.y.clamp(-SCREEN.y / 2.0 + 15.0, SCREEN.y / 2.0 - 15.0) } else { g.y };
        // keep clear of the BOSS/ARENA/SIEGE row in the top-right corner
        let y = if x > 150.0 && y > 205.0 && y < SCREEN.y / 2.0 + 40.0 { 192.0 } else { y };
        t.translation.x = x - off.x; t.translation.y = y - off.y;
    }
}

#[derive(Component)]
struct NoticePopup;
/// UserNoticeFrameGameObject as SceneGameOver leaves it for the map: dim write overlay, tanchuang01 frame, the English
/// 3.4.3 string, button_ok. Own design: text size and the button's y inside the frame.
fn new_hero_popup(cmd: &mut Commands, a: &AssetServer, f: &Fonts) {
    let mut w = Sprite::from_image(a.load("ui/write.png")); w.custom_size = Some(SCREEN * 1.2); w.color = Color::srgba(0.0, 0.0, 0.0, 120.0 / 255.0);
    let mut es = vec![cmd.spawn((w, Transform::from_xyz(0.0, 0.0, 90.0), Scr)).id()];
    es.push(image(cmd, a, "ui/tanchuang01.png", Vec2::ZERO, None, 91.0));
    let t = text(cmd, f, "A new hero has joined your cause!\nGo to the Inn and tap HIRE to add them to your party!", Vec2::new(0.0, 40.0), 26.0, Color::WHITE, 92.0);
    es.push(t);
    es.push(button(cmd, a, "ui/button_ok.png", Some("ui/button_ok_0.png"), Vec2::new(0.0, -90.0), Vec2::new(140.0, 60.0), 92.0, Act::Custom(90, 0)));
    for e in es { cmd.entity(e).insert(NoticePopup); }
}

/// while the notice is up it takes every click (modal); button_ok or Enter closes it
fn popup_close(mut cmd: Commands, mut clicked: ResMut<Clicked>, k: Res<ButtonInput<KeyCode>>, popup: Query<Entity, With<NoticePopup>>) {
    if popup.is_empty() { return; }
    let close = clicked.0.drain(..).any(|c| matches!(c, Act::Custom(90, _))) || k.just_pressed(KeyCode::Enter);
    if close { for e in &popup { cmd.entity(e).despawn(); } }
}
