//! Pieces shared by several screens: the hero portrait strip (char_bg), BACK, gold counter, description box.
use crate::{anim::*, data::GameData, save::Save, ui::*, AppState};
use bevy::prelude::*;

pub const PORTRAIT_ACT: u32 = 900;

/// Top-left 4-slot portrait strip (char_bg, 528×120) as on the Equip/Store/Skills screens.
/// Clicking a portrait emits Custom(PORTRAIT_ACT, roster index).
pub fn portrait_strip(cmd: &mut Commands, a: &AssetServer, data: &GameData, sheets: &Sheets, save: &Save, selected: usize, list: &[usize]) {
    let c = Vec2::new(-480.0 + 264.0 + 12.0, 320.0 - 62.0);
    image(cmd, a, "ui/char_bg.png", c, None, 5.0);
    for (i, &h) in list.iter().take(4).enumerate() {
        let p = c + Vec2::new(-198.0 + 132.0 * i as f32, -44.0);
        crate::anim::hero_figure(cmd, data, sheets, &save.heroes[h], p, 84.0, 6.0);
        hotspot(cmd, p + Vec2::Y * 44.0, Vec2::new(120.0, 100.0), 7.0, Act::Custom(PORTRAIT_ACT, h as i32));
        if h == selected { image(cmd, a, "ui/item_selected.png", p + Vec2::Y * 44.0, Some(Vec2::new(116.0, 100.0)), 6.5); }
    }
}

pub fn back_and_gold(cmd: &mut Commands, a: &AssetServer, f: &Fonts, save: &Save, back_to: AppState) -> Entity {
    button(cmd, a, "ui/button_BACK.png", Some("ui/button_BACK_0.png"), Vec2::new(372.0, 278.0), Vec2::new(176.0, 60.0), 10.0, Act::Goto(back_to));
    image(cmd, a, "ui/coin.png", Vec2::new(330.0, 222.0), None, 10.0);
    let t = text(cmd, f, format!("{}", save.money()), Vec2::new(392.0, 222.0), 26.0, Color::WHITE, 10.0);
    cmd.entity(t).insert(GoldText);
    t
}

#[derive(Component)]
pub struct GoldText;
pub fn update_gold(save: Res<Save>, mut q: Query<&mut Text2d, With<GoldText>>) {
    for mut t in q.iter_mut() { let s = format!("{}", save.money()); if t.0 != s { t.0 = s; } }
}

#[derive(Component)]
pub struct DescTitle;
#[derive(Component)]
pub struct DescBody;

/// Bottom description box (description.png 734×158) with a title and wrapped body text.
pub fn desc_box(cmd: &mut Commands, a: &AssetServer, f: &Fonts, center: Vec2) {
    image(cmd, a, "ui/description.png", center, None, 5.0);
    let t = text(cmd, f, "", center + Vec2::new(-340.0, 50.0), 24.0, Color::srgb(1.0, 0.92, 0.7), 6.0);
    cmd.entity(t).insert((DescTitle, bevy::sprite::Anchor::TOP_LEFT, FitBox { max_w: 680.0, max_h: 30.0, base: 24.0, min: 14.0 }));
    let b = cmd.spawn((Text2d::new(""), TextFont { font: f.main.clone().into(), font_size: bevy::text::FontSize::Px(18.0), ..default() },
        TextColor(Color::srgb(0.25, 0.15, 0.05)), bevy::text::TextBounds::new_horizontal(680.0), bevy::sprite::Anchor::TOP_LEFT,
        Transform::from_translation((center + Vec2::new(-340.0, 16.0)).extend(6.0)), Scr, DescBody, FitBox { max_w: 682.0, max_h: 86.0, base: 18.0, min: 9.0 })).id();
    let _ = b;
}

pub fn set_desc(title: impl Into<String>, body: impl Into<String>, qt: &mut Query<&mut Text2d, (With<DescTitle>, Without<DescBody>)>, qb: &mut Query<&mut Text2d, (With<DescBody>, Without<DescTitle>)>) {
    if let Ok(mut t) = qt.single_mut() { t.0 = title.into(); }
    if let Ok(mut b) = qb.single_mut() { b.0 = body.into(); }
}

