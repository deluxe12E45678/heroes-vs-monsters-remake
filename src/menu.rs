//! SceneMenu: bg_mainmenu + title_android + the wooden sign post (button_mainmenu1) with
//! PLAY / OPTIONS / HOW TO and the 4th (iOS Leaderboards) sign, which here is QUIT.
//! Positions are the values decoded from SceneMenu::init (UI rects in 480×320, y down).
use crate::{ui::*, AppState};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(AppState::Menu), enter)
        .add_systems(Update, actions.run_if(in_state(AppState::Menu)))
        .add_systems(OnExit(AppState::Menu), despawn_screen);
}

#[derive(Component)]
struct Help(usize);

fn enter(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>) {
    image(&mut cmd, &a, "ui/bg_mainmenu.png", Vec2::ZERO, Some(SCREEN), 0.0);
    image(&mut cmd, &a, "ui/title_android.png", obj(-85.0, 15.0), Some(Vec2::new(502.0, 584.0)), 1.0);
    image(&mut cmd, &a, "ui/button_mainmenu1.png", obj(146.0, 25.0), Some(Vec2::new(358.0, 544.0)), 1.0);
    // Slightly shorter hit boxes + more y-gap so OPTIONS / HOW TO never steal each other's taps.
    let signs: [(f32, &str, Option<&str>, Act); 3] = [
        (62.0, "ui/button_play.png", Some("ui/button_play_p.png"), Act::Custom(5, 0)),
        (108.0, "ui/button_options.png", Some("ui/button_options_p.png"), Act::Goto(AppState::Options)),
        (154.0, "ui/button_howto.png", Some("ui/button_howto_p.png"), Act::Custom(1, 0)),
    ];
    for (y, n, p, act) in signs {
        let (c, s) = ui_rect(320.0, y, 145.0, 38.0);
        button(&mut cmd, &a, n, p, c, s, 2.0, act);
    }
    // 4th sign: the blank plank is part of button_mainmenu1; the iOS build put Leaderboards there.
    let (c, s) = ui_rect(320.0, 205.0, 145.0, 38.0);
    hotspot(&mut cmd, c, s, 2.0, Act::Quit);
    text(&mut cmd, &f, "QUIT", c + Vec2::new(0.0, 4.0), 30.0, Color::srgb(0.93, 0.66, 0.24), 3.0);
}

fn actions(mut cmd: Commands, a: Res<AssetServer>, mut clicked: ResMut<Clicked>, save: Res<crate::save::Save>, mut setup: ResMut<crate::battle::BattleSetup>, mut next: ResMut<NextState<AppState>>, mut help: Query<(Entity, &mut Help, &mut Sprite)>,
           keys: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) { exit.write(AppExit::Success); }
    for act in clicked.0.drain(..) {
        if let Act::Custom(c, _) = act {
            match c {
                1 => { // HOW TO: help_1..4 pages, click to advance
                    let mut s = Sprite::from_image(a.load("ui/help_1.png"));
                    s.custom_size = Some(SCREEN);
                    let e = cmd.spawn((s, Transform::from_xyz(0.0, 0.0, 50.0), Scr, Help(1))).id();
                    cmd.entity(e).insert(Hot { size: SCREEN, act: Act::Custom(2, 0), normal: None, pressed: None });
                }
                2 => if let Ok((e, mut h, mut s)) = help.single_mut() {
                    h.0 += 1;
                    if h.0 > 4 { cmd.entity(e).despawn(); } else { s.image = a.load(format!("ui/help_{}.png", h.0)); }
                },
                // PLAY: a new game starts with the guided battle (SceneHelpGame), then the inn
                // PLAY → save slots (own design); a new slot plays the intro, then the guided battle
                5 => { let _ = (&save, &mut setup, data_map0()); next.set(AppState::Slots) },
                _ => {}
            }
        }
    }
}

fn data_map0() -> u32 { 0 }
