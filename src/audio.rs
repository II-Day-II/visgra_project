use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, Sample};
use crossbeam_channel::{Receiver, Sender};
use std::thread;

pub enum ToAudio {
    Pause,
    Play,
    ToggleMic,
    Volume(bool)
}

pub enum FromAudio {
    Data(f32),
}

struct AudioData {
    
}

impl AudioData {
    fn new() -> Self {
        Self {  }
    }
}

pub fn audio_thread() -> (Sender<ToAudio>, Receiver<FromAudio>, thread::JoinHandle<()>) {
    let (tx_in, rx_in) = crossbeam_channel::bounded(1024);
    let (tx_out, rx_out) = crossbeam_channel::bounded(1024);
    
    let handle = thread::spawn(move || {
        let host = cpal::default_host();
        let in_device = host.default_input_device().expect("get input device");
        let out_device = host.default_output_device().expect("get output device");
        let cfg_out = out_device.default_output_config().expect("get output config");  
        let cfg_in = in_device.default_input_config().expect("get output config");  
        match cfg_out.sample_format() {
            cpal::SampleFormat::I16 => {
                audio_handler::<i16>(&in_device, &out_device, &cfg_in.into(), &cfg_out.into(), rx_in.clone(), tx_out.clone()).expect("run i16");
            },
            cpal::SampleFormat::U16 => {
                audio_handler::<u16>(&in_device, &out_device, &cfg_in.into(), &cfg_out.into(), rx_in.clone(), tx_out.clone()).expect("run u16");
            },
            cpal::SampleFormat::F32 => {
                audio_handler::<f32>(&in_device, &out_device, &cfg_in.into(), &cfg_out.into(), rx_in.clone(), tx_out.clone()).expect("run f32");
            },
        }
    });

    (tx_in, rx_out, handle)
}

fn audio_handler<T: Sample>(device_in: &cpal::Device, device_out: &cpal::Device, cfg_in: &cpal::StreamConfig, cfg_out: &cpal::StreamConfig, rx: Receiver<ToAudio>, tx: Sender<FromAudio>) -> anyhow::Result<()> {
    let sample_rate = cfg_out.sample_rate.0 as f32;
    let channels = cfg_out.channels as usize;
    let err_fn = |err| {eprintln!("Error on audio stream: {}", err)};
    
    // get audio data here
    let mut audio_data = AudioData::new();

    let stream_in = device_in.build_input_stream(
        cfg_in,
        move |data: &[T], _: &_| {},
        err_fn,
    )?;

    let stream_out = device_out.build_output_stream(
        cfg_out,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(channels) {
                let value: T = cpal::Sample::from(&todo!());

                for sample in frame.iter_mut() {
                    *sample = value;
                }
            }
        },
        err_fn
    )?;

    stream_out.play()?;
    stream_in.play()?;
    std::thread::park(); // leaves the thread to run until app ends

    Ok(())
}