/// Item tile: frame + icon (icons/item_<tex_index>.png).
pub fn item_icon_path(data: &GameData, id: i32) -> Option<String> {
    data.item(id).map(|e| format!("icons/item_{}.png", e.tex_index))
}

/// One line per stat the item adds (names follow the original's UserHeroAttr wording: Damage, Defense, HP, Critical).
pub fn item_stat_lines(data: &GameData, id: i32) -> Vec<String> {
    use crate::types::EquipEffect as E;
    let Some(e) = data.item(id) else { return vec![] };
    let mut v = vec![];
    if let (Some(lo), Some(hi)) = (e.min_attack, e.max_attack) { if hi > 0 { v.push(format!("Damage {lo} ~ {hi}")); } }
    if let Some(ac) = e.ac.filter(|x| *x != 0) { v.push(format!("Defense +{ac}")); }
    if let Some(sr) = e.sr.filter(|x| *x != 0) { v.push(format!("Magic Defense +{sr}")); }
    for a in &e.attrs {
        let d = a.data; let sg = if d >= 0 { "+" } else { "" };
        let t = match a.effect {
            E::Addac => format!("Defense {sg}{d}"), E::AddacEnemy => format!("Enemy Defense {sg}{d}"),
            E::Addcritdamage => format!("Critical Damage {sg}{d}%"), E::Addcritrate => format!("Critical {sg}{d}%"),
            E::Adddamage => format!("Damage {sg}{d}"), E::AdddamagePercent => format!("Damage {sg}{d}%"),
            E::AdddamageUndead => format!("Damage vs Undead {sg}{d}"), E::AdddamageUndeadPercent => format!("Damage vs Undead {sg}{d}%"),
            E::Addhitrate => format!("Hit Rate {sg}{d}"), E::Addmaxhp => format!("HP {sg}{d}"),
            E::Attackspeed => format!("Attack Speed {:+}%", -d), E::Speed => format!("Move Speed {sg}{d}%"), // intro texts: data -15 = "+15% attack speed"
            E::Avoiddamage => "Avoid Damage".into(), E::Avoidrangeddamage => "Avoid Ranged Damage".into(),
            E::Bloodsuck => format!("Life Steal {d}%"), E::HPrecover => format!("HP Regen {sg}{d}"),
            E::DamageCold => format!("Cold Damage {sg}{d}"), E::DamageFire => format!("Fire Damage {sg}{d}"),
            E::DamagePhysics => format!("Physical Damage {sg}{d}"), E::DamagePoison => format!("Poison Damage {sg}{d}"),
            E::DamageFireSwordPower => format!("Fire Blade {sg}{d}"), E::DamageIceSwordPower => format!("Ice Blade {sg}{d}"),
            E::DeviltongueStateDizzy | E::DizzyHeld => "Stun".into(), E::Stumble => "Knockdown".into(), E::Kill => "Instant Kill".into(),
            E::Firesheild => format!("Fire Shield {d}"),
            E::ImmuneHeld => "Immune to Hold".into(), E::ImmuneKill => "Immune to Instant Kill".into(), E::ImmuneStumble => "Immune to Knockdown".into(),
            E::ResistCold => format!("Cold Resist {sg}{d}"), E::ResistFire => format!("Fire Resist {sg}{d}"),
            E::ResistPhysics => format!("Physical Resist {sg}{d}"), E::ResistPoison => format!("Poison Resist {sg}{d}"),
            E::ResistDamage => format!("Damage Resist {sg}{d}"), E::Toughnessvalue => format!("Toughness {sg}{d}"),
            #[allow(unreachable_patterns)] o => format!("{o:?} {d}"),
        };
        let mut t = t;
        if a.chance_pct > 0 && a.chance_pct < 100 { t += &format!(" ({}% chance)", a.chance_pct); }
        if let Some(ms) = a.time_ms.filter(|m| *m > 0) { t += &format!(" for {:.1}s", ms as f32 / 1000.0); }
        v.push(t);
    }
    v
}

