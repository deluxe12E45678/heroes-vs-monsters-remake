//! Heroes vs Monsters — Bevy recreation prototype built on the decoded APK data and art.
//! Screens follow the original scene classes: SceneMenu → SceneGameMenu (the Inn/Tavern hub) →
//! SceneMoveMap (world map) / SceneSelectHero (hire) / SceneShop / SceneUserBag (equip) / SceneSkillList → battle.
mod anim;
mod autotest;
mod options;
mod arena;
mod siege;
mod summon;
mod audio;
mod battle;
mod combat;
mod common;
mod data;
mod equip;
mod hire;
mod inn;
mod menu;
mod result;
mod save;
mod skills;
mod slots;
mod stats;
mod store;
#[allow(dead_code)]
mod types;
mod ui;
mod worldmap;

use bevy::prelude::*;
use bevy::window::WindowResolution;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState { #[default] Menu, Tavern, Hire, WorldMap, Store, Equip, Skills, Battle, Result, Options, Arena, Siege, Summon, Slots, Intro }

/// Roster index whose equipment/skills are shown on the Equip/Skills screens.
#[derive(Resource, Default)]
pub struct Selected(pub usize);

#[bevy::prelude::bevy_main]
pub fn main() {
    let data = data::GameData::load().unwrap_or_else(|e| panic!("failed to load assets/data: {e}"));
    let mut save = save::Save::load_or_new(|| new_game(&data));
    // self-test hook: HVSM_TESTLEVEL=n levels every hero (only meant together with a throwaway HVSM_SAVE)
    if let (Ok(l), Ok(_)) = (std::env::var("HVSM_TESTLEVEL"), std::env::var("HVSM_SAVE")) {
        if let Ok(l) = l.parse::<i32>() { for h in &mut save.heroes { h.level = l; } }
    }
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { mode: if cfg!(target_os = "android") { bevy::window::WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Primary) } else { bevy::window::WindowMode::Windowed }, title: "Heroes vs Monsters".into(), resolution: { let (w, h) = std::env::var("HVSM_WINDOW").ok().and_then(|v| { let mut it = v.split(',').filter_map(|x| x.parse::<u32>().ok()); Some((it.next()?, it.next()?)) }).unwrap_or((1136, 640)); WindowResolution::new(w, h) }, ..default() }),
            ..default()
        }).set(bevy::render::RenderPlugin { render_creation: render_settings().into(), ..default() })
        .set(AssetPlugin { file_path: if cfg!(target_os = "android") { "".into() } else { data::asset_dir().to_string_lossy().into_owned() }, ..default() }))
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(data)
        .insert_resource(save)
        .init_resource::<ui::Fonts>()
        .init_resource::<anim::Sheets>()
        .init_resource::<ui::Cursor>()
        .init_resource::<ui::TouchPos>()
        .init_resource::<ui::Clicked>()
        .init_resource::<ui::Toast>()
        .init_resource::<audio::Sfx>()
        .init_resource::<Selected>()
        .init_resource::<battle::BattleSetup>()
        .init_resource::<result::Outcome>()
        .init_state::<AppState>()
        .add_systems(PostUpdate, ui::unique_sprite_z)
        // Clicks are resolved (and Goto/Quit/ToggleSound/Stub handled) in PreUpdate, so every screen's Update system only
        // ever sees its own Custom codes. Previously a screen could drain the queue before generic_actions ran and swallow
        // BACK/PLAY clicks at random.
        .add_systems(PreUpdate, (ui::touch_to_mouse, ui::track_cursor, ui::click_system, ui::generic_actions).chain().after(bevy::input::InputSystems))
        .add_systems(Update, (ui::hover_labels, ui::toast_system, anim::animate, anim::build_rigs, anim::pose_rigs.after(anim::animate),
                              audio::play_sfx, common::update_gold, common::fit_desc, state_music, fit_camera).chain())
        .add_plugins((menu::plugin, inn::plugin, hire::plugin, worldmap::plugin, store::plugin, equip::plugin, skills::plugin, battle::plugin, result::plugin, autotest::plugin, options::plugin, arena::plugin, siege::plugin, summon::plugin, slots::plugin));
    // PreStartup runs before the initial state's OnEnter. (Spawning these before app.run(), i.e. before the render
    // plugins finished, desynced the render world: sprites were drawn with other entities' transforms.)
    app.add_systems(PreStartup, (setup, anim::setup_sheets));
    app.run();
}

