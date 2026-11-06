//! Save slots (own design; the original had a single save): PLAY opens three slots on bg_store with the original
//! notice frame (tanchuang01), button_ok to play a slot, button_no to delete it (press twice). A new slot starts
//! with the intro (SceneBeginAnimation: piantoudonghua-001 to "start" music), then the guided first battle.
use crate::{save::Save, ui::*, AppState};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(AppState::Slots), enter)
        .add_systems(Update, actions.run_if(in_state(AppState::Slots)))
        .add_systems(OnExit(AppState::Slots), despawn_screen)
        .add_systems(OnEnter(AppState::Intro), intro_enter)
        .add_systems(Update, intro.run_if(in_state(AppState::Intro)))
        .add_systems(OnExit(AppState::Intro), despawn_screen);
}

#[derive(Component)]
struct SlotUi;

fn slot_y(i: usize) -> f32 { 120.0 - 165.0 * i as f32 }

fn build(cmd: &mut Commands, a: &AssetServer, f: &Fonts, data: &crate::data::GameData, armed: Option<usize>) {
    for i in 1..=3 {
        let y = slot_y(i - 1);
        let e = image(cmd, a, "ui/tanchuang01.png", Vec2::new(-80.0, y), Some(Vec2::new(640.0, 140.0)), 2.0); cmd.entity(e).insert(SlotUi);
        let desc = match Save::read_slot(i) {
            Some(s) => {
                let lead = s.party.first().and_then(|&r| s.heroes.get(r)).map(|h| format!("{} Lv{}", data.hero(h.hero).class_name, h.level)).unwrap_or_default();
                format!("Slot {i} — Continue\n{lead} · {} heroes · {} gold", s.heroes.len(), s.money())
            }
            None => format!("Slot {i} — Empty\nTap OK to start a New Game"),
        };
        let e = text(cmd, f, desc, Vec2::new(-80.0, y + 6.0), 24.0, Color::srgb(0.3, 0.15, 0.05), 3.0); cmd.entity(e).insert(SlotUi);
        let e = button(cmd, a, "ui/button_ok.png", Some("ui/button_ok_0.png"), Vec2::new(300.0, y), Vec2::new(120.0, 52.0), 3.0, Act::Custom(1, i as i32)); cmd.entity(e).insert(SlotUi);
        if Save::read_slot(i).is_some() {
            let e = button(cmd, a, "ui/button_no.png", Some("ui/button_no_0.png"), Vec2::new(420.0, y), Vec2::new(100.0, 52.0), 3.0, Act::Custom(2, i as i32)); cmd.entity(e).insert(SlotUi);
            if armed == Some(i) { let e = text(cmd, f, "Press again to delete", Vec2::new(-80.0, y - 44.0), 20.0, Color::srgb(0.7, 0.1, 0.05), 3.0); cmd.entity(e).insert(SlotUi); }
        }
    }
}

fn enter(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, data: Res<crate::data::GameData>) {
    let mut s = Sprite::from_image(a.load("ui/bg_store.png"));
    s.rect = Some(Rect::new(50.0, 166.0, 912.0, 606.0)); s.custom_size = Some(SCREEN); s.color = Color::srgb(0.6, 0.6, 0.6);
    cmd.spawn((s, Transform::default(), Scr));
    button(&mut cmd, &a, "ui/button_BACK.png", Some("ui/button_BACK_0.png"), Vec2::new(372.0, 278.0), Vec2::new(176.0, 60.0), 10.0, Act::Goto(AppState::Menu));
    build(&mut cmd, &a, &f, &data, None);
}

#[allow(clippy::too_many_arguments)]
fn actions(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, data: Res<crate::data::GameData>, mut clicked: ResMut<Clicked>, mut save: ResMut<Save>,
           mut next: ResMut<NextState<AppState>>, mut setup: ResMut<crate::battle::BattleSetup>, ui: Query<Entity, With<SlotUi>>, mut armed: Local<Option<usize>>,
           k: Res<ButtonInput<KeyCode>>) {
    if k.just_pressed(KeyCode::Escape) { next.set(AppState::Menu); }
    // keys 1-3 pick a slot (also used by the self-test)
    let mut acts: Vec<Act> = clicked.0.drain(..).collect();
    for (i, key) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3].into_iter().enumerate() { if k.just_pressed(key) { acts.push(Act::Custom(1, i as i32 + 1)); } }
    for act in acts {
        match act {
            Act::Custom(1, i) => {
                let i = i as usize;
                crate::save::SLOT.store(i, std::sync::atomic::Ordering::Relaxed);
                match Save::read_slot(i) {
                    Some(s) => {
                        *save = s;
                        if save.tutorial_done { next.set(AppState::Tavern); } else {
                            *setup = crate::battle::BattleSetup { map: 0, level: (1, 1), tutorial: true, ..default() };
                            next.set(AppState::Battle);
                        }
                    }
                    None => { *save = crate::new_game(&data); save.write(); next.set(AppState::Intro); }
                }
            }
            Act::Custom(2, i) => {
                let i = i as usize;
                if *armed == Some(i) {
                    let _ = std::fs::remove_file(Save::slot_path(i));
                    *armed = None;
                } else { *armed = Some(i); }
                for e in &ui { cmd.entity(e).despawn(); }
                build(&mut cmd, &a, &f, &data, *armed);
            }
            _ => {}
        }
    }
}