pub fn item_desc(data: &GameData, id: i32) -> (String, String) {
    let Some(e) = data.item(id) else { return (String::new(), String::new()) };
    let intro = e.intro.replace("\\n", " ").replace("\n", " ").replace("¡¯", "'").split_whitespace().collect::<Vec<_>>().join(" ");
    // the <+8 damage> tags repeat the stat line above, so they are dropped from the flavour text
    let mut intro2 = String::new(); let mut depth = 0;
    for ch in intro.chars() { match ch { '<' => depth += 1, '>' if depth > 0 => depth -= 1, _ if depth == 0 => intro2.push(ch), _ => {} } }
    let intro = intro2.split_whitespace().collect::<Vec<_>>().join(" ");
    let stats = item_stat_lines(data, id).join("   ·   ");
    (format!("{}   (Price: {})", e.name.trim(), e.price),
     if stats.is_empty() { intro } else { format!("{stats}\n{intro}") })
}

/// Text that must stay inside a box: the font shrinks a pixel per frame while the *measured* layout (TextLayoutInfo) is
/// larger than the box, and resets to `base` whenever the text changes, so nothing is ever cut off.
#[derive(Component, Clone, Copy)]
pub struct FitBox { pub max_w: f32, pub max_h: f32, pub base: f32, pub min: f32 }
pub fn fit_desc(mut q: Query<(Ref<Text2d>, &mut TextFont, &bevy::text::TextLayoutInfo, &FitBox)>) {
    for (t, mut f, info, fb) in &mut q {
        let cur = if let bevy::text::FontSize::Px(v) = f.font_size { v } else { fb.base };
        if t.is_changed() { if cur != fb.base { f.font_size = bevy::text::FontSize::Px(fb.base); } continue; }
        let sz = info.size / info.scale_factor.max(0.01);
        if sz == Vec2::ZERO { continue; }
        if (sz.y > fb.max_h + 0.5 || sz.x > fb.max_w + 0.5) && cur > fb.min { f.font_size = bevy::text::FontSize::Px(cur - 1.0); }
    }
}

// ---------- item grids with drag & drop (Store / Equip) ----------
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot { Stash(usize), Shop(usize), Equip(usize) }

