//! SceneUserBag — Team/Equip: portrait strip, BACK, gold; parchment (equipments.png) with the 7 slots
//! (helm, armor, L hand, R hand, ring L, ring R, boots — positions measured from the texture) around the
//! animated hero; stash 5×3 with tabs I/II/III; description box; GET MORE. Drag stash↔slots to equip.
use crate::{anim::*, common::*, data::GameData, save::Save, stats, ui::*, AppState, Selected};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(AppState::Equip), enter)
        .add_systems(Update, (drag_system, logic).chain().run_if(in_state(AppState::Equip)))
        .add_systems(OnExit(AppState::Equip), despawn_screen);
}

// positions measured on the original equip screenshot (ref/equip_screenshot.jpg, 480x320 → x2 world)
const PARCH: Vec2 = Vec2::new(-237.0, 19.0);
const STASH_C: Vec2 = Vec2::new(241.0, 21.0);
/// warehouse cells: 5x3, first centre (-171, +55) from the panel centre, 86.7 apart across and 84 down
fn bag_pos(i: usize) -> Vec2 { STASH_C + Vec2::new(-171.0 + 86.7 * (i % 5) as f32, 55.0 - 84.0 * (i / 5) as f32) }
fn bag_tabs(cmd: &mut Commands, a: &AssetServer, f: &Fonts, cur: usize, unlocked: usize) {
    tabs_at(cmd, a, f, STASH_C + Vec2::new(-163.0, 143.0), 98.0, Vec2::new(86.0, 50.0), cur, 1, unlocked);
}
/// slot centres inside equipments.png (468×356, px from top-left)
const SLOT_PX: [(f32, f32); 7] = [(165.0, 66.0), (305.0, 66.0), (379.0, 157.0), (91.0, 157.0), (91.0, 241.0), (379.0, 241.0), (231.0, 315.0)];
const SHADOW: [&str; 7] = ["helm", "armor", "handR", "handL", "ring", "ring", "boots"];

fn slot_pos(k: usize) -> Vec2 { PARCH + Vec2::new(SLOT_PX[k].0 - 234.0, 178.0 - SLOT_PX[k].1) }

#[derive(Component)]
struct HeroFig;
#[derive(Component)]
struct StatsText;
#[derive(Component)]
struct AttrPage;
#[derive(Component)]
struct StatesBtn;
/// Base / Equipment / Perks / Total table drawn inside the description box whenever no item is being inspected
/// (own layout: the original only has the equipments_0 attribute page).
#[derive(Component)]
struct StatCell(usize, usize);
#[derive(Resource, Default)]
struct ShowStats(bool);

const TABLE_ROWS: usize = 7;
fn cell_pos(row: usize, col: usize) -> Vec2 {
    let half = row / TABLE_ROWS; let r = row % TABLE_ROWS;
    let x0 = -462.0 + half as f32 * 350.0;
    let x = if col == 0 { x0 } else { x0 + 120.0 + 54.0 * col as f32 };
    Vec2::new(x, -186.0 - 17.0 * (r + 1) as f32)
}

