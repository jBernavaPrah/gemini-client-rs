use audio_gate::NoiseGate;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::ops::Deref;
use std::pin::Pin;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tracing::info;

/// A boxed [`tokio_stream::Stream`].
pub type Stream<T> = Pin<Box<dyn tokio_stream::Stream<Item = T> + Send>>;

/// Wrapper around [`cpal::Stream`] so dropping `Input` stops the capture.
pub struct Input {
    stream: cpal::Stream,
}

impl Deref for Input {
    type Target = cpal::Stream;
    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}

#[derive(Clone, Copy, Debug)]
pub struct OutputAudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub batch_size: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum AudioFormat {
    #[default]
    MuLaw8Khz8BitMono,
    ALaw8Khz8BitMono,
    Pcm24Khz16BitMono,
}

impl AudioFormat {
    pub fn as_header(&self) -> Vec<u8> {
        match self {
            AudioFormat::Pcm24Khz16BitMono => hound::WavSpec {
                channels: 1,
                sample_rate: 24_000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            }
            .into_header_for_infinite_file(),
            AudioFormat::MuLaw8Khz8BitMono => {
                let mut header = hound::WavSpec {
                    channels: 1,
                    sample_rate: 8_000,
                    bits_per_sample: 8,
                    sample_format: hound::SampleFormat::Int,
                }
                .into_header_for_infinite_file();
                header[20] = 0x07;
                header[21] = 0x00;
                header
            }
            AudioFormat::ALaw8Khz8BitMono => {
                let mut header = hound::WavSpec {
                    channels: 1,
                    sample_rate: 8_000,
                    bits_per_sample: 8,
                    sample_format: hound::SampleFormat::Int,
                }
                .into_header_for_infinite_file();
                header[20] = 0x06;
                header[21] = 0x00;
                header
            }
        }
    }
}

/// Returns a sender that accepts WAV encoded chunks and plays them on the default device.
pub fn sender_for_default_audio_output(format: AudioFormat) -> std::sync::mpsc::Sender<Vec<u8>> {
    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
    tx.send(format.as_header()).expect("header");

    std::thread::spawn(move || {
        let (_stream, handle) = rodio::OutputStream::try_default().expect("output");
        let sink = rodio::Sink::try_new(&handle).expect("sink");
        let source = StreamMediaSource::new(rx);
        let decoder = rodio::Decoder::new_wav(source).expect("decoder");
        sink.append(decoder);
        sink.sleep_until_end();
    });

    tx
}

pub struct StreamMediaSource {
    inner: std::sync::Mutex<std::sync::mpsc::Receiver<Vec<u8>>>,
    buffer: Vec<u8>,
}

impl StreamMediaSource {
    pub fn new(inner: std::sync::mpsc::Receiver<Vec<u8>>) -> Self {
        Self { inner: std::sync::Mutex::new(inner), buffer: Vec::with_capacity(1024) }
    }

    fn read_inner(&mut self, len: usize) -> Vec<u8> {
        while self.buffer.len() < len {
            let result = {
                let rx = self.inner.lock().unwrap();
                rx.recv_timeout(std::time::Duration::from_millis(10))
            };
            match result {
                Ok(data) => self.buffer.extend(data),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if !self.buffer.is_empty() {
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        let read_len = std::cmp::min(len, self.buffer.len());
        self.buffer.drain(..read_len).collect()
    }
}

impl std::io::Read for StreamMediaSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let data = self.read_inner(buf.len());
        let len = std::cmp::min(buf.len(), data.len());
        buf[..len].copy_from_slice(&data[..len]);
        Ok(len)
    }
}

impl std::io::Seek for StreamMediaSource {
    fn seek(&mut self, _pos: std::io::SeekFrom) -> std::io::Result<u64> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "StreamMediaSource does not support seeking",
        ))
    }
}

