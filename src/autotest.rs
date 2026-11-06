//! Scripted self-test, active only when HVSM_AUTOTEST is set. Lets the game be driven and screenshotted without touching
//! the real mouse/keyboard. Steps separated by ';':
//!   w<secs>       wait            c<fx>,<fy>   click at a fraction of the 1136×640 window (0..1, y down)
//!   k<Letter>     tap a key       s<name>      save a screenshot to $HVSM_SHOTS/<name>.png (default ./shots)
//!   m<fx>,<fy>   move the cursor only   d<fx>,<fy>   press and hold   u   release
//!   t             click the nearest enemy (battle)   y   click hero slot 0 (battle)
//!   Y             press and hold on hero slot 0   T   move (held) onto the nearest enemy
//!   q             quit
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

pub fn plugin(app: &mut App) {
    let Ok(script) = std::env::var("HVSM_AUTOTEST") else { return };
    let steps: Vec<String> = script.split(';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    app.insert_resource(Auto { steps, i: 0, wait: 1.0, release: None })
        .add_systems(PreUpdate, drive.after(crate::ui::track_cursor).before(crate::ui::click_system));
}

#[derive(Resource)]
struct Auto { steps: Vec<String>, i: usize, wait: f32, release: Option<Release> }
enum Release { Mouse, Key(KeyCode) }

fn key(c: char) -> Option<KeyCode> {
    Some(match c.to_ascii_uppercase() {
        'A' => KeyCode::KeyA, 'Q' => KeyCode::KeyQ, 'W' => KeyCode::KeyW, 'E' => KeyCode::KeyE, 'P' => KeyCode::KeyP,
        'M' => KeyCode::KeyM, 'X' => KeyCode::Escape, 'F' => KeyCode::KeyF, '1' => KeyCode::Digit1, '2' => KeyCode::Digit2, '3' => KeyCode::Digit3, _ => return None,
    })
}

fn drive(mut cmd: Commands, mut st: ResMut<Auto>, time: Res<Time<Real>>, mut cur: ResMut<crate::ui::Cursor>,
         mut mouse: ResMut<ButtonInput<MouseButton>>, mut keys: ResMut<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>,
         mut held: Local<Option<Vec2>>, aim: Option<Res<crate::battle::AutoAim>>, mut next: ResMut<NextState<crate::AppState>>, mut save: ResMut<crate::save::Save>, mut setup: ResMut<crate::battle::BattleSetup>, mut run: Option<ResMut<crate::battle::Run>>) {
    let ally_pos = aim.as_ref().and_then(|a| a.1);
    let mut aim_pos = aim.and_then(|a| a.0);
    if let Some(p) = *held { cur.world = Some(p); }
    match st.release.take() {
        Some(Release::Mouse) => { mouse.release(MouseButton::Left); return; }
        Some(Release::Key(k)) => { keys.release(k); return; }
        None => {}
    }
    st.wait -= time.delta_secs();
    if st.wait > 0.0 { return; }
    let Some(step) = st.steps.get(st.i).cloned() else { exit.write(AppExit::Success); return };
    st.i += 1;
    st.wait = 0.3;
    let (op, arg) = step.split_at(1);
    let arg = arg.trim();
    match op {
        "w" => st.wait = arg.parse().unwrap_or(1.0),
        // Bmap,level: start a battle on that map (test only, needs a throwaway HVSM_SAVE); V: start Survival
        "B" | "V" => if std::env::var("HVSM_SAVE").is_ok() {
            let v: Vec<i32> = arg.split(',').filter_map(|x| x.parse().ok()).collect();
            let map = v.first().copied().unwrap_or(0) as u32;
            let lvl = v.get(1).copied().unwrap_or(1);
            let node = v.get(2).copied().map(|n| n as u32).unwrap_or(1); // default area-1 for FallDown tests
            let boss = v.get(3).copied().unwrap_or(0) != 0;
            *setup = crate::battle::BattleSetup { map, level: (lvl, lvl), survival: op == "V", node, boss, ..Default::default() };
            if op == "V" { setup.level = (0, 0); }
            next.set(crate::AppState::Battle); st.wait = 2.0;
        },
        "c" => {
            let v: Vec<f32> = arg.split(',').filter_map(|x| x.parse().ok()).collect();
            if v.len() == 2 {
                let p = Vec2::new((v[0] - 0.5) * crate::ui::WINDOW.x, (0.5 - v[1]) * crate::ui::WINDOW.y);
                *held = Some(p); cur.world = Some(p);
                mouse.press(MouseButton::Left);
                st.release = Some(Release::Mouse);
                st.wait = 1.0;
            }
        }
        "m" | "d" => {
            let v: Vec<f32> = arg.split(',').filter_map(|x| x.parse().ok()).collect();
            if v.len() == 2 {
                let p = Vec2::new((v[0] - 0.5) * crate::ui::WINDOW.x, (0.5 - v[1]) * crate::ui::WINDOW.y);
                *held = Some(p); cur.world = Some(p);
                if op == "d" { mouse.press(MouseButton::Left); }
            }
        }
        "t" => if let Some(p) = std::mem::take(&mut aim_pos) {
            *held = Some(p); cur.world = Some(p); mouse.press(MouseButton::Left); st.release = Some(Release::Mouse); st.wait = 1.0;
        },
        "y" => if let Some(p) = ally_pos {
            *held = Some(p); cur.world = Some(p); mouse.press(MouseButton::Left); st.release = Some(Release::Mouse); st.wait = 1.0;
        },
        "Y" => if let Some(p) = ally_pos { *held = Some(p); cur.world = Some(p); mouse.press(MouseButton::Left); },
        "H" => if let Some(p) = ally_pos { *held = Some(p); cur.world = Some(p); },
        "T" => if let Some(p) = aim_pos { *held = Some(p); cur.world = Some(p); },
        "u" => { mouse.release(MouseButton::Left); }
        "k" => if let Some(k) = arg.chars().next().and_then(key) { keys.press(k); st.release = Some(Release::Key(k)); st.wait = 1.0; },
        "s" => {
            let dir = std::env::var("HVSM_SHOTS").unwrap_or_else(|_| "shots".into());
            let _ = std::fs::create_dir_all(&dir);
            let path = std::path::Path::new(&dir).join(format!("{}.png", if arg.is_empty() { format!("shot{}", st.i) } else { arg.to_string() }));
            cmd.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
            st.wait = 0.5;
        }
        "Z" => if std::env::var("HVSM_SAVE").is_ok() {
            if let Some(ref mut run) = run { run.won = true; run.done = false; }
            st.wait = 2.0;
        },
        "q" => { exit.write(AppExit::Success); }
        // gStore / gHire ... jump straight to a screen; $N gives N gold (only with a throwaway HVSM_SAVE)
        "g" => { use crate::AppState::*; let s = match arg { "Store" => Store, "Hire" => Hire, "Equip" => Equip, "Skills" => Skills, "Tavern" => Tavern, "WorldMap" => WorldMap, "Arena" => Arena, "Options" => Options, "Siege" => Siege, "Summon" => Summon, "Slots" => Slots, "Intro" => Intro, _ => Menu }; next.set(s); st.wait = 1.5; }
        "$" => if std::env::var("HVSM_SAVE").is_ok() { save.add_money(arg.parse().unwrap_or(0)); },
        _ => {}
    }
}
