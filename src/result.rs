//! Result screen: STAGECLEAR / GAMEOVER banner (original textures), rewards, level-ups, button_ok → world map.
use crate::{ui::*, AppState};
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct Outcome { pub won: bool, pub money: i64, pub exp: i64, pub lines: Vec<String>, pub survival: Option<(i32, i32, Option<i32>, usize)> }

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(AppState::Result), enter)
        .add_systems(Update, keys.run_if(in_state(AppState::Result)))
        .add_systems(OnExit(AppState::Result), despawn_screen);
}

fn enter(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, o: Res<Outcome>, mut sfx: ResMut<crate::audio::Sfx>, data: Res<crate::data::GameData>) {
    let mut s = Sprite::from_image(a.load("ui/bg_store.png"));
    s.rect = Some(Rect::new(50.0, 166.0, 912.0, 606.0)); s.custom_size = Some(SCREEN); s.color = Color::srgb(0.5, 0.5, 0.5);
    cmd.spawn((s, Transform::default(), Scr));
    if let Some((kills, waves, item, tier)) = o.survival {
        // Infinite_shut (690x438): CHALLENGE RESULTS, Kills / Waves Complete boxes, 5 loot boxes (1-8 … 45+)
        let c = Vec2::new(0.0, 30.0);
        let px = |x: f32, y: f32| c + Vec2::new(x - 345.0, 219.0 - y);
        image(&mut cmd, &a, "ui/Infinite_shut.png", c, None, 2.0);
        text(&mut cmd, &f, format!("{kills}"), px(466.0, 140.0), 30.0, Color::WHITE, 3.0);
        text(&mut cmd, &f, format!("{waves}"), px(466.0, 190.0), 30.0, Color::WHITE, 3.0);
        let bx = [133.0, 229.0, 327.0, 423.0, 520.0];
        image(&mut cmd, &a, "ui/Infinite_shut_Redborder.png", px(bx[tier], 300.0), None, 3.0);
        if let Some(p) = item.and_then(|i| crate::common::item_icon_path(&data, i)) { image(&mut cmd, &a, &p, px(bx[tier], 300.0), Some(Vec2::splat(56.0)), 3.5); }
        text(&mut cmd, &f, format!("Gold +{}    {}", o.money, item.and_then(|i| data.item(i)).map(|i| format!("Loot: {}", i.name)).unwrap_or_else(|| "No loot".into())), Vec2::new(0.0, -215.0), 22.0, Color::WHITE, 3.0);
        sfx.play("sfx_lose");
        button(&mut cmd, &a, "ui/button_ok.png", Some("ui/button_ok_0.png"), Vec2::new(0.0, -270.0), Vec2::new(140.0, 60.0), 3.0, Act::Goto(AppState::WorldMap));
        return;
    }
    // dark plate so STAGECLEAR / rewards never blend into the faded bg
    let mut plate = Sprite::from_color(Color::srgba(0.05, 0.03, 0.01, 0.72), Vec2::new(700.0, 420.0));
    plate.custom_size = Some(Vec2::new(700.0, 420.0));
    cmd.spawn((plate, Transform::from_xyz(0.0, 10.0, 1.5), Scr));
    image(&mut cmd, &a, if o.won { "ui/STAGECLEAR.png" } else { "ui/GAMEOVER.png" }, Vec2::new(0.0, 180.0), None, 2.0);
    sfx.play(if o.won { "sfx_win" } else { "sfx_lose" });
    let mut body = format!("Gold +{}    EXP +{}\n", o.money, o.exp);
    for l in &o.lines { body += l; body.push('\n'); }
    text(&mut cmd, &f, body, Vec2::new(0.0, 20.0), 26.0, Color::srgb(1.0, 0.96, 0.85), 2.0);
    text(&mut cmd, &f, "Tap OK to continue", Vec2::new(0.0, -140.0), 18.0, Color::srgb(1.0, 0.85, 0.45), 2.5);
    button(&mut cmd, &a, "ui/button_ok.png", Some("ui/button_ok_0.png"), Vec2::new(0.0, -200.0), Vec2::new(140.0, 60.0), 3.0, Act::Goto(AppState::WorldMap));
}

fn keys(k: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if k.any_just_pressed([KeyCode::Enter, KeyCode::Space, KeyCode::Escape]) { next.set(AppState::WorldMap); }
}
