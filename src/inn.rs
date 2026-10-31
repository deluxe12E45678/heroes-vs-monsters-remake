//! SceneGameMenu — the Inn/Tavern hub (bar_bg interior, bar_table_0 counter, two animated innkeepers).
//! The props are hotspots wired to the scenes SceneGameMenu::OnUIEvent switches to; pressing a prop shows
//! the original bar_* sign for it. Positions come from the English build's gamedata/Bar.xml.
use crate::{anim::*, data::GameData, ui::*, AppState};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(AppState::Tavern), enter)
        .add_systems(Update, keys.run_if(in_state(AppState::Tavern)))
        .add_systems(OnExit(AppState::Tavern), despawn_screen);
}

fn enter(mut cmd: Commands, a: Res<AssetServer>, data: Res<GameData>, sheets: Res<Sheets>, f: Res<Fonts>) {
    image(&mut cmd, &a, "ui/bar_bg.png", Vec2::ZERO, Some(SCREEN), 0.0);
    // Layout from the English build's gamedata/Bar.xml (x,y centre-origin, y up, 480×320 units → ×2):
    // role_0 "SKILLS" innkeeper (-170,20), role_0 "STORE" innkeeper (-70,50), IAP_1 chest (-75,-25), IAP_0 gold (-105,-45).
    figure(&mut cmd, &data, &sheets, "npc_skills", obj(-170.0, 20.0), 2.0, 1.0);
    figure(&mut cmd, &data, &sheets, "npc_store", obj(-70.0, 50.0), 2.0, 1.0);
    image(&mut cmd, &a, "ui/bar_table_0.png", obj(-111.25, 70.75), None, 2.0);
    image(&mut cmd, &a, "ui/IAP_1.png", obj(-75.0, -25.0), None, 3.0);
    image(&mut cmd, &a, "ui/IAP_0.png", obj(-105.0, -45.0), None, 4.0);
    // (prop hotspot centre, size, target, bar_* sign and its Bar.xml position). The signs are always shown.
    let props: [(Vec2, Vec2, Act, &str, Vec2); 5] = [
        (obj(-170.0, 45.0), Vec2::new(110.0, 120.0), Act::Goto(AppState::Skills), "ui/bar_skills.png", obj(-175.0, 30.0)),
        (obj(-70.0, 75.0), Vec2::new(110.0, 120.0), Act::Goto(AppState::Store), "ui/bar_store.png", obj(-55.0, 60.0)),
        (obj(12.0, -5.0), Vec2::new(200.0, 100.0), Act::Goto(AppState::Equip), "ui/bar_armory.png", obj(12.0, -5.0)),
        (obj(125.0, -15.0), Vec2::new(130.0, 100.0), Act::Goto(AppState::Hire), "ui/bar_team.png", obj(120.0, 0.0)),
        (obj(-95.0, -38.0), Vec2::new(190.0, 110.0), Act::Stub("GET GOLD (in-app purchase)"), "ui/bar_GETGOLD.png", obj(-105.0, -60.0)),
    ];
    for (c, s, act, sign, at) in props {
        let h_act = act.clone();
        let h = hotspot(&mut cmd, c, s, 10.0, act);
        let lbl = image(&mut cmd, &a, sign, at, None, 20.0);
        // the sign is part of the prop: clicking it does the same as the prop
        cmd.entity(lbl).insert(Hot { size: s * 0.6, act: h_act, normal: None, pressed: None });
        let _ = h;
    }
    // map sign (bar_map, "地图") → SceneMoveMap; siege sign (bar_map2, "攻城战") is a stub like the original's locked mode
    button(&mut cmd, &a, "ui/bar_map.png", Some("ui/bar_map_0.png"), Vec2::new(-405.0, 262.0), Vec2::new(122.0, 96.0), 25.0, Act::Goto(AppState::WorldMap));
    button(&mut cmd, &a, "ui/bar_map2.png", None, Vec2::new(-275.0, 262.0), Vec2::new(122.0, 96.0), 25.0, Act::Goto(AppState::Siege));
    button(&mut cmd, &a, "ui/bar_menu.png", Some("ui/bar_menu_0.png"), Vec2::new(-403.0, -265.0), Vec2::new(146.0, 78.0), 5.0, Act::Goto(AppState::Menu));
    let _ = f;
}

fn keys(k: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if k.just_pressed(KeyCode::Escape) { next.set(AppState::Menu); }
    if k.just_pressed(KeyCode::KeyM) { next.set(AppState::WorldMap); }
}
