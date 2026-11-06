//! Pre-rendered unit sheets (decoded from the original .acb skeletal animations + paper-doll parts by
//! /workspace/hvsm/anim/batch.py). Rows: 0 standby, 1 move, 2 attack, 3 hurt, 4 dead, 5 skill. 24 fps,
//! every frame of the original .acb animations.
use crate::{data::GameData, ui::Scr};
use bevy::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;

pub const FPS: f32 = 24.0; // AnimationType.xml rate="24"; sheets now keep every frame (the original plays baked frames, no runtime tweening)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Row { Standby = 0, Move = 1, Attack = 2, Hurt = 3, Dead = 4, Skill = 5 }

#[derive(Resource, Default)]
pub struct Sheets { pub layouts: HashMap<String, Handle<TextureAtlasLayout>>, pub images: HashMap<String, Handle<Image>> }

pub fn setup_sheets(data: Res<GameData>, a: Res<AssetServer>, mut layouts: ResMut<Assets<TextureAtlasLayout>>, mut sheets: ResMut<Sheets>) {
    for (k, s) in &data.sheets {
        let l = TextureAtlasLayout::from_grid(UVec2::new(s.cell.0, s.cell.1), s.cols, 6, None, None);
        sheets.layouts.insert(k.clone(), layouts.add(l));
        if crate::data::asset_exists(format!("sprites/{k}.png")) { sheets.images.insert(k.clone(), a.load(format!("sprites/{k}.png"))); }
    }
}

/// Animated sheet on a child entity; the parent's origin is the unit's feet.
#[derive(Component)]
pub struct Anim { pub cols: u32, pub frames: Vec<u32>, pub row: Row, pub frame: u32, pub t: f32, pub once: bool, pub done: bool, pub offset: Vec2, pub flip: bool }
impl Anim {
    pub fn play(&mut self, r: Row, once: bool) {
        if self.row == r && !self.done && !once { return; }
        self.row = r; self.frame = 0; self.t = 0.0; self.once = once; self.done = false;
    }
}

/// Spawns `key`'s sheet as a child of `parent`, feet at the parent's origin. Returns the sprite entity.
pub fn attach_sheet(cmd: &mut Commands, data: &GameData, sheets: &Sheets, parent: Entity, key: &str, scale: f32, z: f32) -> Option<Entity> {
    if let Some(h) = key.strip_prefix("hero_").and_then(|h| h.parse::<i32>().ok()) {
        if rigs().keys.contains_key(key) { return attach_rig(cmd, parent, key, hero_look(data, h, None), scale, z); }
    }
    if let Some(e) = key.strip_prefix("enemy_") {
        if rigs().keys.contains_key(key) { return attach_rig(cmd, parent, key, ware_look(data, rigs().base.get(&format!("e{e}")).cloned().unwrap_or_default()), scale, z); }
    }
    let info = data.sheets.get(key)?;
    let (w, h) = (info.cell.0 as f32, info.cell.1 as f32);
    // anchor = feet position inside the cell (px from top-left)
    let offset = Vec2::new(w / 2.0 - info.anchor.0, info.anchor.1 - h / 2.0) * scale;
    let mut s = Sprite::from_atlas_image(sheets.images.get(key)?.clone(), TextureAtlas { layout: sheets.layouts[key].clone(), index: 0 });
    s.custom_size = Some(Vec2::new(w, h) * scale);
    let e = cmd.spawn((s, Transform::from_translation(offset.extend(z)),
        Anim { cols: info.cols, frames: info.frames.clone(), row: Row::Standby, frame: 0, t: fastrand::f32(), once: false, done: false, offset, flip: false })).id();
    cmd.entity(parent).add_child(e);
    Some(e)
}

/// Scale that makes `key`'s standing figure `target_h` px tall (feet to top), so every hero fills a slot the same.
pub fn fit(data: &GameData, key: &str, target_h: f32) -> f32 {
    let h = data.sheets.get(key).map(|s| if s.body_h > 0.0 { s.body_h } else { s.anchor.1 }).unwrap_or(80.0);
    target_h / h.max(10.0)
}

