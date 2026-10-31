//! Arena (the original's online PVP lobby, ScenePVP*), played offline against the PVP_DummyList.xml teams:
//! arena_background + arena_kuang sign boards with three opponents, arena_fight to battle, the round
//! arena_refresh_ui button rerolls them. Rewards follow PVP_Reward.xml's lobby tier "Low" (medal 4, score +5, half on a loss).
use crate::{battle::BattleSetup, data::GameData, save::Save, ui::*, AppState};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.init_resource::<Picks>()
        .add_systems(OnEnter(AppState::Arena), enter)
        .add_systems(Update, actions.run_if(in_state(AppState::Arena)))
        .add_systems(OnExit(AppState::Arena), despawn_screen);
}

#[derive(Resource, Default)]
pub struct Picks(pub Vec<usize>);

fn reroll(data: &GameData, save: &Save, picks: &mut Picks) {
    let lv = save.party.iter().map(|&r| save.heroes[r].level).sum::<i32>() / save.party.len().max(1) as i32;
    let mut idx: Vec<usize> = (0..data.arena.targets.len()).collect();
    fastrand::shuffle(&mut idx);
    idx.sort_by_key(|&i| (data.arena.targets[i].ave_level - lv).abs() / 4);
    picks.0 = idx.into_iter().take(3).collect();
}

fn enter(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, data: Res<GameData>, save: Res<Save>, mut picks: ResMut<Picks>) {
    if picks.0.is_empty() { reroll(&data, &save, &mut picks); }
    let mut bg = Sprite::from_image(a.load("ui/arena_background.png"));
    bg.rect = Some(Rect::new(0.0, 0.0, 960.0, 640.0)); bg.custom_size = Some(SCREEN);
    cmd.spawn((bg, Transform::from_xyz(0.0, 0.0, 0.0), Scr));
    let mut k = Sprite::from_image(a.load("ui/arena_kuang.png"));
    k.rect = Some(Rect::new(0.0, 0.0, 960.0, 554.0));
    cmd.spawn((k, Transform::from_xyz(0.0, 43.0, 1.0), Scr));
    image(&mut cmd, &a, "ui/arena_title01.png", Vec2::new(0.0, 296.0), None, 2.0);
    for (row, &ti) in picks.0.iter().enumerate() {
        let Some(t) = data.arena.targets.get(ti) else { continue };
        let y = [161.0, 37.0, -92.0][row];
        text(&mut cmd, &f, &t.name, Vec2::new(-240.0, y + 12.0), 24.0, Color::srgb(1.0, 0.95, 0.8), 3.0);
        text(&mut cmd, &f, format!("Lv {}  Rank {}  Score {}", t.ave_level, t.rank, t.score), Vec2::new(-240.0, y - 16.0), 15.0, Color::srgb(0.25, 0.12, 0.05), 3.0);
        let team: Vec<String> = t.team.iter().filter_map(|&(h, _)| data.heroes.heroes.iter().find(|x| x.index == h).map(|x| x.class_name.clone())).collect();
        text(&mut cmd, &f, team.join(", "), Vec2::new(20.0, y), 15.0, Color::srgb(0.25, 0.12, 0.05), 3.0);
        button(&mut cmd, &a, "ui/arena_fight.png", Some("ui/arena_fight_0.png"), Vec2::new(340.0, y), Vec2::new(150.0, 66.0), 3.0, Act::Custom(1, ti as i32));
    }
    text(&mut cmd, &f, format!("Medals {}   Arena score {}", save.medals, save.arena_score), Vec2::new(38.0, -213.0), 20.0, Color::srgb(1.0, 0.95, 0.8), 3.0);
    button(&mut cmd, &a, "ui/arena_refresh_ui.png", Some("ui/arena_refresh_ui_0.png"), Vec2::new(427.0, -213.0), Vec2::new(54.0, 52.0), 3.0, Act::Custom(2, 0));
    button(&mut cmd, &a, "ui/bar_menu.png", Some("ui/bar_menu_0.png"), Vec2::new(-403.0, -265.0), Vec2::new(146.0, 78.0), 3.0, Act::Goto(AppState::WorldMap));
}

#[allow(clippy::too_many_arguments)]
fn actions(mut clicked: ResMut<Clicked>, data: Res<GameData>, save: Res<Save>, mut picks: ResMut<Picks>, mut setup: ResMut<BattleSetup>,
           mut next: ResMut<NextState<AppState>>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Escape) { next.set(AppState::WorldMap); }
    for act in clicked.0.drain(..) {
        match act {
            Act::Custom(1, ti) => {
                let lv = data.arena.targets.get(ti as usize).map(|t| t.ave_level).unwrap_or(1);
                // any wave map for the arena floor; the opponents replace its waves
                let map = data.waves.maps.keys().next().copied().unwrap_or(0);
                *setup = BattleSetup { node: 0, map, level: (lv, lv), boss: false, arena: Some(ti as usize), tutorial: false, siege: None , survival: false };
                next.set(AppState::Battle);
            }
            Act::Custom(2, _) => { reroll(&data, &save, &mut picks); next.set(AppState::Arena); }
            _ => {}
        }
    }
}