/// Rows of (stat, base, equipment, perks, total) from hero_fighter with/without equipment and passives.
fn breakdown(data: &GameData, oh: &crate::save::OwnedHero) -> Vec<[String; 5]> {
    let (b, bx) = stats::hero_fighter_with(data, oh, false, false);
    let (e, ex) = stats::hero_fighter_with(data, oh, true, false);
    let (f, fx) = stats::hero_fighter_with(data, oh, true, true);
    let d = |v: f32| if v.abs() < 0.05 { "-".to_string() } else if v.fract().abs() < 0.05 { format!("{:+.0}", v) } else { format!("{:+.1}", v) };
    let n = |v: f32| if v.fract().abs() < 0.05 { format!("{:.0}", v) } else { format!("{:.1}", v) };
    let row = |name: &str, bv: f32, ev: f32, fv: f32| [name.to_string(), n(bv), d(ev - bv), d(fv - ev), n(fv)];
    let dmg = |x: &crate::combat::Fighter| (x.weapon_min + x.attack, x.weapon_max + x.attack);
    let (b0, b1) = dmg(&b); let (e0, e1) = dmg(&e); let (f0, f1) = dmg(&f);
    let rng = |lo: i32, hi: i32| if lo == 0 && hi == 0 { "-".to_string() } else { format!("{:+}~{}", lo, hi) };
    vec![
        row("HP", b.max_hp, e.max_hp, f.max_hp),
        ["Damage".into(), format!("{b0}~{b1}"), rng(e0 - b0, e1 - b1), rng(f0 - e0, f1 - e1), format!("{f0}~{f1}")],
        row("Defense", b.ac as f32, e.ac as f32, f.ac as f32),
        row("Magic Defense", b.sr as f32, e.sr as f32, f.sr as f32),
        row("Hit Rate", b.hit_rate as f32, e.hit_rate as f32, f.hit_rate as f32),
        row("Critical %", b.crit_rate as f32, e.crit_rate as f32, f.crit_rate as f32),
        row("Crit Damage %", b.crit_damage * 100.0, e.crit_damage * 100.0, f.crit_damage * 100.0),
        row("Atk Time (s)", b.attack_interval, e.attack_interval, f.attack_interval),
        row("Move Speed", b.move_speed, e.move_speed, f.move_speed),
        row("Physical Resist", b.resist_physics as f32, e.resist_physics as f32, f.resist_physics as f32),
        row("Fire Resist", bx.resist_fire as f32, ex.resist_fire as f32, fx.resist_fire as f32),
        row("Cold Resist", bx.resist_cold as f32, ex.resist_cold as f32, fx.resist_cold as f32),
        row("Damage Resist", bx.resist_flat as f32, ex.resist_flat as f32, fx.resist_flat as f32),
        row("Avoid %", bx.avoid_pct as f32, ex.avoid_pct as f32, fx.avoid_pct as f32),
    ]
}

fn enter(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, data: Res<GameData>, sheets: Res<Sheets>, save: Res<Save>, mut tab: ResMut<Tab>, mut sel: ResMut<Selected>) {
    tab.0 = 0;
    if sel.0 >= save.heroes.len() { sel.0 = 0; }
    image(&mut cmd, &a, "ui/team_bg.png", Vec2::ZERO, Some(SCREEN), 0.0);
    let list: Vec<usize> = (0..save.heroes.len()).collect();
    portrait_strip(&mut cmd, &a, &data, &sheets, &save, sel.0, &list);
    back_and_gold(&mut cmd, &a, &f, &save, AppState::Tavern);
    image(&mut cmd, &a, "ui/equipments.png", PARCH, None, 2.0);
    for k in 0..7 {
        slot_tile(&mut cmd, &a, Slot::Equip(k), slot_pos(k), &format!("ui/itemshadow_{}.png", SHADOW[k]), 3.0);
    }
    image(&mut cmd, &a, "ui/warehouse.png", STASH_C, None, 2.0);
    for i in 0..15 { slot_tile(&mut cmd, &a, Slot::Stash(i), bag_pos(i), "ui/item.png", 3.0); }
    bag_tabs(&mut cmd, &a, &f, 0, save.bag_tabs as usize);
    desc_box(&mut cmd, &a, &f, Vec2::new(-110.0, -245.0));
    button(&mut cmd, &a, "ui/GETMORE.png", Some("ui/GETMORE_0.png"), Vec2::new(390.0, -275.0), Vec2::new(186.0, 76.0), 5.0, Act::Stub("GET MORE (in-app purchase)"));
    // button_states (silhouette) on the parchment's top-right flips it to equipments_0 with the hero's attributes
    // (UserHeroAttrGameObject); button_states_0 (arrow) flips back
    let b = button(&mut cmd, &a, "ui/button_states.png", None, Vec2::new(-60.0, 126.0), Vec2::new(102.0, 80.0), 4.8, Act::Custom(2, 0));
    cmd.entity(b).insert(StatesBtn);
    let page = image(&mut cmd, &a, "ui/equipments_0.png", PARCH, None, 4.6);
    cmd.entity(page).insert((AttrPage, Visibility::Hidden));
    let st = cmd.spawn((Text2d::new(""), TextFont { font: f.main.clone().into(), font_size: bevy::text::FontSize::Px(20.0), ..default() },
        TextColor(Color::srgb(0.3, 0.15, 0.05)), bevy::sprite::Anchor::TOP_LEFT,
        Transform::from_translation((PARCH + Vec2::new(-200.0, 140.0)).extend(4.7)), Visibility::Hidden, Scr, StatsText, AttrPage)).id();
    let _ = st;
    for half in 0..2 {
        for (c, h) in ["", "Base", "Equip", "Perks", "Total"].iter().enumerate() {
            let p = cell_pos(half * TABLE_ROWS, c) + Vec2::new(0.0, 17.0);
            let e = text(&mut cmd, &f, *h, p, 14.0, Color::srgb(1.0, 0.92, 0.7), 6.0);
            cmd.entity(e).insert((StatCell(usize::MAX, c), if c == 0 { bevy::sprite::Anchor::CENTER_LEFT } else { bevy::sprite::Anchor::CENTER_RIGHT }));
        }
    }
    for r in 0..TABLE_ROWS * 2 {
        for c in 0..5 {
            let e = text(&mut cmd, &f, "", cell_pos(r, c), 14.0, if c == 4 { Color::srgb(0.45, 0.1, 0.0) } else { Color::srgb(0.25, 0.15, 0.05) }, 6.0);
            cmd.entity(e).insert((StatCell(r, c), if c == 0 { bevy::sprite::Anchor::CENTER_LEFT } else { bevy::sprite::Anchor::CENTER_RIGHT }));
        }
    }
    cmd.insert_resource(ShowStats(true));
    cmd.insert_resource(crate::store::Rebuild(true));
}