/// Convenience: a standalone animated figure (Inn NPCs, hire list, equip screen).
pub fn figure(cmd: &mut Commands, data: &GameData, sheets: &Sheets, key: &str, pos: Vec2, scale: f32, z: f32) -> Entity {
    let p = cmd.spawn((Transform::from_translation(pos.extend(z)), Visibility::default(), Scr)).id();
    attach_sheet(cmd, data, sheets, p, key, scale, 0.0);
    p
}

pub fn animate(time: Res<Time>, mut q: Query<(&mut Anim, Option<&mut Sprite>, &mut Transform)>) {
    for (mut a, s, mut t) in q.iter_mut() {
        let n = a.frames.get(a.row as usize).copied().unwrap_or(1).max(1);
        a.t += time.delta_secs() * FPS;
        while a.t >= 1.0 {
            a.t -= 1.0;
            if a.frame + 1 >= n {
                if a.once { a.done = true; if a.row != Row::Dead { a.row = Row::Standby; a.frame = 0; a.once = false; } }
                else { a.frame = 0; }
            } else { a.frame += 1; }
        }
        let Some(mut s) = s else { t.scale.x = t.scale.x.abs() * if a.flip { -1.0 } else { 1.0 }; continue };
        if let Some(at) = &mut s.texture_atlas { at.index = (a.row as u32 * a.cols + a.frame.min(n - 1)) as usize; }
        s.flip_x = a.flip;
        t.translation.x = if a.flip { -a.offset.x } else { a.offset.x };
    }
}

/// Unit sheet key for a hero class (heroes.ron index); classes without a sheet reuse the same base class.
pub fn hero_key(data: &GameData, hero: i32) -> String {
    let k = format!("hero_{hero}");
    if data.sheets.contains_key(&k) { return k; }
    let base = match data.hero(hero).profession {
        crate::types::Profession::Archer => 5, crate::types::Profession::Healer => 4,
        crate::types::Profession::FireStaff => 3, crate::types::Profession::IceStaff => 6,
        crate::types::Profession::MagicStaff | crate::types::Profession::LightStaff => 29,
        crate::types::Profession::Soldier => 0,
    };
    format!("hero_{base}")
}
pub fn enemy_key(data: &GameData, e: i32) -> String {
    let k = format!("enemy_{e}");
    if data.sheets.contains_key(&k) { k } else { "enemy_0".into() }
}

// ---------------------------------------------------------------------------------------------
// Paper-doll rigs (heroes). The original (GOHero::initHero -> AnimationConfig::SetAvator) builds each
// hero from the .acb skeleton: every component (body, armL/armR, footL/footR, head, eye, helm,
// weaponL, weaponR, shadow) is a separate part image placed per frame; HEROSAVE's ware list picks the
// image: type 0 = ALLEQUIP texture index, type 1 = equipment index -> its texIndex. "foot" sets
// footL+footR, "body" also sets the arms ("arm" + the body's "_NNN" suffix), rings are not drawn.
// rigs.json is exported from the original files by anim/rig_export.py; parts/ holds the part images.

#[derive(serde::Deserialize)]
pub struct RigAnim { pub comps: Vec<(String, String)>, pub n: u32, pub f: Vec<Vec<Option<[f32; 7]>>> }
#[derive(serde::Deserialize)]
pub struct RigKey { pub rows: Vec<String>, pub hit: u32 }
#[derive(serde::Deserialize)]
pub struct Rigs { pub all: HashMap<String, String>, pub base: HashMap<String, Vec<(String, i32, i32)>>, pub keys: HashMap<String, RigKey>, pub anims: HashMap<String, RigAnim> }
static RIGS: std::sync::OnceLock<Rigs> = std::sync::OnceLock::new();
pub fn rigs() -> &'static Rigs {
    RIGS.get_or_init(|| {
        serde_json::from_str(&crate::data::read_text("rig/rigs.json").unwrap_or_default())
            .unwrap_or(Rigs { all: HashMap::new(), base: HashMap::new(), keys: HashMap::new(), anims: HashMap::new() })
    })
}

