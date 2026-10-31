//! SceneSelectHero — hire / dismiss heroes and choose the party (≤3).
//! bg_store sub-rect (25,83,431,220)@1x stretched to the screen; party strip (char_bg slots) at
//! UserSelectHeroGameObject (-108,130); hero list (team_bg pedestals, spacing 114) at (0,-31.5);
//! back (195,140), DISMISS (50,135), HIRE (110,135), coin (130,108) — all from SceneSelectHero::init.
use crate::{anim::*, data::GameData, save::Save, ui::*, AppState};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(AppState::Hire), enter)
        .add_systems(Update, (actions, scroll).run_if(in_state(AppState::Hire)))
        .add_systems(OnExit(AppState::Hire), despawn_screen);
}

#[derive(Resource, Default)]
struct HireSel { hero: i32, scroll: f32 }
#[derive(Component)]
struct ListRoot;
#[derive(Component)]
struct Dynamic;

const SPACING: f32 = 228.0;

fn enter(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>) {
    let mut s = Sprite::from_image(a.load("ui/bg_store.png"));
    s.rect = Some(Rect::new(50.0, 166.0, 912.0, 606.0));
    s.custom_size = Some(SCREEN);
    cmd.spawn((s, Transform::default(), Scr));
    button(&mut cmd, &a, "ui/back.png", Some("ui/back_0.png"), obj(195.0, 140.0), Vec2::new(180.0, 80.0), 5.0, Act::Goto(AppState::Tavern));
    button(&mut cmd, &a, "ui/button_DISMISS.png", Some("ui/button_DISMISS_0.png"), obj(50.0, 135.0), Vec2::new(102.0, 78.0), 5.0, Act::Custom(2, 0));
    button(&mut cmd, &a, "ui/button_HIRE.png", Some("ui/button_HIRE_0.png"), obj(110.0, 135.0), Vec2::new(102.0, 78.0), 5.0, Act::Custom(1, 0));
    image(&mut cmd, &a, "ui/coin.png", obj(130.0, 108.0), None, 5.0);
    let g = text(&mut cmd, &f, "", obj(165.0, 108.0), 26.0, Color::WHITE, 5.0);
    cmd.entity(g).insert(crate::common::GoldText);
    cmd.insert_resource(HireSel { hero: 0, scroll: 0.0 });
    cmd.spawn((Transform::from_translation(obj(0.0, -31.5).extend(1.0)), Visibility::default(), Scr, ListRoot));
}