/// UserHeroAttrGameObject lines (3.4.3 English format strings): NAME / LEVEL (MAX) / EXP / HP / Defense / MAINHAND, OFFHAND with Damage and Critical
fn stats_line(data: &GameData, save: &Save, r: usize) -> String {
    let oh = &save.heroes[r];
    let (ft, _ex) = stats::hero_fighter(data, oh);
    let need = stats::exp_to_next(data, oh);
    let mut s = format!("NAME: {}\nLEVEL: {}{}\nEXP: {} / {}\nHP: {:.0}\nDefense: {}\n", data.hero(oh.hero).class_name, oh.level,
        if oh.level >= 50 { " (MAX)" } else { "" }, oh.exp, need, ft.max_hp, ft.ac);
    for (lbl, k) in [("MAINHAND", 3usize), ("OFFHAND", 2usize)] {
        match oh.equip[k].and_then(|i| data.item(i)) {
            Some(it) => {
                s += &format!("{lbl}: {}\n", it.name.trim());
                if let (Some(lo), Some(hi)) = (it.min_attack, it.max_attack) { s += &format!("      Damage: {} ~ {}\n      Critical: {} %\n", lo + ft.attack, hi + ft.attack, ft.crit_rate); }
            }
            None => s += &format!("{lbl}: -\n"),
        }
    }
    s
}

