//! Music per screen and one-shot SFX from the original Audio.xml set (assets/audio, ogg + wav).
//! When the game window is unfocused (PC) or the app is suspended (Android), every sink is muted
//! and new SFX are dropped; audio resumes on focus / resume.
use bevy::audio::AudioSinkPlayback;
use bevy::prelude::*;
use bevy::window::{AppLifecycle, PrimaryWindow, WindowFocused};

#[derive(Component)]
pub struct Music;

/// Queue of SFX names (e.g. "sfx_chop1"), drained once per frame.
#[derive(Resource, Default)]
pub struct Sfx(pub Vec<&'static str>);
impl Sfx { pub fn play(&mut self, n: &'static str) { if self.0.len() < 8 { self.0.push(n); } } }

/// True while the window has focus / the Android app is in the foreground. Own design.
#[derive(Resource)]
pub struct AudioFocus(pub bool);

pub fn play_sfx(mut cmd: Commands, a: Res<AssetServer>, mut q: ResMut<Sfx>, save: Res<crate::save::Save>, focus: Res<AudioFocus>) {
    if !focus.0 { q.0.clear(); return; }
    for n in q.0.drain(..) {
        if save.sound { cmd.spawn((AudioPlayer::new(a.load(format!("audio/{n}.wav"))), PlaybackSettings::DESPAWN)); }
    }
}

pub fn music(cmd: &mut Commands, a: &AssetServer, save: &crate::save::Save, old: &Query<Entity, With<Music>>, file: &str) {
    for e in old.iter() { cmd.entity(e).despawn(); }
    if save.music { cmd.spawn((AudioPlayer::new(a.load(format!("audio/{file}"))), PlaybackSettings::LOOP, Music)); }
}

/// Track focus / Android lifecycle, then mute or unmute every live sink to match.
/// Polls `Window.focused` every frame (more reliable than events alone on Windows) and also
/// listens to WindowFocused + AppLifecycle (Android suspend/resume).
pub fn focus_audio(
    mut focus: ResMut<AudioFocus>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut focused: MessageReader<WindowFocused>,
    mut life: MessageReader<AppLifecycle>,
    mut sinks: Query<&mut AudioSink>,
) {
    let mut want = focus.0;
    for e in focused.read() { want = e.focused; }
    for e in life.read() {
        want = match e {
            AppLifecycle::WillSuspend | AppLifecycle::Suspended | AppLifecycle::Idle => false,
            AppLifecycle::Running | AppLifecycle::WillResume => true,
        };
    }
    // Prefer the live window flag when we have a primary window (PC blur / Alt-Tab).
    if let Ok(w) = windows.single() {
        // On Android the window often stays "focused" while suspended; lifecycle above wins then.
        if !cfg!(target_os = "android") { want = w.focused; }
    }
    let changed = want != focus.0;
    focus.0 = want;
    for mut s in &mut sinks {
        if focus.0 {
            if s.is_muted() { s.unmute(); }
        } else if !s.is_muted() {
            s.mute();
        }
    }
    if changed {
        eprintln!("HVSM_AUDIO_FOCUS {}", if focus.0 { "on" } else { "off" });
    }
}