fn rebuild(cmd: &mut Commands, a: &AssetServer, f: &Fonts, data: &GameData, sheets: &Sheets, save: &Save, sel: &HireSel, root: Entity, old: &Query<Entity, With<Dynamic>>) {
    for e in old.iter() { cmd.entity(e).despawn(); }
    // party strip
    let pc = obj(-108.0, 130.0);
    let strip = image(cmd, a, "ui/char_bg.png", pc, None, 3.0);
    cmd.entity(strip).insert(Dynamic);
    for i in 0..4 {
        let p = pc + Vec2::new(-198.0 + 132.0 * i as f32, -44.0);
        if i >= save.team_size as usize { // locked 4th slot (original BUY_4player, 50,000 gold)
            let l = image(cmd, a, "ui/map2_locked.png", p + Vec2::Y * 44.0, Some(Vec2::new(48.0, 54.0)), 5.0);
            let h = hotspot(cmd, p + Vec2::Y * 44.0, Vec2::new(120.0, 100.0), 6.0, Act::Custom(6, 0));
            cmd.entity(l).insert(Dynamic); cmd.entity(h).insert(Dynamic);
            continue;
        }
        if let Some(&r) = save.party.get(i) {
            let fig = crate::anim::hero_figure(cmd, data, sheets, &save.heroes[r], p, 84.0, 4.0);
            let h = hotspot(cmd, p + Vec2::Y * 44.0, Vec2::new(120.0, 100.0), 6.0, Act::Custom(3, i as i32));
            cmd.entity(fig).insert(Dynamic);
            cmd.entity(h).insert(Dynamic);
        }
    }
    let cov = image(cmd, a, "ui/char_cover.png", pc, None, 4.5);
    cmd.entity(cov).insert(Dynamic);
    // hero list on pedestals
    let list = hire_list(save);
    if list.is_empty() {
        let t = text(cmd, f, "No heroes to hire yet.\nBeat the heroes at Hero camps on the map to recruit them.", Vec2::new(0.0, 0.0), 22.0, Color::WHITE, 5.0);
        cmd.entity(t).insert(Dynamic);
    }
    for (i, &h) in list.iter().enumerate() {
        let x = (i as f32 - 1.5) * SPACING;
        let card = cmd.spawn((Transform::from_xyz(x, 0.0, 0.0), Visibility::default(), Dynamic)).id();
        cmd.entity(root).add_child(card);
        let mut bg = Sprite::from_image(a.load("ui/team_ul.png")); bg.custom_size = Some(Vec2::new(210.0, 300.0));
        let b = cmd.spawn((bg, Transform::from_xyz(0.0, 0.0, 0.0))).id();
        cmd.entity(card).add_child(b);
        {
            let fp = cmd.spawn((Transform::from_xyz(0.0, -90.0, 1.0), Visibility::default())).id();
            cmd.entity(card).add_child(fp);
            let k = hero_key(data, h);
            if attach_rig(cmd, fp, &k, hero_look(data, h, None), fit(data, &k, 120.0), 0.0).is_none() { attach_sheet(cmd, data, sheets, fp, &k, fit(data, &k, 120.0), 0.0); }
        }
        let owned = save.heroes.iter().filter(|o| o.hero == h).count();
        let hd = data.hero(h);
        let t = cmd.spawn((Text2d::new(format!("{}\n{} {}{}", hd.class_name, hd.price, "gold", if owned > 0 { format!("  ×{owned}") } else { String::new() })),
            TextFont { font: f.main.clone().into(), font_size: bevy::text::FontSize::Px(20.0), ..default() },
            TextColor(if sel.hero == h { Color::srgb(1.0, 0.9, 0.3) } else { Color::WHITE }), Transform::from_xyz(0.0, 110.0, 3.0))).id();
        cmd.entity(card).add_child(t);
        if sel.hero == h {
            let mut s = Sprite::from_image(a.load("ui/guangquan.png")); s.custom_size = Some(Vec2::new(150.0, 60.0));
            let r = cmd.spawn((s, Transform::from_xyz(0.0, -92.0, 0.5))).id();
            cmd.entity(card).add_child(r);
        }
        let hs = cmd.spawn((Transform::from_xyz(0.0, 0.0, 5.0), Visibility::default(), Hot { size: Vec2::new(200.0, 280.0), act: Act::Custom(4, h), normal: None, pressed: None })).id();
        cmd.entity(card).add_child(hs);
    }
    // roster (owned heroes not in party): click to add to party
    let mut x = -440.0;
    let lbl = text(cmd, f, "Hired heroes (click to add to party):", Vec2::new(-440.0, -205.0), 18.0, Color::WHITE, 5.0);
    cmd.entity(lbl).insert((Dynamic, bevy::sprite::Anchor::CENTER_LEFT));
    for (r, oh) in save.heroes.iter().enumerate() {
        let inp = save.party.contains(&r);
        let fig = crate::anim::hero_figure(cmd, data, sheets, oh, Vec2::new(x + 30.0, -300.0), 60.0, 5.0);
        let t = text(cmd, f, format!("Lv{}", oh.level), Vec2::new(x + 30.0, -230.0), 16.0, if inp { Color::srgb(0.4, 1.0, 0.4) } else { Color::WHITE }, 6.0);
        let h = hotspot(cmd, Vec2::new(x + 30.0, -270.0), Vec2::new(60.0, 80.0), 6.0, Act::Custom(5, r as i32));
        cmd.entity(fig).insert(Dynamic); cmd.entity(t).insert(Dynamic); cmd.entity(h).insert(Dynamic);
        x += 64.0;
        if x > 420.0 { break; }
    }
}