#[allow(clippy::too_many_arguments)]
fn logic(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, data: Res<GameData>, sheets: Res<Sheets>, mut save: ResMut<Save>, mut tab: ResMut<Tab>, mut sel: ResMut<Selected>,
         mut ev: ResMut<SlotEvents>, mut clicked: ResMut<Clicked>, mut rb: ResMut<crate::store::Rebuild>,
         (slots, icons): (Query<(&ItemSlot, &Transform)>, Query<Entity, With<ItemIcon>>), (mut sfx, mut toast): (ResMut<crate::audio::Sfx>, ResMut<Toast>),
         (mut qt, mut qb, mut cells, mut show): (Query<&mut Text2d, (With<DescTitle>, Without<DescBody>)>, Query<&mut Text2d, (With<DescBody>, Without<DescTitle>)>, Query<(&StatCell, &mut Text2d, &mut Visibility), (Without<DescTitle>, Without<DescBody>, Without<StatsText>)>, ResMut<ShowStats>),
         (k, mut next): (Res<ButtonInput<KeyCode>>, ResMut<NextState<AppState>>), (fig, tabq, mut stq, pages, sbtn, mut page_on): (Query<Entity, With<HeroFig>>, Query<Entity, With<TabBtn>>, Query<&mut Text2d, (With<StatsText>, Without<DescTitle>, Without<DescBody>)>, Query<Entity, With<AttrPage>>, Query<Entity, With<StatesBtn>>, Local<bool>)) {
    if k.just_pressed(KeyCode::Escape) { next.set(AppState::Tavern); }
    let r = sel.0;
    for act in clicked.0.drain(..) {
        match act {
            Act::Custom(1, t) if (t as u8) < save.bag_tabs => { tab.0 = t as usize; for e in tabq.iter() { cmd.entity(e).despawn(); } bag_tabs(&mut cmd, &a, &f, tab.0, save.bag_tabs as usize); }
            Act::Custom(2, _) => {
                *page_on = !*page_on;
                let v = if *page_on { Visibility::Inherited } else { Visibility::Hidden };
                for e in pages.iter() { cmd.entity(e).insert(v); }
                for e in sbtn.iter() { cmd.entity(e).insert(Sprite::from_image(a.load(if *page_on { "ui/button_states_0.png" } else { "ui/button_states.png" }))); }
            }
            Act::Custom(PORTRAIT_ACT, h) => { sel.0 = h as usize; next.set(AppState::Equip); }
            _ => {}
        }
        rb.0 = true;
    }
    for (from, to) in std::mem::take(&mut ev.drops) {
        match (from, to) {
            (Slot::Stash(i), Slot::Equip(k)) => if let Some(id) = save.stash[i] {
                let it = data.item(id).unwrap();
                if !stats::slots_for(it).contains(&k) { toast.0 = Some(("That item doesn't go in this slot".into(), 2.0)); sfx.play("sfx_wrong"); continue; }
                if !stats::hero_can_use(data.hero(save.heroes[r].hero), it) { toast.0 = Some(("This hero can't use that item".into(), 2.0)); sfx.play("sfx_wrong"); continue; }
                let old = save.heroes[r].equip[k];
                save.heroes[r].equip[k] = Some(id);
                save.stash[i] = old;
                sfx.play(match k { 2 | 3 => "sfx_equipweapon", 4 | 5 => "sfx_equipring", _ => "sfx_equiparmor" });
            },
            (Slot::Equip(k), Slot::Stash(i)) => {
                let cur = save.stash[i];
                if let Some(c) = cur { // swap only if the stash item fits this slot
                    let it = data.item(c).unwrap();
                    if !stats::slots_for(it).contains(&k) || !stats::hero_can_use(data.hero(save.heroes[r].hero), it) { continue; }
                }
                save.stash[i] = save.heroes[r].equip[k];
                save.heroes[r].equip[k] = cur;
            }
            (Slot::Equip(a1), Slot::Equip(b1)) => {
                if let Some(id) = save.heroes[r].equip[a1] { if stats::slots_for(data.item(id).unwrap()).contains(&b1) { save.heroes[r].equip.swap(a1, b1); } }
            }
            (Slot::Stash(i), Slot::Stash(j)) => save.stash.swap(i, j),
            _ => {}
        }
        save.write();
        show.0 = true; set_desc("", "", &mut qt, &mut qb);
        rb.0 = true;
    }
    for c in std::mem::take(&mut ev.clicks) {
        // tapping an item shows what it adds; tapping it again (or an empty slot) returns to the hero's stat table
        let id = slot_item(&save, r, c);
        let again = qt.single().map(|t| !t.0.is_empty()).unwrap_or(false) && !show.0 && id.is_some_and(|i| qt.single().map(|t| t.0.starts_with(data.item(i).map(|e| e.name.trim()).unwrap_or("?"))).unwrap_or(false));
        show.0 = id.is_none() || again;
        let (t, b) = if show.0 { Default::default() } else { id.map(|i| item_desc(&data, i)).unwrap_or_default() };
        set_desc(t, b, &mut qt, &mut qb);
        rb.0 = true;
    }
    if rb.0 {
        rb.0 = false;
        refresh_icons(&mut cmd, &a, &data, &save, r, tab.0, &slots, &icons);
        for e in fig.iter() { cmd.entity(e).despawn(); }
        let e = crate::anim::hero_figure(&mut cmd, &data, &sheets, &save.heroes[r], PARCH + Vec2::new(0.0, -81.0), 125.0, 3.5);
        cmd.entity(e).insert(HeroFig);
        if let Ok(mut s) = stq.single_mut() { s.0 = stats_line(&data, &save, r); }
        let rows = breakdown(&data, &save.heroes[r]);
        for (c, mut t, mut v) in &mut cells {
            *v = if show.0 { Visibility::Inherited } else { Visibility::Hidden };
            if c.0 != usize::MAX { let s = rows.get(c.0).map(|x| x[c.1].clone()).unwrap_or_default(); if t.0 != s { t.0 = s; } }
        }
    }
}
