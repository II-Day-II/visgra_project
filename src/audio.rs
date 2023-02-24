use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, Sample};
use crossbeam_channel::{Receiver, Sender};
use std::{thread, sync::Arc};
use ringbuf::{HeapRb, Consumer, Rb};
use hound;

const LATENCY: f32 = 150.0;


#[derive(Debug)]
pub enum ToAudio {
    ToggleMic,
    ToggleVisuals,
}

pub enum FromAudio {
    Data(f32),
}

struct AudioData {
    cons: Consumer<ToAudio, Arc<HeapRb<ToAudio>>>,
    send_to_gfx: bool,
    use_mic: bool,
    wav_src: hound::WavIntoSamples<std::io::BufReader<std::fs::File>,f32>,
}

impl AudioData {
    fn new(cons: Consumer<ToAudio, Arc<HeapRb<ToAudio>>>) -> Self {
        // find first wav file in ./music
        let fname = std::fs::read_dir("./music").expect("no music dir")
            .filter_map(Result::ok)
            .filter(|x| x.path().to_string_lossy().ends_with("wav"))
            .next().expect("find a wav file")
            .path();
        let reader = hound::WavReader::open(fname).expect("open wav file");
        let spec = reader.spec();
        eprintln!("{:?}", spec);
        Self { 
            cons,
            send_to_gfx: true,
            use_mic: false,
            wav_src: reader.into_samples(),
        }
    }

    fn handle_commands(&mut self) {
        while let Some(cmd) = self.cons.pop() {
            match cmd {
                ToAudio::ToggleVisuals => {
                    self.send_to_gfx = !self.send_to_gfx;
                },
                ToAudio::ToggleMic => {
                    self.use_mic = !self.use_mic;
                }
            }
        }
    }

    fn next_sample(&mut self) -> f32 {
        if let Some(sample) = self.wav_src.next() {
            sample.unwrap_or(0.0)
        } else {
            0.0
        }
    }
}

pub fn audio_thread() -> (Sender<ToAudio>, Receiver<FromAudio>) {
    let (tx_in, rx_in) = crossbeam_channel::bounded(1024);
    let (tx_out, rx_out) = crossbeam_channel::bounded(1024);
    let _handle = thread::spawn(move || {
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

    (tx_in, rx_out)
}

fn audio_handler<T: Sample>(device_in: &cpal::Device, device_out: &cpal::Device, cfg_in: &cpal::StreamConfig, cfg_out: &cpal::StreamConfig, rx: Receiver<ToAudio>, tx: Sender<FromAudio>) -> anyhow::Result<()> {
    let sample_rate = cfg_out.sample_rate.0 as f32;
    let channels = cfg_out.channels as usize;
    let err_fn = |err| {eprintln!("Error on audio stream: {}", err)};
    
    // get audio data here
    let audio_rb = HeapRb::new(1024);
    let (mut audio_prod, audio_cons) = audio_rb.split();
    let mut audio_data = AudioData::new(audio_cons);
    
    let latency_frames = (LATENCY / 1000.0) * sample_rate;
    let latency_samples = latency_frames as usize * channels;

    let mic_buf = HeapRb::new(latency_samples * 2);
    let (mut prod, mut cons) = mic_buf.split();

    for _ in 0..latency_samples {
        // add latency to mic echo
        prod.push(0.0).unwrap()
    }

    // duplicate it to keep track of if mic should do things in both threads
    let mut dupe_use_mic = audio_data.use_mic;

    let stream_in = device_in.build_input_stream(
        cfg_in,
        move |data: &[T], _: &_| {
            let mut need_more_latency = false;
            while let Ok(cmd) = rx.try_recv() {
                if let ToAudio::ToggleMic = cmd {
                    dupe_use_mic = !dupe_use_mic;
                }
                audio_prod.push(cmd).expect("audio commands are handled");
            }
            for &sample in data {
                if dupe_use_mic {
                    if prod.push(sample.to_f32()).is_err() {
                        need_more_latency = true;
                    }
                }
            }
            if need_more_latency {
                eprintln!("output is falling behind, need more latency!");
            }
        },
        err_fn,
    )?;

    let stream_out = device_out.build_output_stream(
        cfg_out,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut need_more_latency = false;
            audio_data.handle_commands();
            for frame in data.chunks_mut(channels) {
                for sample in frame.iter_mut() {
                    let tmp = if audio_data.use_mic {
                        match cons.pop() {
                            Some(s) => s,
                            None => {
                                need_more_latency = true;
                                0.0
                            }
                        }
                    } else {
                        audio_data.next_sample()
                    };
                    if audio_data.send_to_gfx { // send it to the graphics part
                        tx.send(FromAudio::Data(tmp)).expect("send response to gfx thread");
                    }
                    *sample = cpal::Sample::from(&tmp);
                }
            }
            if need_more_latency {
                eprintln!("input is falling behind, need more latency!");
            }
        },
        err_fn
    )?;

    stream_out.play()?;
    stream_in.play()?;
    std::thread::park(); // leaves the thread to run until app ends

    Ok(())
}
