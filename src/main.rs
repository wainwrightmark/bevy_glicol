use std::sync::{Arc, Mutex};

use bevy::{
    audio::{play_queued_audio_system, AddAudioSource},
    log,
    prelude::*,
    reflect::TypeUuid,
};

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::{egui, EguiContext, EguiPlugin};

use glicol::Engine;
use glicol_synth::{ Buffer};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .init_resource::<Song>()
        .init_resource::<GlicolAudioSource>()
        .add_audio_source::<GlicolAudioSource>()
        .add_system(ui_system)
        .add_system(song_update_system.after(ui_system).before(play_queued_audio_system::<GlicolAudioSource>))
        .add_startup_system(setup)
        .run();
}

#[derive(Resource)]
struct Song {
    code: String,
    updated: bool
}

impl Default for Song{
    fn default() -> Self {
        Self { code: ONTHERUN.to_string(), updated: false }
    }
}

fn song_update_system(mut song: ResMut<Song>, source: ResMut<GlicolAudioSource>){
    if song.updated{

        let mut engine= source.engine.lock().unwrap();
        engine.update_with_code(&song.code);
        let update_result = engine.update();
        match update_result {
            Ok(_) => {},
            Err(err) => {
                log::error!("{err:?}");
            },
        }
        song.updated = false;
    }
}

fn ui_system(mut egui_ctx: Query<&mut EguiContext, With<PrimaryWindow>>,mut song: ResMut<Song>) {
    egui::Window::new("glicol").show(egui_ctx.single_mut().get_mut(), |ui| {

        let response = ui.text_edit_multiline(&mut song.code);
        song.updated = response.changed();

    });
}

fn setup(mut assets: ResMut<Assets<GlicolAudioSource>>, audio: Res<Audio<GlicolAudioSource>>, source: Res<GlicolAudioSource>) {
    // add a `SineAudio` to the asset server so that it can be played
    let audio_handle = assets.add(source.clone());
    audio.play(audio_handle);
}

#[derive(TypeUuid, Clone, Resource)]
#[uuid = "96fff988-bcfe-11ed-afa1-0242ac120002"]
struct GlicolAudioSource {
    engine: Arc<Mutex<Engine<128>>>,
}





impl Default for GlicolAudioSource {
    fn default() -> Self {
        let mut engine = Engine::<128>::new();
        engine.set_bpm(66.);
        engine.update_with_code(ONTHERUN);
        engine.update().unwrap();

        Self {
            engine: Arc::new(engine.into()),
        }
    }
}

struct GlicolDecoder {
    source: GlicolAudioSource,
    buffers: Vec<Buffer<128>>,
    channel: u16,
    index: usize,
}

impl GlicolDecoder {
    pub fn new(source: GlicolAudioSource) -> Self {
        let mut s = Self {
            source,
            buffers: vec![],
            channel: 0,
            index: 128,
        };
        Self::refresh_buffers(&mut s);
        s
    }

    pub fn refresh_buffers(&mut self) {
        self.channel = 0;
        self.index = 0;
        self.buffers.clear();
        let mut engine = self.source.engine.lock().unwrap();
        let context = &mut engine.context;
        let block = context.next_block();
        self.buffers.extend_from_slice(block);
    }
}

impl bevy::audio::Source for GlicolDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        Some(128 - self.index)
    }

    fn channels(&self) -> u16 {
        let result = self.buffers.len() as u16;
        //log::info!("Channels: {result}");
        result
    }

    fn sample_rate(&self) -> u32 {
        44100
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

impl Iterator for GlicolDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let buffer: &Buffer<128> = if let Some(buffer) = self.buffers.get(self.channel as usize) {
            buffer
        } else {
            self.channel = 0;
            self.index += 1;
            if self.index >= 128 {
                self.refresh_buffers();
            }
            if let Some(buffer) = self.buffers.get(0) {
                buffer
            } else {
                return None;
            }
        };
        self.channel += 1;
        let item = buffer[self.index];

        Some(item)
    }
}

impl Decodable for GlicolAudioSource {
    type DecoderItem = f32;

    type Decoder = GlicolDecoder;

    fn decoder(&self) -> Self::Decoder {
        GlicolDecoder::new(self.clone())
    }
}


const ONTHERUN: &'static str = r#"
// ##setBPM(66)#

~bd: speed 4.0 >> seq 60 >> bd 0.03

~sn: speed 4.0 >> seq _ 60 >> sn 0.05 >> mul 0.5

~hh: speed 16.0 >> seq 60 >> hh 0.03

~basslow: speed 1.0
>> seq 33 _33 36_33_ 36_33_ 28 _28 31_28_ 31
>> sawsynth 0.01 0.2 >> lpf 500 1.0

~lead: seq 64_60_ 57_60_ 64_60_ 64_65_ 64_59_ 55_59_ 64 62
>> squsynth 0.01 0.2 >> lpf 800 1.0 >> mul 0.0
// change the mul 0.0 to 0.7

~bassmid: speed 1.0
>> seq 45 _45 48_45_ 48_45_ 40 _40 43_40_ 43
>> mul 0.99 >> sawsynth 0.01 0.21 >> lpf 1000 1.0

out: mix ~bd ~sn ~hh ~lead ~basslow ~bassmid >> plate 0.1
"#;