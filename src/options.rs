//! SceneOptions: bg_mainmenu, OPTIONS title, music / sound tick boxes (tick_BG + tick), O_RESET with a yes/no confirm
//! (team_description_0 panel), CREDITS, and button_menu back to SceneMenu. Textures are the original's; the positions
//! are laid out by eye (SceneOptions' rects were not decoded).
use crate::{data::GameData, save::Save, ui::*, AppState};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(AppState::Options), enter)
        .add_systems(Update, actions.run_if(in_state(AppState::Options)))
        .add_systems(OnExit(AppState::Options), despawn_screen);
}

#[derive(Component)]
struct Tick(u8);
#[derive(Component)]
struct Confirm;

fn enter(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, save: Res<Save>) {
    image(&mut cmd, &a, "ui/bg_mainmenu.png", Vec2::ZERO, Some(SCREEN), 0.0);
    let mut dim = Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.45), SCREEN);
    dim.custom_size = Some(SCREEN);
    cmd.spawn((dim, Transform::from_xyz(0.0, 0.0, 0.5), Scr));
    image(&mut cmd, &a, "ui/OPTIONS.png", Vec2::new(0.0, 230.0), None, 2.0);
    for (i, (lbl, name, y, on)) in [("ui/music.png", "Music", 110.0, save.music), ("ui/sound.png", "Sound", 30.0, save.sound)].into_iter().enumerate() {
        // parchment plate behind the dark music/sound PNGs so the labels stay readable
        let mut plate = Sprite::from_color(Color::srgba(0.55, 0.42, 0.22, 0.85), Vec2::new(340.0, 70.0));
        plate.custom_size = Some(Vec2::new(340.0, 70.0));
        cmd.spawn((plate, Transform::from_xyz(0.0, y, 1.5), Scr));
        let mut sp = Sprite::from_image(a.load(lbl));
        sp.color = Color::srgb(1.4, 1.3, 1.1); // brighten dark label art
        cmd.spawn((sp, Transform::from_xyz(-90.0, y, 2.0), Scr));
        text(&mut cmd, &f, name, Vec2::new(-90.0, y - 28.0), 18.0, Color::srgb(1.0, 0.96, 0.8), 2.5);
        image(&mut cmd, &a, "ui/tick_BG.png", Vec2::new(90.0, y), None, 2.0);
        let t = image(&mut cmd, &a, "ui/tick.png", Vec2::new(92.0, y + 4.0), None, 2.5);
        cmd.entity(t).insert((Tick(i as u8), if on { Visibility::Inherited } else { Visibility::Hidden }));
        hotspot(&mut cmd, Vec2::new(0.0, y), Vec2::new(320.0, 60.0), 3.0, Act::Custom(1, i as i32));
    }
    button(&mut cmd, &a, "ui/O_RESET.png", Some("ui/O_RESET_0.png"), Vec2::new(-150.0, -90.0), Vec2::new(140.0, 60.0), 2.0, Act::Custom(2, 0));
    button(&mut cmd, &a, "ui/button_credits.png", Some("ui/button_CREDITS_0.png"), Vec2::new(150.0, -90.0), Vec2::new(254.0, 84.0), 2.0,
        Act::Custom(4, 0));
    button(&mut cmd, &a, "ui/button_menu.png", Some("ui/button_menu_0.png"), Vec2::new(-403.0, -265.0), Vec2::new(140.0, 60.0), 2.0, Act::Goto(AppState::Menu));
}

fn actions(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, data: Res<GameData>, mut save: ResMut<Save>, mut clicked: ResMut<Clicked>,
           mut ticks: Query<(&Tick, &mut Visibility)>, confirm: Query<Entity, With<Confirm>>, mut toast: ResMut<Toast>,
           keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if keys.just_pressed(KeyCode::Escape) { next.set(AppState::Menu); }
    for act in clicked.0.drain(..) {
        match act {
            Act::Custom(1, i) if confirm.is_empty() => {
                if i == 0 { save.music = !save.music } else { save.sound = !save.sound }
                save.write();
                for (t, mut v) in ticks.iter_mut() {
                    let on = if t.0 == 0 { save.music } else { save.sound };
                    *v = if on { Visibility::Inherited } else { Visibility::Hidden };
                }
            }
            Act::Custom(2, 0) if confirm.is_empty() => {
                let p = image(&mut cmd, &a, "ui/team_description_0.png", Vec2::new(0.0, 0.0), None, 40.0);
                let t = text(&mut cmd, &f, "Reset all progress?\nThis cannot be undone.", Vec2::new(0.0, 40.0), 26.0, Color::WHITE, 41.0);
                let y = button(&mut cmd, &a, "ui/button_yes.png", Some("ui/button_yes_0.png"), Vec2::new(-110.0, -70.0), Vec2::new(140.0, 60.0), 41.0, Act::Custom(3, 1));
                let n = button(&mut cmd, &a, "ui/button_no.png", Some("ui/button_no_0.png"), Vec2::new(110.0, -70.0), Vec2::new(140.0, 60.0), 41.0, Act::Custom(3, 0));
                for e in [p, t, y, n] { cmd.entity(e).insert(Confirm); }
            }
            Act::Custom(3, yes) => {
                for e in confirm.iter() { cmd.entity(e).despawn(); }
                if yes == 1 {
                    let keep = (save.sound, save.music);
                    *save = crate::new_game(&data);
                    save.sound = keep.0; save.music = keep.1;
                    save.write();
                    toast.0 = Some(("Progress reset".into(), 2.0));
                }
            }
            // SceneCredits: bg_mainmenu + the CREDITS sheet + back. (The English build ships no about_dx/about_mm art,
            // so the "About" line under it is this recreation's own text.)
            Act::Custom(4, 0) if confirm.is_empty() => {
                let bg = image(&mut cmd, &a, "ui/bg_mainmenu.png", Vec2::ZERO, Some(SCREEN), 40.0);
                let mut dim = Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.3), SCREEN); dim.custom_size = Some(SCREEN);
                let d = cmd.spawn((dim, Transform::from_xyz(0.0, 0.0, 40.5), Scr)).id();
                let c = image(&mut cmd, &a, "ui/CREDITS.png", Vec2::new(0.0, 40.0), None, 41.0);
                let t = text(&mut cmd, &f, "Heroes vs Monsters (c) Triniti Interactive. Offline fan recreation in Bevy.", Vec2::new(0.0, -215.0), 18.0, Color::srgb(0.85, 0.75, 0.55), 41.0);
                let b = button(&mut cmd, &a, "ui/back.png", Some("ui/back_0.png"), Vec2::new(-403.0, -265.0), Vec2::new(180.0, 80.0), 42.0, Act::Custom(3, 0));
                for e in [bg, d, c, t, b] { cmd.entity(e).insert(Confirm); }
            }
            _ => {}
        }
    }
}