/// Renderer choice. On the dev laptop (GTX 1660 Ti, Windows) DX12 drew Bevy 0.20's mesh-based sprites with each other's
/// transforms (title on the sign post, button labels on the frame pillars), while Vulkan renders correctly. Vulkan
/// itself failed with "Unable to find a GPU" because an implicit Vulkan layer (overlay/capture software) broke adapter
/// enumeration, so implicit layers are disabled for this process. WGPU_BACKEND still overrides.
fn render_settings() -> bevy::render::settings::WgpuSettings {
    #[allow(unused_mut)]
    let mut w = bevy::render::settings::WgpuSettings::default();
    // DX12 in this Bevy/wgpu combination draws sprites with each other's transforms (instance index mix-up), so
    // Windows always uses Vulkan, even if a WGPU_BACKEND (e.g. "dx12") is left over in the shell. HVSM_BACKEND=dx12
    // forces DX12 for diagnosis.
    #[cfg(windows)]
    {
        let want = std::env::var("HVSM_BACKEND").unwrap_or_default().to_lowercase();
        w.backends = Some(match want.as_str() {
            "dx12" => bevy::render::settings::Backends::DX12,
            "gl" => bevy::render::settings::Backends::GL,
            _ => bevy::render::settings::Backends::VULKAN,
        });
        // SAFETY: called from main() before any other thread exists.
        unsafe {
            std::env::remove_var("WGPU_BACKEND");
            if std::env::var("VK_LOADER_LAYERS_DISABLE").is_err() { std::env::set_var("VK_LOADER_LAYERS_DISABLE", "~implicit~"); }
        }
    }
    w
}

/// Starting weapon from HeroAttribute base_ware["weaponL"] (an Equipment index, e.g. Archer 63 = Longbow) into the main-hand slot 3.
pub fn starter_weapon(data: &data::GameData, oh: &mut save::OwnedHero) {
    if oh.equip[3].is_some() { return; }
    if let Some(&w) = data.hero(oh.hero).base_ware.get("weaponL") {
        if w >= 0 && data.equipment.items.iter().any(|i| i.index == w) { oh.equip[3] = Some(w); }
    }
}
pub fn new_game(data: &data::GameData) -> save::Save {
    let mut heroes = vec![];
    for h in [0, 4] { // GameData::ResetGameData: Fighter (0) and Cleric (4); the Archer comes from the first hero camp
        let root = data.hero(h).skill_tree.iter().find(|n| n.before < 0 && data.skill(n.skill).map(|s| s.kind == types::SkillKind::Active).unwrap_or(false)).map(|n| n.skill);
        let mut oh = save::Save::new_hero(h, root); starter_weapon(data, &mut oh); heroes.push(oh);
    }
    let mut s = save::Save { money_x4: 2000 * 4, heroes, party: vec![0, 1], stash: vec![None; save::STASH], shop: vec![None; save::SHOP], unlocked: 0, current_node: 0, sound: true, music: true, medals: 0, arena_score: 0, bag_tabs: 1, team_size: 3, tutorial_done: false, castles: vec![], pets: vec![], active_pet: None, badges: 0, stars: 0, battle_speed: 1, found: vec![], survival_best: 0 };
    store::refresh_shop(data, &mut s);
    s
}

fn setup(mut cmd: Commands, a: Res<AssetServer>) {
    cmd.spawn(Camera2d);
    // the original widescreen frame pillars (hvsMiphon5_left/right), shown on every screen
    for (n, x) in [("ui/hvsMiphon5_left.png", -524.0), ("ui/hvsMiphon5_right.png", 524.0)] {
        let mut s = Sprite::from_image(a.load(n));
        s.custom_size = Some(Vec2::new(88.0, 640.0));
        cmd.spawn((s, Transform::from_xyz(x, 0.0, 900.0)));
    }
    cmd.spawn((Text2d::new(""), TextFont { font: a.load("fonts/gilsanub.ttf").into(), font_size: bevy::text::FontSize::Px(24.0), ..default() },
        TextColor(Color::srgb(1.0, 0.95, 0.6)), Transform::from_xyz(0.0, -250.0, 950.0), Visibility::Hidden, ui::ToastText));
}

/// Keep the 1136×640 composition letterboxed in any window size.
fn fit_camera(windows: Query<&Window>, mut q: Query<&mut Projection, With<Camera2d>>) {
    let (Ok(w), Ok(mut p)) = (windows.single(), q.single_mut()) else { return };
    if let Projection::Orthographic(o) = &mut *p {
        let s = (ui::WINDOW.x / w.width()).max(ui::WINDOW.y / w.height());
        if (o.scale - s).abs() > 1e-3 { o.scale = s; }
    }
}