#[allow(clippy::too_many_arguments)]
fn actions(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, data: Res<GameData>, sheets: Res<Sheets>, mut save: ResMut<Save>, mut sel: ResMut<HireSel>,
           mut clicked: ResMut<Clicked>, root: Query<Entity, With<ListRoot>>, old: Query<Entity, With<Dynamic>>, mut toast: ResMut<Toast>,
           mut sfx: ResMut<crate::audio::Sfx>, (mut built, mut pend): (Local<bool>, Local<bool>), st: Res<State<AppState>>, k: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if k.just_pressed(KeyCode::Escape) { next.set(AppState::Tavern); }
    if st.is_changed() { *built = false; }
    let Ok(root) = root.single() else { return };
    let mut dirty = !*built;
    for act in clicked.0.drain(..) {
        let Act::Custom(c, v) = act else { continue };
        dirty = true;
        match c {
            1 => { // HIRE (only heroes found at hero camps: the original's unhired list)
                if !hire_list(&save).contains(&sel.hero) { toast.0 = Some(("Find and beat this hero at a Hero camp first".into(), 2.0)); sfx.play("sfx_wrong"); continue; }
                let price = data.hero(sel.hero).price as i64;
                if save.money() < price { toast.0 = Some(("Not enough gold".into(), 2.0)); sfx.play("sfx_wrong"); continue; }
                save.add_money(-price);
                let h = sel.hero; save.found.retain(|&x| x != h);
                let root_skill = data.hero(sel.hero).skill_tree.iter().find(|n| n.before < 0 && data.skill(n.skill).map(|s| s.kind == crate::types::SkillKind::Active).unwrap_or(false)).map(|n| n.skill);
                let mut oh = Save::new_hero(sel.hero, root_skill); crate::starter_weapon(&data, &mut oh); save.heroes.push(oh);
                let r = save.heroes.len() - 1;
                if save.party.len() < save.team_size as usize && !data.hero(sel.hero).super_hero && !save.party.iter().any(|&p| data.hero(save.heroes[p].hero).super_hero) { save.party.push(r); }
                sfx.play("sfx_buy");
                toast.0 = Some((format!("{} has joined!", data.hero(sel.hero).class_name), 2.0));
            }
            2 => { // DISMISS: the last owned hero of the selected class (no refund, items return to the stash)
                if save.heroes.len() <= 1 { toast.0 = Some(("You must keep at least one hero".into(), 2.0)); continue; }
                if let Some(r) = save.heroes.iter().rposition(|o| o.hero == sel.hero) {
                    let items: Vec<i32> = save.heroes[r].equip.iter().flatten().copied().collect();
                    for it in items { if let Some(s) = save.first_free_stash() { save.stash[s] = Some(it); } }
                    let cls = save.heroes[r].hero; if !save.found.contains(&cls) { save.found.push(cls); } // back on the hire list (own simplification of the original's "rest" list)
                    save.heroes.remove(r);
                    save.party.retain(|&p| p != r);
                    for p in save.party.iter_mut() { if *p > r { *p -= 1; } }
                    if save.party.is_empty() { save.party.push(0); }
                    sfx.play("sfx_back");
                }
            }
            3 => { if save.party.len() > 1 { save.party.remove(v as usize); } }
            4 => { sel.hero = v; sfx.play("sfx_select"); }
            5 => {
                let r = v as usize;
                if let Some(i) = save.party.iter().position(|&p| p == r) { if save.party.len() > 1 { save.party.remove(i); } }
                // SceneSelectHero: "If you use a legendary hero, you won't be able to use any other heroes." (superHero=1 fights alone)
                else if data.hero(save.heroes[r].hero).super_hero { save.party = vec![r]; toast.0 = Some(("If you use a legendary hero, you won't be able to use any other heroes.".into(), 3.0)); }
                else if save.party.iter().any(|&p| data.hero(save.heroes[p].hero).super_hero) { toast.0 = Some(("A legendary hero fights alone".into(), 2.0)); sfx.play("sfx_wrong"); }
                else if save.party.len() < save.team_size as usize { save.party.push(r); } else { toast.0 = Some((format!("Party is full ({} heroes)", save.team_size), 2.0)); }
            }
            6 => { // buy the 4th party slot: first click asks, second confirms
                if *pend && toast.0.is_some() {
                    if save.money() < crate::save::UNLOCK_COST { toast.0 = Some(("Not enough gold".into(), 2.0)); sfx.play("sfx_wrong"); }
                    else { save.add_money(-crate::save::UNLOCK_COST); save.team_size = 4; sfx.play("sfx_buy"); toast.0 = Some(("A 4th hero can now join your party".into(), 2.0)); }
                    *pend = false;
                } else { *pend = true; toast.0 = Some((format!("Unlock a 4th party slot for {} gold? Click again to confirm", crate::save::UNLOCK_COST), 3.0)); sfx.play("sfx_select"); }
            }
            _ => {}
        }
        if c != 6 { *pend = false; }
        save.write();
    }
    if save.found.is_empty() && save.heroes.iter().all(|o| o.hero != 5) { save.found.push(5); } // SceneSelectHero::OnLoad: empty list -> addHeroToUnHire(5)
    { let l = hire_list(&save); if !l.is_empty() && !l.contains(&sel.hero) { sel.hero = l[0]; dirty = true; } }
    if dirty { *built = true; rebuild(&mut cmd, &a, &f, &data, &sheets, &save, &sel, root, &old); }
}

/// HeroAttribute superHero=1 heroes outside the camp drops: 35 Elite Fighter. The original sold him only in the
/// SceneSuperHero in-app pack (product 5121490); own rule: he is always on the Inn list at his HeroAttribute money (700).
/// (36 Shaman, the other superHero, stays a hero-camp drop: map 116.)
pub const LEGENDARY: [i32; 1] = [35];
fn hire_list(save: &Save) -> Vec<i32> {
    let mut l = save.found.clone();
    for h in LEGENDARY { if !l.contains(&h) && save.heroes.iter().all(|o| o.hero != h) { l.push(h); } }
    l
}

#[allow(clippy::too_many_arguments)]
fn scroll(save: Res<Save>, mut wheel: MessageReader<bevy::input::mouse::MouseWheel>, k: Res<ButtonInput<KeyCode>>, mut sel: ResMut<HireSel>, mut q: Query<&mut Transform, With<ListRoot>>,
          mouse: Res<ButtonInput<MouseButton>>, mut motion: MessageReader<bevy::input::mouse::MouseMotion>, cur: Res<Cursor>) {
    for w in wheel.read() { sel.scroll += (w.y + w.x) * 60.0; }
    if k.pressed(KeyCode::ArrowLeft) { sel.scroll += 12.0; }
    if k.pressed(KeyCode::ArrowRight) { sel.scroll -= 12.0; }
    let dragging = mouse.pressed(MouseButton::Left) && cur.world.map(|p| p.y.abs() < 170.0).unwrap_or(false);
    for m in motion.read() { if dragging { sel.scroll += m.delta.x; } }
    let min = -((hire_list(&save).len() as f32 - 4.0).max(0.0)) * SPACING;
    sel.scroll = sel.scroll.clamp(min, 0.0);
    if let Ok(mut t) = q.single_mut() { t.translation.x = sel.scroll; }
}
