use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(target_os = "macos")]
use std::sync::mpsc;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub struct Recording {
    #[cfg(not(target_os = "macos"))]
    stream: cpal::Stream,
    #[cfg(target_os = "macos")]
    stop: Option<mpsc::Sender<()>>,
    #[cfg(target_os = "macos")]
    audio_thread: Option<std::thread::JoinHandle<()>>,
    samples: Arc<Mutex<Vec<f32>>>,
    error: Arc<Mutex<Option<String>>>,
    sample_rate: u32,
    started_at: Instant,
}

impl Recording {
    pub fn start(input_device: Option<&str>) -> Result<Self, String> {
        let samples = Arc::new(Mutex::new(Vec::new()));
        let error = Arc::new(Mutex::new(None));

        #[cfg(target_os = "macos")]
        {
            let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
            let (stop_sender, stop_receiver) = mpsc::channel();
            let device_name = input_device.map(str::to_owned);
            let thread_samples = samples.clone();
            let thread_error = error.clone();
            let audio_thread = std::thread::spawn(move || {
                match start_input_stream(
                    device_name.as_deref(),
                    thread_samples,
                    thread_error.clone(),
                ) {
                    Ok((stream, sample_rate)) => {
                        let _ = ready_sender.send(Ok(sample_rate));
                        let _ = stop_receiver.recv();
                        drop(stream);
                    }
                    Err(cause) => {
                        let _ = ready_sender.send(Err(cause));
                    }
                }
            });
            let sample_rate = match ready_receiver.recv() {
                Ok(Ok(sample_rate)) => sample_rate,
                Ok(Err(cause)) => {
                    let _ = audio_thread.join();
                    return Err(cause);
                }
                Err(_) => {
                    let _ = audio_thread.join();
                    return Err("麦克风线程在初始化前退出".into());
                }
            };
            Ok(Self {
                stop: Some(stop_sender),
                audio_thread: Some(audio_thread),
                samples,
                error,
                sample_rate,
                started_at: Instant::now(),
            })
        }

        #[cfg(not(target_os = "macos"))]
        {
            let (stream, sample_rate) =
                start_input_stream(input_device, samples.clone(), error.clone())?;
            Ok(Self {
                stream,
                samples,
                error,
                sample_rate,
                started_at: Instant::now(),
            })
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn level(&self) -> f32 {
        let Ok(samples) = self.samples.lock() else {
            return 0.0;
        };
        let recent = &samples[samples.len().saturating_sub(1_600)..];
        if recent.is_empty() {
            return 0.0;
        }
        (recent.iter().map(|sample| sample * sample).sum::<f32>() / recent.len() as f32)
            .sqrt()
            .clamp(0.0, 1.0)
    }

    pub fn take_error(&self) -> Option<String> {
        self.error.lock().ok()?.take()
    }

    pub fn preview_chunk(&self, cursor: &mut usize) -> (u32, Vec<i16>) {
        let Ok(samples) = self.samples.lock() else {
            return (self.sample_rate, Vec::new());
        };
        (self.sample_rate, pcm16_chunk(&samples, cursor))
    }

    pub fn finish(self) -> Result<Vec<i16>, String> {
        #[cfg(target_os = "macos")]
        let mut recording = self;
        #[cfg(not(target_os = "macos"))]
        let recording = self;
        #[cfg(target_os = "macos")]
        recording.stop_audio_thread();
        #[cfg(not(target_os = "macos"))]
        drop(recording.stream);
        if let Some(error) = recording.error.lock().map_err(|_| "录音状态损坏")?.take() {
            return Err(format!("麦克风录音中断：{error}"));
        }
        let input = std::mem::take(&mut *recording.samples.lock().map_err(|_| "录音缓冲损坏")?);
        if input.is_empty() {
            return Err("没有采集到音频".into());
        }
        Ok(resample_to_16khz(&input, recording.sample_rate))
    }

    #[cfg(target_os = "macos")]
    fn stop_audio_thread(&mut self) {
        drop(self.stop.take());
        if let Some(thread) = self.audio_thread.take() {
            if thread.join().is_err() {
                set_error(&self.error, "麦克风线程异常退出".into());
            }
        }
    }
}

#[cfg(target_os = "macos")]
impl Drop for Recording {
    fn drop(&mut self) {
        self.stop_audio_thread();
    }
}

fn start_input_stream(
    input_device: Option<&str>,
    samples: Arc<Mutex<Vec<f32>>>,
    error: Arc<Mutex<Option<String>>>,
) -> Result<(cpal::Stream, u32), String> {
    let host = cpal::default_host();
    let device = if let Some(name) = input_device.filter(|value| !value.is_empty()) {
        host.input_devices()
            .map_err(|error| format!("无法读取麦克风列表：{error}"))?
            .find(|device| device.name().as_deref() == Ok(name))
            .ok_or_else(|| format!("麦克风已断开：{name}"))?
    } else {
        host.default_input_device().ok_or("未找到可用麦克风")?
    };
    let supported = device
        .default_input_config()
        .map_err(|error| format!("无法读取麦克风格式：{error}"))?;
    let sample_rate = supported.sample_rate().0;
    let sample_format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let channels = usize::from(config.channels);
    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            let output = samples.clone();
            let failure = error.clone();
            device.build_input_stream(
                &config,
                move |data: &[f32], _| push_mono_f32(data, channels, &output),
                move |cause| set_error(&failure, cause.to_string()),
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let output = samples.clone();
            let failure = error.clone();
            device.build_input_stream(
                &config,
                move |data: &[i16], _| push_mono_i16(data, channels, &output),
                move |cause| set_error(&failure, cause.to_string()),
                None,
            )
        }
        cpal::SampleFormat::U16 => {
            let output = samples.clone();
            let failure = error.clone();
            device.build_input_stream(
                &config,
                move |data: &[u16], _| push_mono_u16(data, channels, &output),
                move |cause| set_error(&failure, cause.to_string()),
                None,
            )
        }
        format => return Err(format!("暂不支持麦克风采样格式：{format:?}")),
    }
    .map_err(|error| format!("无法启动麦克风：{error}"))?;
    stream
        .play()
        .map_err(|error| format!("无法开始录音：{error}"))?;
    Ok((stream, sample_rate))
}

pub fn input_devices() -> Result<Vec<String>, String> {
    let host = cpal::default_host();
    let mut names = host
        .input_devices()
        .map_err(|error| format!("无法读取麦克风列表：{error}"))?
        .filter_map(|device| device.name().ok())
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    Ok(names)
}

pub fn measure_input_level(input_device: Option<&str>) -> Result<f32, String> {
    let recording = Recording::start(input_device)?;
    std::thread::sleep(Duration::from_millis(500));
    if let Some(error) = recording.take_error() {
        return Err(format!("AUDIO_DEVICE_DISCONNECTED：{error}"));
    }
    Ok(recording.level())
}

fn set_error(target: &Arc<Mutex<Option<String>>>, message: String) {
    if let Ok(mut slot) = target.lock() {
        *slot = Some(message);
    }
}

fn push_mono_f32(data: &[f32], channels: usize, output: &Arc<Mutex<Vec<f32>>>) {
    push_mono(data, channels, output, |sample| sample);
}

fn push_mono_i16(data: &[i16], channels: usize, output: &Arc<Mutex<Vec<f32>>>) {
    push_mono(data, channels, output, |sample| {
        sample as f32 / i16::MAX as f32
    });
}

fn push_mono_u16(data: &[u16], channels: usize, output: &Arc<Mutex<Vec<f32>>>) {
    push_mono(data, channels, output, |sample| {
        sample as f32 / 32767.5 - 1.0
    });
}

fn push_mono<T: Copy>(
    data: &[T],
    channels: usize,
    output: &Arc<Mutex<Vec<f32>>>,
    normalize: impl Fn(T) -> f32,
) {
    let Ok(mut output) = output.lock() else {
        return;
    };
    output.reserve(data.len() / channels.max(1));
    for frame in data.chunks_exact(channels.max(1)) {
        output.push(frame.iter().copied().map(&normalize).sum::<f32>() / frame.len() as f32);
    }
}

fn resample_to_16khz(input: &[f32], input_rate: u32) -> Vec<i16> {
    let output_len = (input.len() as u64 * 16_000 / u64::from(input_rate)) as usize;
    let mut output = Vec::with_capacity(output_len);
    for index in 0..output_len {
        let position = index as f64 * input_rate as f64 / 16_000.0;
        let left = position.floor() as usize;
        let fraction = (position - left as f64) as f32;
        let right = (left + 1).min(input.len() - 1);
        let sample = input[left] + (input[right] - input[left]) * fraction;
        output.push((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
    }
    output
}

fn pcm16_chunk(samples: &[f32], cursor: &mut usize) -> Vec<i16> {
    *cursor = (*cursor).min(samples.len());
    let output = samples[*cursor..]
        .iter()
        .map(|sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect();
    *cursor = samples.len();
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resampler_keeps_duration() {
        let output = resample_to_16khz(&vec![0.25; 48_000], 48_000);
        assert_eq!(output.len(), 16_000);
        assert!(output
            .iter()
            .all(|sample| *sample > 8_000 && *sample < 8_300));
    }

    #[test]
    fn preview_chunks_only_return_new_samples() {
        let mut cursor = 0;
        assert_eq!(pcm16_chunk(&[0.0, 0.5], &mut cursor), [0, 16_383]);
        assert!(pcm16_chunk(&[0.0, 0.5], &mut cursor).is_empty());
        assert_eq!(pcm16_chunk(&[0.0, 0.5, -0.5], &mut cursor), [-16_383]);
    }
}
