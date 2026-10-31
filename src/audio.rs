//! Music per screen and one-shot SFX from the original Audio.xml set (assets/audio, ogg + wav).
use bevy::prelude::*;

#[derive(Component)]
pub struct Music;

/// Queue of SFX names (e.g. "sfx_chop1"), drained once per frame.
#[derive(Resource, Default)]
pub struct Sfx(pub Vec<&'static str>);
impl Sfx { pub fn play(&mut self, n: &'static str) { if self.0.len() < 8 { self.0.push(n); } } }

pub fn play_sfx(mut cmd: Commands, a: Res<AssetServer>, mut q: ResMut<Sfx>, save: Res<crate::save::Save>) {
    for n in q.0.drain(..) {
        if save.sound { cmd.spawn((AudioPlayer::new(a.load(format!("audio/{n}.wav"))), PlaybackSettings::DESPAWN)); }
    }
}

pub fn music(cmd: &mut Commands, a: &AssetServer, save: &crate::save::Save, old: &Query<Entity, With<Music>>, file: &str) {
    for e in old.iter() { cmd.entity(e).despawn(); }
    if save.music { cmd.spawn((AudioPlayer::new(a.load(format!("audio/{file}"))), PlaybackSettings::LOOP, Music)); }
}
