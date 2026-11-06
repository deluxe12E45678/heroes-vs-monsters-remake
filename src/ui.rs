//! Small UI kit in world space (no bevy_ui): textured sprites, hotspots/buttons, text, and cursor tracking.
//! Layout units: the original 480×320 logical screen drawn at 2× (= the *_2x art at 1:1) inside a 1136×640 window;
//! the extra 88-px columns hold the hvsMiphon5_left/right frame pillars (the original widescreen border).
use bevy::prelude::*;
use bevy::text::FontSize;

pub const SCREEN: Vec2 = Vec2::new(960.0, 640.0);
pub const WINDOW: Vec2 = Vec2::new(1136.0, 640.0);

/// Every screen entity carries this; it is despawned when the state exits.
#[derive(Component)]
pub struct Scr;

#[derive(Resource)]
pub struct Fonts { pub main: Handle<Font> }
impl FromWorld for Fonts { fn from_world(w: &mut World) -> Self { Fonts { main: w.resource::<AssetServer>().load("fonts/gilsanub.ttf") } } }

#[derive(Resource, Default)]
pub struct Cursor { pub world: Option<Vec2>, pub pressed_on: Option<Entity> }

/// What a hotspot does when clicked. Screens read `Clicked` and handle their own `Custom` codes.
#[derive(Clone, Debug, PartialEq)]
pub enum Act { Goto(crate::AppState), Quit, ToggleSound, Stub(&'static str), Custom(u32, i32) }

#[derive(Resource, Default)]
pub struct Clicked(pub Vec<Act>);

#[derive(Component)]
pub struct Hot { pub size: Vec2, pub act: Act, pub normal: Option<Handle<Image>>, pub pressed: Option<Handle<Image>> }

/// Shown only while the owning hotspot is hovered (Inn prop labels).
#[derive(Component)]
pub struct HoverShow(pub Entity);

/// Original UI Rect (x, y, w, h) in 480×320 top-left space → (center, size) in world px.
pub fn ui_rect(x: f32, y: f32, w: f32, h: f32) -> (Vec2, Vec2) {
    (Vec2::new((x + w / 2.0 - 240.0) * 2.0, (160.0 - y - h / 2.0) * 2.0), Vec2::new(w * 2.0, h * 2.0))
}
/// Original GameObject position (center origin, y up, 480×320 units) → world px.
pub fn obj(x: f32, y: f32) -> Vec2 { Vec2::new(x * 2.0, y * 2.0) }

pub fn image(cmd: &mut Commands, a: &AssetServer, path: &str, pos: Vec2, size: Option<Vec2>, z: f32) -> Entity {
    let mut s = Sprite::from_image(a.load(path.to_string()));
    s.custom_size = size;
    cmd.spawn((s, Transform::from_translation(pos.extend(z)), Scr)).id()
}

pub fn button(cmd: &mut Commands, a: &AssetServer, normal: &str, pressed: Option<&str>, pos: Vec2, size: Vec2, z: f32, act: Act) -> Entity {
    let n: Handle<Image> = a.load(normal.to_string());
    let mut s = Sprite::from_image(n.clone());
    s.custom_size = Some(size);
    cmd.spawn((s, Transform::from_translation(pos.extend(z)), Scr,
        Hot { size, act, normal: Some(n), pressed: pressed.map(|p| a.load(p.to_string())) })).id()
}

/// Invisible clickable area.
pub fn hotspot(cmd: &mut Commands, pos: Vec2, size: Vec2, z: f32, act: Act) -> Entity {
    cmd.spawn((Transform::from_translation(pos.extend(z)), Visibility::default(), Scr, Hot { size, act, normal: None, pressed: None })).id()
}

pub fn text(cmd: &mut Commands, f: &Fonts, s: impl Into<String>, pos: Vec2, size: f32, color: Color, z: f32) -> Entity {
    cmd.spawn((Text2d::new(s.into()), TextFont { font: f.main.clone().into(), font_size: FontSize::Px(size), ..default() },
        TextColor(color), Transform::from_translation(pos.extend(z)), Scr)).id()
}

pub fn track_cursor(windows: Query<&Window>, cam: Query<(&Camera, &GlobalTransform)>, mut c: ResMut<Cursor>, tp: Res<TouchPos>) {
    c.world = None;
    let (Ok(w), Ok((cam, gt))) = (windows.single(), cam.single()) else { return };
    if let Some(p) = w.cursor_position().or(tp.0) { c.world = cam.viewport_to_world_2d(gt, p).ok(); }
}

/// Touch → mouse: the first finger acts as the left mouse button (tap = click, drag = drag), its motion feeds
/// MouseMotion (hire-list scrolling), and the Android back key acts as Escape. Desktop mouse/keyboard is untouched.
#[derive(Resource, Default)]
pub struct TouchPos(pub Option<Vec2>, pub Option<u64>);
pub fn touch_to_mouse(touches: Res<Touches>, mut mouse: ResMut<ButtonInput<MouseButton>>, mut keys: ResMut<ButtonInput<KeyCode>>,
                      mut tp: ResMut<TouchPos>, mut motion: MessageWriter<bevy::input::mouse::MouseMotion>) {
    if keys.just_pressed(KeyCode::BrowserBack) { keys.press(KeyCode::Escape); }
    if keys.just_released(KeyCode::BrowserBack) { keys.release(KeyCode::Escape); }
    if let Some(id) = tp.1 {
        if let Some(t) = touches.get_pressed(id) {
            tp.0 = Some(t.position());
            if t.delta() != Vec2::ZERO { motion.write(bevy::input::mouse::MouseMotion { delta: t.delta() }); }
        } else {
            if let Some(t) = touches.get_released(id) { tp.0 = Some(t.position()); }
            mouse.release(MouseButton::Left); tp.1 = None;
        }
    } else if let Some(t) = touches.iter_just_pressed().next() {
        tp.0 = Some(t.position()); tp.1 = Some(t.id()); mouse.press(MouseButton::Left);
    }
}

pub fn hot_at(p: Vec2, q: &Query<(Entity, &Hot, &GlobalTransform, &InheritedVisibility)>) -> Option<Entity> {
    q.iter().filter(|(_, h, t, v)| v.get() && {
        let c = t.translation().truncate();
        (p - c).abs().cmple(h.size / 2.0).all()
    }).max_by(|a, b| a.2.translation().z.total_cmp(&b.2.translation().z)).map(|x| x.0)
}

/// Press/release on the same hotspot = click. Swaps to the pressed texture while held.
pub fn click_system(mouse: Res<ButtonInput<MouseButton>>, mut cur: ResMut<Cursor>, hots: Query<(Entity, &Hot, &GlobalTransform, &InheritedVisibility)>,
                    mut sprites: Query<&mut Sprite>, mut clicked: ResMut<Clicked>, mut sfx: ResMut<crate::audio::Sfx>) {
    let Some(p) = cur.world else { return };
    if mouse.just_pressed(MouseButton::Left) {
        cur.pressed_on = hot_at(p, &hots);
        if let Some(e) = cur.pressed_on {
            if let (Ok((_, h, _, _)), Ok(mut s)) = (hots.get(e), sprites.get_mut(e)) { if let Some(pr) = &h.pressed { s.image = pr.clone(); } }
        }
    }
    if mouse.just_released(MouseButton::Left) {
        if let Some(e) = cur.pressed_on.take() {
            if let Ok((_, h, _, _)) = hots.get(e) {
                if let (Some(n), Ok(mut s)) = (&h.normal, sprites.get_mut(e)) { s.image = n.clone(); }
                if hot_at(p, &hots) == Some(e) { clicked.0.push(h.act.clone()); sfx.play("sfx_click"); }
            }
        }
    }
}

/// Prop signs show only while the mouse button is held on that prop (the touch original has no hover state).
pub fn hover_labels(cur: Res<Cursor>, mouse: Res<ButtonInput<MouseButton>>, hots: Query<(Entity, &Hot, &GlobalTransform, &InheritedVisibility)>,
                    mut q: Query<(&HoverShow, &mut Visibility)>) {
    let over = if mouse.pressed(MouseButton::Left) { cur.world.and_then(|p| hot_at(p, &hots)).filter(|e| cur.pressed_on == Some(*e)) } else { None };
    for (h, mut v) in q.iter_mut() { *v = if Some(h.0) == over { Visibility::Inherited } else { Visibility::Hidden }; }
}

/// Generic handling of Goto / Quit / Stub / ToggleSound; screen-specific codes stay in the queue for the screen.
pub fn generic_actions(mut clicked: ResMut<Clicked>, mut next: ResMut<NextState<crate::AppState>>, mut exit: MessageWriter<AppExit>,
                       mut save: ResMut<crate::save::Save>, mut toast: ResMut<Toast>) {
    clicked.0.retain(|a| match a {
        Act::Goto(s) => { next.set(*s); false }
        Act::Quit => { exit.write(AppExit::Success); false }
        Act::ToggleSound => { save.sound = !save.sound; save.write(); toast.0 = Some((if save.sound { "Sound ON" } else { "Sound OFF" }.into(), 1.5)); false }
        Act::Stub(n) => { toast.0 = Some((format!("{n}"), 2.5)); false }
        Act::Custom(..) => true,
    });
}

/// One-line message banner (bottom centre).
#[derive(Resource, Default)]
pub struct Toast(pub Option<(String, f32)>);
#[derive(Component)]
pub struct ToastText;

pub fn toast_system(time: Res<Time<Real>>, mut t: ResMut<Toast>, mut q: Query<(&mut Text2d, &mut Visibility), With<ToastText>>) {
    let Ok((mut tx, mut v)) = q.single_mut() else { return };
    match &mut t.0 {
        Some((s, ttl)) => { tx.0 = s.clone(); *v = Visibility::Visible; *ttl -= time.delta_secs(); if *ttl <= 0.0 { t.0 = None; } }
        None => *v = Visibility::Hidden,
    }
}

pub fn despawn_screen(mut cmd: Commands, q: Query<Entity, With<Scr>>, mut clicked: ResMut<Clicked>) {
    for e in q.iter() { cmd.entity(e).try_despawn(); }
    clicked.0.clear();
}

/// Give every new sprite a distinct z: Bevy 0.20 batches sprites via Mesh2d/Material2d and sprites at an identical z
/// got their textures/draw order mixed (title and sign-post swapped, HOW TO drawn on the left pillar).
pub fn unique_sprite_z(mut q: Query<&mut Transform, Added<Sprite>>, mut n: Local<u32>) {
    for mut t in q.iter_mut() { *n = (*n + 1) % 4000; t.translation.z += *n as f32 * 1e-5; }
}