fn state_music(mut cmd: Commands, st: Res<State<AppState>>, a: Res<AssetServer>, save: Res<save::Save>, old: Query<Entity, With<audio::Music>>,
               mut cur: Local<String>) {
    let want = match st.get() {
        AppState::Menu => "mainmenu.ogg",
        AppState::Tavern | AppState::Hire | AppState::Store | AppState::Equip | AppState::Skills => "tavern.ogg",
        AppState::WorldMap => "story.ogg",
        AppState::Battle => "Music1.ogg",
        AppState::Options => "mainmenu.ogg",
        AppState::Arena | AppState::Siege | AppState::Summon => "story.ogg",
        AppState::Result => "",
        AppState::Slots => "mainmenu.ogg",
        AppState::Intro => "start.ogg",
    };
    let key = if save.music { want.to_string() } else { String::new() };
    if *cur == key || (want.is_empty() && st.is_changed()) { if want.is_empty() { for e in old.iter() { cmd.entity(e).despawn(); } cur.clear(); } return; }
    *cur = key;
    audio::music(&mut cmd, &a, &save, &old, want);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn d() -> data::GameData { data::GameData::load().unwrap() }
    #[test] fn learned_actives_and_upgrades() {
        let d = d();
        let mut oh = save::Save::new_hero(0, Some(2));
        oh.skills.insert(11, 1); oh.skills.insert(101, 1);
        let v = stats::learned_actives(&d, &oh);
        assert_eq!(v.iter().map(|(s, _)| s.name.as_str()).collect::<Vec<_>>(), ["Taunt", "Trip"]);
        // Bold Taunt hangs under Taunt: its timed damage cut is added to Taunt
        assert!(v[0].0.attrs.len() == 2);
        let mut ar = save::Save::new_hero(5, Some(8));
        ar.skills.insert(306, 1); // Great Rapid Shot +15% on Rapid Shot's 50%
        let v = stats::learned_actives(&d, &ar);
        assert_eq!(v[0].0.attrs[0].data, Some(65.0));
        let mut cl = save::Save::new_hero(4, Some(5));
        cl.skills.insert(211, 1); // Quick Healing Circle: -1 s cooldown
        let base = d.skill(5).unwrap().cd_ms.unwrap() as f32 / 1000.0;
        assert_eq!(stats::learned_actives(&d, &cl)[0].1, base - 1.0);
    }

    #[test] fn new_game_is_valid() {
        let data = d();
        let s = new_game(&data);
        assert_eq!(s.party.len(), 2); // Fighter + Cleric (ResetGameData)
        assert_eq!(s.money(), 2000);
        assert!(s.shop.iter().all(|i| i.map(|i| data.item(i).is_some()).unwrap_or(true)));
        for oh in &s.heroes { assert_eq!(stats::points_left(oh), 0, "root skill uses the level-1 point"); assert!(!oh.skills.is_empty()); }
    }

    #[test] fn siege_castles_are_valid() {
        let data = d();
        assert_eq!(data.siege.castles.len(), 13);
        for c in &data.siege.castles { assert!(data.enemies.enemies.iter().any(|e| e.index == c.boss), "boss {}", c.boss);
            assert!(data::asset_dir().join(format!("ui/{}.png", c.icon)).exists(), "{}", c.icon); }
    }

    #[test] fn world_nodes_have_waves() {
        let data = d();
        for n in &data.world.nodes { for m in &n.maps { assert!(data.waves.maps.contains_key(m), "node {} map {m}", n.index); } }
    }

    #[test] fn referenced_textures_exist() {
        let dir = data::asset_dir();
        for f in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src")).unwrap() {
            let src = std::fs::read_to_string(f.unwrap().path()).unwrap();
            for part in src.split('"').skip(1).step_by(2) {
                if (part.starts_with("ui/") || part.starts_with("maps/") || part.starts_with("fonts/")) && !part.contains('{') {
                    assert!(dir.join(part).exists(), "missing asset {part}");
                }
            }
        }
        for n in ["button_summon", "button_inn", "button_shop", "button_skill", "button_team", "button_stash"] {
            assert!(dir.join(format!("ui/{n}0.png")).exists() && dir.join(format!("ui/{n}1.png")).exists(), "{n}");
        }
        for k in ["helm", "armor", "handL", "handR", "ring", "boots"] { assert!(dir.join(format!("ui/itemshadow_{k}.png")).exists()); }
    }

    #[test] fn party_has_sheets_and_skills() {
        let data = d();
        for h in data::INN_HEROES { assert!(data.sheets.contains_key(&anim::hero_key(&data, h)), "sheet for hero {h}"); }
        let s = new_game(&data);
        for oh in &s.heroes {
            let tree = &data.hero(oh.hero).skill_tree;
            assert!(tree.iter().all(|n| (1..=4).contains(&n.row) && (1..=8).contains(&n.line)));
            let (f, _) = stats::hero_fighter(&data, oh);
            assert!(f.max_hp > 0.0);
        }
    }

    #[test] fn equip_changes_stats() {
        let data = d();
        let mut s = new_game(&data);
        let oh = &mut s.heroes[0];
        let h = data.hero(oh.hero);
        let it = data.equipment.items.iter().find(|e| e.slot == types::EquipSlot::Clothes && e.ac.unwrap_or(0) > 0 && stats::hero_can_use(h, e)).unwrap();
        let before = stats::hero_fighter(&data, oh).0.ac;
        oh.equip[1] = Some(it.index);
        assert!(stats::hero_fighter(&data, oh).0.ac > before);
    }

    #[test] fn save_roundtrip() {
        let data = d();
        let s = new_game(&data);
        let txt = ron::ser::to_string(&s).unwrap();
        let b: save::Save = ron::from_str(&txt).unwrap();
        assert_eq!(b.money_x4, s.money_x4);
    }
}