/// Creates a stream from the default input device returning audio as [`Stream`] of chunks.
#[tracing::instrument(skip(output), fields(sample_rate = output.sample_rate, channels = output.channels, bits_per_sample = output.bits_per_sample, batch_size = output.batch_size))]
pub async fn listen_from_default_input(output: OutputAudioConfig) -> Result<(Stream<Vec<u8>>, Input), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or("no input device")?;
    let device_config = device.default_input_config()?;
    let config: cpal::StreamConfig = device_config.clone().into();

    info!("Using default input device {:?}", device.name());
    info!("Device config: {:?}", device_config);

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    let err = |err| tracing::error!("Trying to stream input: {err}");

    let mut gate = NoiseGate::new(
        -36.0,
        -54.0,
        device_config.sample_rate().0 as f32,
        device_config.channels() as usize,
        150.0,
        25.0,
        150.0,
    );
    let mut buffer = Vec::with_capacity(output.batch_size);
    let stream = match device_config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _| {
                let mut processed = gate.process_frame(data);
                if device_config.channels() != output.channels {
                    processed = convert_channels(
                        &processed,
                        device_config.channels() as usize,
                        output.channels as usize,
                    );
                }
                if device_config.sample_rate().0 != output.sample_rate {
                    processed = resample_linear(
                        &processed,
                        device_config.sample_rate().0,
                        output.sample_rate,
                        output.channels as usize,
                    );
                }
                for sample in processed {
                    buffer.extend(sample_to_bytes(sample, output.bits_per_sample));
                    while buffer.len() >= output.batch_size {
                        let chunk: Vec<u8> = buffer.drain(..output.batch_size).collect();
                        let _ = tx.send(chunk);
                    }
                }
            },
            err,
            None,
        ),
        _ => panic!("Unsupported sample format"),
    }?;

    stream.play()?;

    let input = Input { stream };
    let stream = Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(rx));

    Ok((stream, input))
}

pub fn stdin_stream() -> Pin<Box<dyn tokio_stream::Stream<Item = String> + Send>> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    tokio::spawn(async move {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(rx))
}

fn convert_channels(data: &[f32], in_ch: usize, out_ch: usize) -> Vec<f32> {
    if in_ch == out_ch {
        return data.to_vec();
    }
    let frames = data.len() / in_ch;
    let mut out = Vec::with_capacity(frames * out_ch);
    for frame in 0..frames {
        for oc in 0..out_ch {
            if out_ch == 1 {
                let mut acc = 0f32;
                for ic in 0..in_ch {
                    acc += data[frame * in_ch + ic];
                }
                out.push(acc / in_ch as f32);
            } else if in_ch == 1 {
                out.push(data[frame]);
            } else {
                out.push(data[frame * in_ch + oc.min(in_ch - 1)]);
            }
        }
    }
    out
}

fn resample_linear(data: &[f32], in_rate: u32, out_rate: u32, channels: usize) -> Vec<f32> {
    if in_rate == out_rate {
        return data.to_vec();
    }
    let in_frames = data.len() / channels;
    let ratio = out_rate as f64 / in_rate as f64;
    let out_frames = ((in_frames as f64) * ratio).ceil() as usize;
    let mut out = Vec::with_capacity(out_frames * channels);
    for out_idx in 0..out_frames {
        let src_pos = out_idx as f64 / ratio;
        let src_idx = src_pos.floor() as usize;
        let frac = src_pos - src_idx as f64;
        for ch in 0..channels {
            let i0 = std::cmp::min(src_idx, in_frames - 1) * channels + ch;
            let i1 = std::cmp::min(src_idx + 1, in_frames - 1) * channels + ch;
            let s0 = data[i0];
            let s1 = data[i1];
            out.push(((1.0 - frac) as f32) * s0 + (frac as f32) * s1);
        }
    }
    out
}

fn sample_to_bytes(sample: f32, bits: u16) -> Vec<u8> {
    let clamped = sample.clamp(-1.0, 1.0);
    match bits {
        8 => vec![((clamped * i8::MAX as f32) as i8) as u8],
        16 => ((clamped * i16::MAX as f32) as i16).to_le_bytes().to_vec(),
        24 => {
            let val = (clamped * 8_388_607.0) as i32;
            vec![(val & 0xff) as u8, ((val >> 8) & 0xff) as u8, ((val >> 16) & 0xff) as u8]
        }
        32 => clamped.to_le_bytes().to_vec(),
        _ => clamped.to_le_bytes().to_vec(),
    }
}
