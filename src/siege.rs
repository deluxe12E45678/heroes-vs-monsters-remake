//! Siege (the original's online GVG castle map, SceneGVGMap / SceneGVGCastleInfo), played offline:
//! the GVGConfig.xml castles on the island map (worldmap_daoyu_0) with their chengbao_* boss icons, map2_name_tag
//! name plates, map2_owned flags on captured castles and the map2_requirements bubble on locked ones. Clicking a castle
//! opens the castle_tanchuang04 info board; FIGHT starts a battle in front of castle_background against its boss.
//! Winning captures the castle and adds its boss as a minion for the Summon screen.
//! Own design (the online parts can't be reproduced): castles are captured once and can be re-fought for rewards,
//! there are no other players / rankings / daily challenge timers / castle props, the boss brings 3 guards picked
//! from the world-map waves of a similar level, and the map positions are GVGConfig posX/posY fitted to the screen.
use crate::{anim::*, battle::BattleSetup, data::GameData, save::Save, ui::*, AppState};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(AppState::Siege), enter)
        .add_systems(Update, actions.run_if(in_state(AppState::Siege)))
        .add_systems(OnExit(AppState::Siege), despawn_screen);
}

#[derive(Component)]
struct Popup;

pub fn team_level(save: &Save) -> i32 { save.party.iter().map(|&r| save.heroes[r].level).sum::<i32>() / save.party.len().max(1) as i32 }

fn castle_pos(x: f32, y: f32) -> Vec2 { Vec2::new((x - 233.0) * 1.5, (y - 151.0) * 0.8 + 5.0) }

fn enter(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, data: Res<GameData>, save: Res<Save>) {
    image(&mut cmd, &a, "ui/worldmap_daoyu_0.png", Vec2::ZERO, Some(Vec2::new(1136.0, 1136.0)), 0.0);
    // castle_title reads "MY CASTLE" (the player's base page), so the map gets a plain SIEGE heading (own design)
    text(&mut cmd, &f, "SIEGE", Vec2::new(0.0, 288.0), 40.0, Color::srgb(1.0, 0.8, 0.25), 2.0);
    let tl = team_level(&save);
    for (i, c) in data.siege.castles.iter().enumerate() {
        let mut p = castle_pos(c.x, c.y);
        // Apeman (BOSS_dian_015) sits on top of the Skeleton castle in GVGConfig; nudge any such overlap aside (own design)
        if data.siege.castles[..i].iter().any(|o| castle_pos(o.x, o.y).distance(p) < 60.0) { p += Vec2::new(-150.0, -40.0); }
        let icon = format!("ui/{}.png", c.icon);
        let e = image(&mut cmd, &a, &icon, p, Some(Vec2::splat(96.0)), 3.0 + i as f32 * 0.01);
        let locked = tl < c.min;
        if locked { cmd.entity(e).insert(Sprite { image: a.load(icon), custom_size: Some(Vec2::splat(96.0)), color: Color::srgb(0.45, 0.45, 0.45), ..default() }); }
        image(&mut cmd, &a, "ui/map2_name_tag.png", p + Vec2::new(0.0, -52.0), Some(Vec2::new(120.0, 50.0)), 4.0);
        text(&mut cmd, &f, format!("{}\nLv {}", c.name, c.level), p + Vec2::new(0.0, -52.0), 13.0, Color::srgb(0.25, 0.12, 0.05), 4.5);
        if save.castles.contains(&i) { image(&mut cmd, &a, "ui/map2_owned.png", p + Vec2::new(36.0, 30.0), Some(Vec2::splat(40.0)), 4.6); }
        if locked { image(&mut cmd, &a, "ui/map2_locked.png", p + Vec2::new(0.0, 8.0), Some(Vec2::new(32.0, 36.0)), 4.6); }
        hotspot(&mut cmd, p, Vec2::new(96.0, 110.0), 5.0, Act::Custom(1, i as i32));
    }
    text(&mut cmd, &f, format!("Team Lv {}   Castles {}/{}   Badges {}   Guardian Stars {}", tl, save.castles.len(), data.siege.castles.len(), save.badges, save.stars),
        Vec2::new(0.0, 256.0), 17.0, Color::srgb(1.0, 0.95, 0.8), 6.0);
    button(&mut cmd, &a, "ui/bar_menu.png", Some("ui/bar_menu_0.png"), Vec2::new(-403.0, -265.0), Vec2::new(146.0, 78.0), 6.0, Act::Goto(AppState::WorldMap));
    button(&mut cmd, &a, "ui/button_summon0.png", Some("ui/button_summon1.png"), Vec2::new(430.0, -250.0), Vec2::new(96.0, 95.0), 6.0, Act::Goto(AppState::Summon));
}