#[derive(Component)]
pub struct ItemSlot(pub Slot);
#[derive(Component)]
pub struct ItemIcon(#[allow(dead_code)] pub Slot);
#[derive(Component)]
pub struct DragGhost;

#[derive(Resource, Default)]
pub struct DragState { pub from: Option<Slot>, pub start: Vec2, pub moved: bool }
/// (from, to) drops and plain clicks produced this frame; screens consume them.
#[derive(Resource, Default)]
pub struct SlotEvents { pub drops: Vec<(Slot, Slot)>, pub clicks: Vec<Slot> }

pub const CELL: f32 = 86.0;
/// 5×3 grid inside a warehouse/store panel centred at `c`.
pub fn grid_pos(c: Vec2, i: usize) -> Vec2 { c + Vec2::new(-172.0 + CELL * (i % 5) as f32, 44.0 - 90.0 * (i / 5) as f32) }

pub fn slot_tile(cmd: &mut Commands, a: &AssetServer, slot: Slot, p: Vec2, frame: &str, z: f32) {
    let mut s = Sprite::from_image(a.load(frame.to_string()));
    s.custom_size = Some(Vec2::splat(80.0));
    cmd.spawn((s, Transform::from_translation(p.extend(z)), Scr, ItemSlot(slot)));
}

pub fn slot_item(save: &Save, hero: usize, s: Slot) -> Option<i32> {
    match s {
        Slot::Stash(i) => save.stash.get(i).copied().flatten(),
        Slot::Shop(i) => save.shop.get(i).copied().flatten(),
        Slot::Equip(k) => save.heroes.get(hero).and_then(|h| h.equip[k]),
    }
}

/// Re-create icons on top of every visible ItemSlot (cheap: ≤60 sprites).
pub fn refresh_icons(cmd: &mut Commands, a: &AssetServer, data: &GameData, save: &Save, hero: usize, tab: usize,
                     slots: &Query<(&ItemSlot, &Transform)>, icons: &Query<Entity, With<ItemIcon>>) {
    for e in icons.iter() { cmd.entity(e).despawn(); }
    for (s, t) in slots.iter() {
        let real = match s.0 { Slot::Stash(i) => Slot::Stash(i + tab * 15), o => o };
        if let Some(id) = slot_item(save, hero, real) {
            if let Some(p) = item_icon_path(data, id) {
                let mut sp = Sprite::from_image(a.load(p));
                sp.custom_size = Some(Vec2::splat(66.0));
                cmd.spawn((sp, Transform::from_translation(t.translation + Vec3::Z), Scr, ItemIcon(real)));
            }
        }
    }
}

pub fn drag_system(mut cmd: Commands, a: Res<AssetServer>, data: Res<GameData>, save: Res<Save>, sel: Res<crate::Selected>, tab: Res<Tab>,
                   mouse: Res<ButtonInput<MouseButton>>, cur: Res<Cursor>, slots: Query<(&ItemSlot, &GlobalTransform)>,
                   mut ds: ResMut<DragState>, mut ev: ResMut<SlotEvents>, mut ghost: Query<(Entity, &mut Transform), With<DragGhost>>) {
    let Some(p) = cur.world else { return };
    let under = || slots.iter().find(|(_, t)| (p - t.translation().truncate()).abs().cmple(Vec2::splat(40.0)).all())
        .map(|(s, _)| match s.0 { Slot::Stash(i) => Slot::Stash(i + tab.0 * 15), o => o });
    if mouse.just_pressed(MouseButton::Left) {
        ds.from = under(); ds.start = p; ds.moved = false;
    }
    if let Some(from) = ds.from {
        if mouse.pressed(MouseButton::Left) && !ds.moved && p.distance(ds.start) > 8.0 {
            if let Some(id) = slot_item(&save, sel.0, from) {
                ds.moved = true;
                if let Some(path) = item_icon_path(&data, id) {
                    let mut sp = Sprite::from_image(a.load(path)); sp.custom_size = Some(Vec2::splat(70.0));
                    cmd.spawn((sp, Transform::from_translation(p.extend(100.0)), Scr, DragGhost));
                }
            }
        }
        for (_, mut t) in ghost.iter_mut() { t.translation = p.extend(100.0); }
        if mouse.just_released(MouseButton::Left) {
            for (e, _) in ghost.iter() { cmd.entity(e).despawn(); }
            if ds.moved { if let Some(to) = under() { if to != from { ev.drops.push((from, to)); } } }
            else { ev.clicks.push(from); }
            ds.from = None;
        }
    }
}

/// Current stash tab (I/II/III).
#[derive(Resource, Default)]
pub struct Tab(pub usize);

pub fn tabs(cmd: &mut Commands, a: &AssetServer, f: &Fonts, panel: Vec2, cur: usize, code: u32, unlocked: usize) {
    tabs_at(cmd, a, f, panel + Vec2::new(-150.0, 140.0), 80.0, Vec2::new(70.0, 40.0), cur, code, unlocked)
}
#[allow(clippy::too_many_arguments)]
pub fn tabs_at(cmd: &mut Commands, a: &AssetServer, f: &Fonts, first: Vec2, step: f32, size: Vec2, cur: usize, code: u32, unlocked: usize) {
    for i in 0..3 {
        let p = first + Vec2::new(step * i as f32, 0.0);
        // English build: wooden tabs with red roman numerals (button_warehouse is the STASH bag there, not a tab)
        let col = if i == cur { Color::srgb(0.80, 0.58, 0.30) } else { Color::srgb(0.55, 0.38, 0.20) };
        let b = cmd.spawn((Sprite::from_color(col, size), Transform::from_translation(p.extend(8.0)), Scr,
            Hot { size, act: Act::Custom(code, i as i32), normal: None, pressed: None })).id();
        let t = text(cmd, f, ["I", "II", "III"][i], p, 24.0, Color::srgb(0.65, 0.08, 0.05), 9.0);
        if i >= unlocked { // original UserBagGameObject: map2_locked over a locked bag tab
            let mut s = Sprite::from_image(a.load("ui/map2_locked.png")); s.custom_size = Some(Vec2::new(26.0, 29.0));
            let l = cmd.spawn((s, Transform::from_translation((p + Vec2::new(22.0, 0.0)).extend(9.5)), Scr)).id();
            cmd.entity(l).insert(TabBtn);
        }
        cmd.entity(b).insert(TabBtn); cmd.entity(t).insert(TabBtn);
    }
}

#[derive(Component)]
pub struct TabBtn;