/// component -> part image name (None = hidden)
pub type Look = HashMap<String, Option<String>>;

/// Save equip slots (helm, armor, L hand, R hand, ring, ring, boots) -> ware component.
pub const SLOT_COMP: [(usize, &str); 5] = [(3, "weaponL"), (2, "weaponR"), (1, "body"), (0, "helm"), (6, "foot")];

pub fn hero_look(data: &GameData, hero: i32, equip: Option<&[Option<i32>; crate::save::SLOTS]>) -> Look {
    let r = rigs();
    let mut ware: Vec<(String, i32, i32)> = r.base.get(&hero.to_string()).cloned().unwrap_or_default();
    if let Some(eq) = equip {
        for (slot, comp) in SLOT_COMP {
            // HEROSAVE ware: an equipped item replaces the component; an empty hand holds nothing,
            // empty armour/helm/boots fall back to the hero's own base look
            let v = match eq[slot] { Some(i) => Some((i, 1)), None if slot == 2 || slot == 3 => Some((-1, 0)), None => None };
            if let Some((i, t)) = v {
                if let Some(w) = ware.iter_mut().find(|w| w.0 == comp) { w.1 = i; w.2 = t; } else { ware.push((comp.to_string(), i, t)); }
            }
        }
    }
    ware_look(data, ware)
}

/// Resolve a ware list (component, index, type) like GOHero/GOEnemy::initHero/initEnemy -> SetAvator.
pub fn ware_look(data: &GameData, ware: Vec<(String, i32, i32)>) -> Look {
    let r = rigs();
    let mut look = Look::new();
    for (comp, idx, ty) in ware {
        if comp.starts_with("ring") { continue; }
        let tex = if idx < 0 { None } else if ty == 1 { data.item(idx).and_then(|it| r.all.get(&it.tex_index.to_string()).cloned()) } else { r.all.get(&idx.to_string()).cloned() };
        match comp.as_str() {
            "foot" => { look.insert("footL".into(), tex.clone()); look.insert("footR".into(), tex); }
            "body" => {
                let arm = tex.as_ref().and_then(|t| t.rfind('_').map(|k| format!("arm{}", &t[k..])));
                look.insert("armL".into(), arm.as_ref().map(|a| format!("{a}_L")));
                look.insert("armR".into(), arm.map(|a| format!("{a}_R")));
                look.insert("body".into(), tex);
            }
            _ => { look.insert(comp, tex); }
        }
    }
    look
}

#[derive(Component)]
pub struct RigRoot { pub key: String, pub look: Look, pub built: bool }
#[derive(Component)]
pub struct RigPart { pub name: String, pub cur: String }

pub fn attach_rig(cmd: &mut Commands, parent: Entity, key: &str, look: Look, scale: f32, z: f32) -> Option<Entity> {
    let k = rigs().keys.get(key)?;
    let frames: Vec<u32> = k.rows.iter().map(|n| rigs().anims.get(n).map(|a| a.n).unwrap_or(1)).collect();
    let e = cmd.spawn((Transform::from_translation(Vec3::new(0.0, 0.0, z)).with_scale(Vec3::new(scale, scale, 1.0)), Visibility::default(),
        Anim { cols: 0, frames, row: Row::Standby, frame: 0, t: fastrand::f32(), once: false, done: false, offset: Vec2::ZERO, flip: false },
        RigRoot { key: key.to_string(), look, built: false })).id();
    cmd.entity(parent).add_child(e);
    Some(e)
}