#[allow(clippy::too_many_arguments)]
fn actions(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, data: Res<GameData>, sheets: Res<Sheets>, save: Res<Save>, mut clicked: ResMut<Clicked>,
           pop: Query<Entity, With<Popup>>, mut setup: ResMut<BattleSetup>, mut next: ResMut<NextState<AppState>>, keys: Res<ButtonInput<KeyCode>>,
           mut sfx: ResMut<crate::audio::Sfx>) {
    if keys.just_pressed(KeyCode::Escape) { if pop.is_empty() { next.set(AppState::WorldMap) } else { for e in pop.iter() { cmd.entity(e).despawn(); } } }
    for act in clicked.0.drain(..) {
        match act {
            Act::Custom(1, i) if pop.is_empty() => {
                let Some(c) = data.siege.castles.get(i as usize) else { continue };
                sfx.play("sfx_select");
                let tl = team_level(&save);
                let x = c.level as f32;
                let s = &data.siege;
                let mut es = vec![];
                let mut dim = Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.5), SCREEN); dim.custom_size = Some(SCREEN);
                es.push(cmd.spawn((dim, Transform::from_xyz(0.0, 0.0, 30.0), Scr, Hot { size: SCREEN, act: Act::Custom(9, 0), normal: None, pressed: None })).id());
                es.push(image(&mut cmd, &a, "ui/castle_tanchuang04.png", Vec2::new(0.0, 0.0), None, 31.0));
                es.push(text(&mut cmd, &f, format!("{} Castle", c.name), Vec2::new(0.0, 150.0), 26.0, Color::srgb(0.45, 0.12, 0.05), 32.0));
                let key = enemy_key(&data, c.boss);
                es.push(figure(&mut cmd, &data, &sheets, &key, Vec2::new(-175.0, -60.0), fit(&data, &key, 140.0), 32.0));
                let owned = save.castles.contains(&(i as usize));
                let body = format!("Boss: {}  Lv {}\nRequirements: Team Lv {}+\n\nRewards:\nGuardian Stars {}\nBadges {}\nGold {}\nEXP {}\n{}",
                    c.name, c.level, c.min, SiegeTableEval::f(s.stars, x), SiegeTableEval::f(s.badges, x), SiegeTableEval::f(s.money, x), SiegeTableEval::f(s.exp, x),
                    if owned { "Captured" } else { "Win to capture it and gain\nits boss as a minion" });
                let t = text(&mut cmd, &f, body, Vec2::new(-90.0, 20.0), 15.0, Color::srgb(0.3, 0.15, 0.05), 32.0);
                cmd.entity(t).insert(bevy::sprite::Anchor::CENTER_LEFT);
                es.push(t);
                if tl >= c.min {
                    es.push(button(&mut cmd, &a, "ui/button_fight.png", Some("ui/button_fight_0.png"), Vec2::new(70.0, -150.0), Vec2::new(140.0, 60.0), 33.0, Act::Custom(2, i)));
                } else {
                    es.push(image(&mut cmd, &a, "ui/map2_requirements.png", Vec2::new(70.0, -150.0), None, 33.0));
                    es.push(text(&mut cmd, &f, format!("Team Lv {} needed", c.min), Vec2::new(70.0, -178.0), 16.0, Color::srgb(1.0, 0.4, 0.3), 33.0));
                }
                for e in es { cmd.entity(e).insert(Popup); }
            }
            Act::Custom(9, _) => for e in pop.iter() { cmd.entity(e).despawn(); },
            Act::Custom(2, i) => {
                let Some(c) = data.siege.castles.get(i as usize) else { continue };
                let map = data.world.nodes.iter().min_by_key(|n| (n.max_level - c.level).abs()).and_then(|n| n.maps.first().copied())
                    .unwrap_or_else(|| data.waves.maps.keys().next().copied().unwrap_or(0));
                *setup = BattleSetup { node: 0, map, level: ((c.level - 2).max(1), c.level), boss: false, arena: None, tutorial: false, siege: Some(i as usize) , survival: false };
                next.set(AppState::Battle);
            }
            _ => {}
        }
    }
}

struct SiegeTableEval;
impl SiegeTableEval { fn f(v: (f32, f32, f32), x: f32) -> i64 { crate::data::SiegeTable::eval(v, x) } }
