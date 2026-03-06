use bytes::Bytes;
use cpal::{
    BufferSize, InputCallbackInfo, Sample, SampleFormat, SizedSample, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use futures_util::future;
use num_traits::cast::ToPrimitive;
use std::{
    error::Error,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::mpsc;
use webrtc::{
    api::media_engine::MIME_TYPE_PCMU, media::Sample as WebRtcSample,
    rtp_transceiver::rtp_codec::RTCRtpCodecCapability,
    track::track_local::track_local_static_sample::TrackLocalStaticSample,
};

const TARGET_SAMPLE_RATE: u32 = 8000;
const PACKET_SAMPLES: usize = 160;

pub fn build_pcmu_track() -> Arc<TrackLocalStaticSample> {
    let codec_capability = RTCRtpCodecCapability {
        mime_type: MIME_TYPE_PCMU.to_string(),
        clock_rate: 8000,
        channels: 1,
        ..Default::default()
    };
    Arc::new(TrackLocalStaticSample::new(
        codec_capability,
        "pc_audio".to_string(),
        "portapad".to_string(),
    ))
}

pub fn start_system_audio_capture(
    track: Arc<TrackLocalStaticSample>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("出力デバイスが見つかりません")?;
    let supported_config = device.default_output_config()?;
    let mut stream_config: StreamConfig = supported_config.clone().into();
    stream_config.buffer_size = BufferSize::Default;

    let ratio = stream_config.sample_rate as f32 / TARGET_SAMPLE_RATE as f32;
    let step = ratio.max(1.0).round() as usize;
    let step = step.max(1);
    let channels = stream_config.channels as usize;
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(64);
    let track_for_writer: Arc<TrackLocalStaticSample> = Arc::clone(&track);

    tokio::spawn(async move {
        while let Some(packet) = rx.recv().await {
            let duration = Duration::from_micros(
                (packet.len() as u64 * 1_000_000) / TARGET_SAMPLE_RATE as u64,
            );
            let sample = WebRtcSample {
                data: Bytes::from(packet),
                duration,
                timestamp: SystemTime::now(),
                packet_timestamp: 0,
                prev_dropped_packets: 0,
                prev_padding_packets: 0,
            };
            if let Err(err) = track_for_writer.write_sample(&sample).await {
                eprintln!("Audio track write failed: {:?}", err);
            }
        }
    });

    let stream = match supported_config.sample_format() {
        SampleFormat::F32 => build_raw_stream::<f32>(
            &device,
            &stream_config,
            SampleFormat::F32,
            tx.clone(),
            step,
            channels,
        )?,
        SampleFormat::I16 => build_raw_stream::<i16>(
            &device,
            &stream_config,
            SampleFormat::I16,
            tx.clone(),
            step,
            channels,
        )?,
        SampleFormat::U16 => build_raw_stream::<u16>(
            &device,
            &stream_config,
            SampleFormat::U16,
            tx.clone(),
            step,
            channels,
        )?,
        _ => return Err(Box::new(cpal::BuildStreamError::StreamConfigNotSupported)),
    };

    stream.play()?;
    tokio::spawn(async move {
        let _stream = stream;
        future::pending::<()>().await;
    });

    Ok(())
}

fn build_raw_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    sender: mpsc::Sender<Vec<u8>>,
    step: usize,
    channels: usize,
) -> Result<Stream, cpal::BuildStreamError>
where
    T: Sample + SizedSample + Send + 'static + ToPrimitive,
{
    let err_fn = |err| eprintln!("Audio capture error: {:?}", err);
    let mut packet = Vec::with_capacity(PACKET_SAMPLES);
    let mut sample_counter = 0usize;

    device.build_input_stream_raw(
        config,
        sample_format,
        move |data: &cpal::Data, _: &InputCallbackInfo| {
            if let Some(samples) = data.as_slice::<T>() {
                process_samples(
                    samples,
                    channels,
                    step,
                    &sender,
                    &mut packet,
                    &mut sample_counter,
                );
            }
        },
        err_fn,
        None,
    )
}

fn process_samples<T>(
    samples: &[T],
    channels: usize,
    step: usize,
    sender: &mpsc::Sender<Vec<u8>>,
    packet: &mut Vec<u8>,
    sample_counter: &mut usize,
) where
    T: Sample + ToPrimitive,
{
    let mut iterator = samples.chunks(channels);
    while let Some(frame) = iterator.next() {
        if frame.len() < channels {
            break;
        }
        *sample_counter = sample_counter.wrapping_add(1);
        if step > 1 && (*sample_counter % step != 0) {
            continue;
        }
        let sum: f32 = frame
            .iter()
            .map(|s| <T as ToPrimitive>::to_f32(s).unwrap_or(0.0))
            .sum();
        let average = sum / channels as f32;
        let sample_i16 = (average * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        packet.push(pcm_to_ulaw(sample_i16));
        if packet.len() >= PACKET_SAMPLES {
            let ready_packet = std::mem::take(packet);
            if sender.try_send(ready_packet).is_err() {
                eprintln!("Audio packet dropped");
            }
            packet.reserve(PACKET_SAMPLES);
        }
    }
}

fn pcm_to_ulaw(mut sample: i16) -> u8 {
    const BIAS: i16 = 0x84;
    const CLIP: i16 = 32635;
    let sign = if sample < 0 {
        sample = -sample;
        0x80
    } else {
        0
    };
    if sample > CLIP {
        sample = CLIP;
    }
    sample += BIAS;
    let mut exponent = 7;
    let mut mask = 0x4000;
    while exponent > 0 && (sample & mask) == 0 {
        mask >>= 1;
        exponent -= 1;
    }
    let mantissa = ((sample >> (exponent + 3)) & 0x0F) as u8;
    !(sign | ((exponent << 4) as u8) | mantissa)
}