/// A standalone hero figure wearing `oh`'s gear (equip screen, portraits, party strip).
pub fn hero_figure(cmd: &mut Commands, data: &GameData, sheets: &Sheets, oh: &crate::save::OwnedHero, pos: Vec2, target_h: f32, z: f32) -> Entity {
    let key = hero_key(data, oh.hero);
    let p = cmd.spawn((Transform::from_translation(pos.extend(z)), Visibility::default(), Scr)).id();
    if attach_rig(cmd, p, &key, hero_look(data, oh.hero, Some(&oh.equip)), fit(data, &key, target_h), 0.0).is_none() {
        attach_sheet(cmd, data, sheets, p, &key, fit(data, &key, target_h), 0.0);
    }
    p
}

pub fn build_rigs(mut cmd: Commands, mut q: Query<(Entity, &mut RigRoot)>) {
    for (e, mut r) in q.iter_mut() {
        if r.built { continue; }
        r.built = true;
        let Some(k) = rigs().keys.get(&r.key) else { continue };
        let mut names: Vec<String> = vec![];
        for n in &k.rows { if let Some(a) = rigs().anims.get(n) { for (c, _) in &a.comps { if !names.contains(c) { names.push(c.clone()); } } } }
        for n in names {
            let c = cmd.spawn((Sprite::default(), Transform::default(), Visibility::Hidden, RigPart { name: n, cur: String::new() })).id();
            cmd.entity(e).add_child(c);
        }
    }
}

/// Places every part for the current frame: the .acb keyframe gives position, rotation, scale, alpha
/// and draw order ("last"); part images are Imgs_2x, the rig works in half-size units like the old sheets.
pub fn pose_rigs(a: Res<AssetServer>, roots: Query<(&Anim, &RigRoot, &Children)>, mut parts: Query<(&mut RigPart, &mut Sprite, &mut Transform, &mut Visibility)>) {
    for (an, rr, ch) in roots.iter() {
        let Some(k) = rigs().keys.get(&rr.key) else { continue };
        let Some(anim) = k.rows.get(an.row as usize).and_then(|n| rigs().anims.get(n)) else { continue };
        let fi = an.frame.min(anim.n.saturating_sub(1)) as usize;
        for c in ch.iter() {
            let Ok((mut p, mut s, mut t, mut v)) = parts.get_mut(c) else { continue };
            let Some(ci) = anim.comps.iter().position(|(n, _)| *n == p.name) else { *v = Visibility::Hidden; continue };
            let tex = match rr.look.get(&p.name) { Some(Some(t)) => t.clone(), Some(None) => String::new(), None => anim.comps[ci].1.clone() };
            let Some(Some(f)) = anim.f[ci].get(fi) else { *v = Visibility::Hidden; continue };
            if tex.is_empty() { *v = Visibility::Hidden; continue; }
            if p.cur != tex { s.image = a.load(format!("parts/{tex}.png")); p.cur = tex; }
            s.color = Color::srgba(1.0, 1.0, 1.0, f[6] / 255.0);
            *v = Visibility::Inherited;
            t.translation = Vec3::new(f[0], f[1], f[5] * 0.001);
            t.rotation = Quat::from_rotation_z(f[4]);
            t.scale = Vec3::new(f[2] * 0.5, f[3] * 0.5, 1.0);
        }
    }
}

/// Skill / hit VFX sheets from EffectList.xml ACB → assets/fx/*.png (see anim/export via proto assets/fx).
#[derive(Debug, Clone, Deserialize)]
pub struct FxInfo { pub cell: (u32, u32), pub frames: u32, pub anchor: (f32, f32) }

#[derive(Resource, Default)]
pub struct FxSheets {
    pub info: HashMap<String, FxInfo>,
    pub layouts: HashMap<String, Handle<TextureAtlasLayout>>,
    pub images: HashMap<String, Handle<Image>>,
}