#[derive(Component)]
struct IntroFrame;
#[derive(Component)]
struct IntroText;

/// SceneBeginAnimation (piantoudonghua-001, 330 frames at 24 fps, "start" music) then SceneKillAllAnimation
/// (KillAll.xml on map_17, "start2" music: ten heroes walk in, the lich wuyao_datusha burns them with fire_explode,
/// the closing text, leave at 13 s). Both are pre-rendered from the original animations at 12 fps (f000-f164, k000-k191).
/// Frames stream ahead (24 queued) and a frame is only shown once loaded, so nothing flickers; old frames are dropped.
const FPS: f32 = 24.0;
/// frames decoded ahead of the playhead (~2 s), so playback never waits on a JPEG decode
const AHEAD: usize = 48;
const PART1: usize = 330; // piantoudonghua-001, every frame (24 fps)
const TOTAL: usize = PART1 + 437; // KillAll scene rendered at 24 fps
const TEXT_AT: usize = PART1 + 317; // skill at lich frame 125 (5.2 s), SHOWTEXT 7 s later + 1 s fade to black
fn frame_path(i: usize) -> String { if i < PART1 { format!("intro/f{i:03}.jpg") } else { format!("intro/k{:03}.jpg", i - PART1) } }

#[derive(Resource, Default)]
struct IntroPlay { t: f32, shown: usize, handles: std::collections::BTreeMap<usize, Handle<Image>>, music2: bool }

fn intro_enter(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>) {
    let mut p = IntroPlay::default();
    for i in 0..AHEAD { p.handles.insert(i, a.load(frame_path(i))); }
    let mut s = Sprite::from_image(p.handles[&0].clone()); s.custom_size = Some(Vec2::new(960.0, 640.0));
    cmd.spawn((s, Transform::default(), Scr, IntroFrame));
    let t = text(&mut cmd, &f, "By a tragic twist of fate, the young warriors\nleft behind in the village become the new heroes.\nAnd so begins their quest...",
        Vec2::new(0.0, 0.0), 30.0, Color::WHITE, 5.0);
    cmd.entity(t).insert((IntroText, Visibility::Hidden));
    cmd.insert_resource(p);
}

#[allow(clippy::too_many_arguments)]
fn intro(mut cmd: Commands, a: Res<AssetServer>, time: Res<Time<Real>>, mut p: ResMut<IntroPlay>, mut q: Query<&mut Sprite, With<IntroFrame>>, mouse: Res<ButtonInput<MouseButton>>,
         k: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>, mut setup: ResMut<crate::battle::BattleSetup>,
         mut txt: Query<(&mut Visibility, &mut TextColor), With<IntroText>>, save: Res<Save>, music: Query<Entity, With<crate::audio::Music>>) {
    let want = ((p.t + time.delta_secs()) * FPS) as usize;
    if want >= TOTAL || mouse.just_pressed(MouseButton::Left) || k.just_pressed(KeyCode::Space) || k.just_pressed(KeyCode::Escape) {
        *setup = crate::battle::BattleSetup { map: 0, level: (1, 1), tutorial: true, ..default() };
        next.set(AppState::Battle);
        return;
    }
    // hold the clock while the next frame is still loading (no skipped or blank frames)
    let ready = p.handles.get(&want).map(|h| a.is_loaded_with_dependencies(h.id())).unwrap_or(false)
        // before starting, buffer the first second so the opening never stutters
        && (p.t > 0.0 || (0..24).all(|i| p.handles.get(&i).is_some_and(|h| a.is_loaded_with_dependencies(h.id()))));
    if !ready { if !p.handles.contains_key(&want) { p.handles.insert(want, a.load(frame_path(want))); } return; }
    p.t += time.delta_secs();
    if want != p.shown {
        p.shown = want;
        if let Ok(mut s) = q.single_mut() { s.image = p.handles[&want].clone(); }
        for i in want..(want + AHEAD).min(TOTAL) { if !p.handles.contains_key(&i) { let h = a.load(frame_path(i)); p.handles.insert(i, h); } }
        let keep = want.saturating_sub(2);
        p.handles.retain(|&i, _| i >= keep);
        if want >= TEXT_AT { let al = ((want - TEXT_AT) as f32 / FPS).min(1.0); for (mut v, mut c) in &mut txt { *v = Visibility::Inherited; c.0 = Color::srgba(1.0, 1.0, 1.0, al); } }
        if want >= PART1 && !p.music2 { p.music2 = true; crate::audio::music(&mut cmd, &a, &save, &music, "start2.ogg"); }
    }
}
