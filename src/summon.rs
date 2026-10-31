//! Summon (the original's SceneGVGSelectPets "Summon Monsters"): Summon_Monsters_background with a row of
//! Summon_Monsters_kuang02/03 cards, one per captured castle boss (castle_lv "LV:" tag), button_on/button_off to pick the
//! minion that joins battles, and the Summon_Monsters_text01 "Minion Auto-Attack" label (the minion fights on its own).
//! Own design: one active minion, its level follows the castle boss level, no pet skills/upgrades (UserPetSkillList).
use crate::{anim::*, data::GameData, save::Save, ui::*, AppState};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(AppState::Summon), enter)
        .add_systems(Update, actions.run_if(in_state(AppState::Summon)))
        .add_systems(OnExit(AppState::Summon), despawn_screen);
}

fn enter(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, data: Res<GameData>, sheets: Res<Sheets>, save: Res<Save>) {
    let mut s = Sprite::from_image(a.load("ui/bg_store.png"));
    s.rect = Some(Rect::new(50.0, 166.0, 912.0, 606.0)); s.custom_size = Some(SCREEN);
    cmd.spawn((s, Transform::default(), Scr));
    image(&mut cmd, &a, "ui/Summon_Monsters_background.png", Vec2::new(0.0, -20.0), Some(Vec2::new(1136.0, 343.0)), 1.0);
    image(&mut cmd, &a, "ui/bar_summon_monsters.png", Vec2::new(0.0, 250.0), None, 2.0);
    image(&mut cmd, &a, "ui/Summon_Monsters_text01.png", Vec2::new(-300.0, 195.0), None, 2.0);
    text(&mut cmd, &f, "Your minion fights beside your heroes in every battle.", Vec2::new(120.0, 195.0), 16.0, Color::WHITE, 2.0);
    if save.pets.is_empty() {
        text(&mut cmd, &f, "No minions yet.\nCapture castles in SIEGE to win their bosses as minions.", Vec2::new(0.0, -10.0), 24.0, Color::srgb(1.0, 0.9, 0.6), 3.0);
    }
    let n = save.pets.len().min(5);
    for (i, &(boss, lv)) in save.pets.iter().enumerate().take(5) {
        let x = (i as f32 - (n as f32 - 1.0) / 2.0) * 200.0;
        let on = save.active_pet == Some(i);
        image(&mut cmd, &a, if on { "ui/Summon_Monsters_kuang03.png" } else { "ui/Summon_Monsters_kuang02.png" }, Vec2::new(x, -10.0), None, 2.0);
        let key = enemy_key(&data, boss);
        figure(&mut cmd, &data, &sheets, &key, Vec2::new(x, -112.0), fit(&data, &key, 120.0), 3.0);
        text(&mut cmd, &f, format!("{lv}"), Vec2::new(x + 24.0, 99.0), 18.0, Color::srgb(0.3, 0.15, 0.05), 3.5);
        let name = data.siege.castles.iter().find(|c| c.boss == boss).map(|c| c.name.clone()).unwrap_or_else(|| data.enemy(boss).name.clone());
        text(&mut cmd, &f, name, Vec2::new(x, 70.0), 17.0, Color::WHITE, 3.5);
        if on { image(&mut cmd, &a, "ui/Summon_Monsters_kuang04_selected.png", Vec2::new(x, -10.0), Some(Vec2::new(190.0, 262.0)), 2.5); }
        let (n, p) = if on { ("ui/button_off.png", "ui/button_off_0.png") } else { ("ui/button_on.png", "ui/button_on_0.png") };
        button(&mut cmd, &a, n, Some(p), Vec2::new(x, -175.0), Vec2::new(140.0, 60.0), 4.0, Act::Custom(1, i as i32));
    }
    button(&mut cmd, &a, "ui/back.png", Some("ui/back_0.png"), Vec2::new(403.0, 268.0), Vec2::new(180.0, 80.0), 5.0, Act::Goto(AppState::WorldMap));
    button(&mut cmd, &a, "ui/button_siege0.png", Some("ui/button_siege1.png"), Vec2::new(430.0, -255.0), Vec2::new(96.0, 95.0), 5.0, Act::Goto(AppState::Siege));
}

fn actions(mut clicked: ResMut<Clicked>, mut save: ResMut<Save>, mut next: ResMut<NextState<AppState>>, keys: Res<ButtonInput<KeyCode>>, mut sfx: ResMut<crate::audio::Sfx>) {
    if keys.just_pressed(KeyCode::Escape) { next.set(AppState::WorldMap); }
    for act in clicked.0.drain(..) {
        if let Act::Custom(1, i) = act {
            let i = i as usize;
            save.active_pet = if save.active_pet == Some(i) { None } else { Some(i) };
            save.write(); sfx.play("sfx_select");
            next.set(AppState::Summon);
        }
    }
}