pub fn setup_fx(a: Res<AssetServer>, mut layouts: ResMut<Assets<TextureAtlasLayout>>, mut fx: ResMut<FxSheets>) {
    let Ok(s) = crate::data::read_text("fx/manifest.json") else { eprintln!("fx/manifest.json missing"); return };
    let Ok(man): Result<HashMap<String, FxInfo>, _> = serde_json::from_str(&s) else { eprintln!("fx manifest parse fail"); return };
    for (k, info) in man {
        let path = format!("fx/{k}.png");
        if !crate::data::asset_exists(&path) { continue; }
        let l = TextureAtlasLayout::from_grid(UVec2::new(info.cell.0, info.cell.1), info.frames, 1, None, None);
        fx.layouts.insert(k.clone(), layouts.add(l));
        fx.images.insert(k.clone(), a.load(path));
        fx.info.insert(k, info);
    }
    eprintln!("fx sheets loaded: {}", fx.info.len());
}

/// One-shot or looping effect sprite (EffectList anim).
#[derive(Component)]
pub struct FxAnim {
    pub frames: u32,
    pub frame: u32,
    pub t: f32,
    pub once: bool,
    pub done: bool,
    pub follow: Option<Entity>,
    pub life: f32, // >0 = despawn after; 0 = when once done
    pub z: f32,
}

/// Display scale for FX strips. Sheets are half-res ACB (same pipeline as unit sprites).
/// Unit sprites use SCALE=1.6; FX stay at 1.0 so on-screen size matches original gameplay
/// footage (PAQEh3KoHm4 / mSTSrkz-Je8): skill bursts ≈2.5× unit height. CreateEffect in the
/// .so applies no extra SetScale — ACB sx/sy only — so half-res@1.0 ≈ video after unit SCALE.
pub const FX_SCALE: f32 = 1.0;

pub fn spawn_fx(cmd: &mut Commands, fx: &FxSheets, name: &str, pos: Vec2, z: f32, once: bool, life: f32, follow: Option<Entity>) -> Option<Entity> {
    let info = fx.info.get(name)?;
    let img = fx.images.get(name)?.clone();
    let layout = fx.layouts.get(name)?.clone();
    let (w, h) = (info.cell.0 as f32 * FX_SCALE, info.cell.1 as f32 * FX_SCALE);
    let mut s = Sprite::from_atlas_image(img, TextureAtlas { layout, index: 0 });
    s.custom_size = Some(Vec2::new(w, h));
    // anchor is feet-ish inside cell; center the sprite roughly
    let e = cmd.spawn((
        s,
        Transform::from_translation(pos.extend(z)),
        FxAnim { frames: info.frames.max(1), frame: 0, t: 0.0, once, done: false, follow, life, z },
        crate::ui::Scr,
    )).id();
    Some(e)
}

pub fn animate_fx(
    time: Res<Time>,
    mut cmd: Commands,
    mut q: Query<(Entity, &mut FxAnim, &mut Sprite, &mut Transform)>,
    follows: Query<&Transform, Without<FxAnim>>,
) {
    let dt = time.delta_secs();
    for (e, mut a, mut s, mut t) in q.iter_mut() {
        if let Some(u) = a.follow {
            if let Ok(ut) = follows.get(u) {
                t.translation.x = ut.translation.x;
                t.translation.y = ut.translation.y + 40.0;
                t.translation.z = a.z;
            } else {
                cmd.entity(e).despawn();
                continue;
            }
        }
        a.t += dt * FPS;
        while a.t >= 1.0 {
            a.t -= 1.0;
            if a.frame + 1 >= a.frames {
                if a.once { a.done = true; a.frame = a.frames.saturating_sub(1); }
                else { a.frame = 0; }
            } else { a.frame += 1; }
        }
        if let Some(at) = &mut s.texture_atlas { at.index = a.frame.min(a.frames - 1) as usize; }
        if a.life > 0.0 {
            a.life -= dt;
            if a.life <= 0.0 { cmd.entity(e).despawn(); continue; }
        } else if a.once && a.done {
            cmd.entity(e).despawn();
        }
    }
}
