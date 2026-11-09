use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use std::collections::VecDeque;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const OUT_RATE: u32 = 48_000;
const MAX_BACKLOG_FRAMES: usize = 9_600;

type Log = Arc<Mutex<VecDeque<String>>>;
type Queue = Arc<Mutex<VecDeque<f32>>>;

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    System,
    Microphone,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Kind::System => "system",
            Kind::Microphone => "microphone",
        }
    }
}

pub struct AudioInput {
    pub port: u16,
    pub kind: Kind,
}

pub struct AudioSession {
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
    pub inputs: Vec<AudioInput>,
}

impl AudioSession {
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

fn note(log: &Log, text: String) {
    let mut lines = log.lock().unwrap_or_else(|e| e.into_inner());
    lines.push_back(text);
    while lines.len() > 40 {
        lines.pop_front();
    }
}

struct Converter {
    channels: usize,
    step: f64,
    pos: f64,
    prev: [f32; 2],
}

impl Converter {
    fn new(channels: usize, rate: u32) -> Self {
        Self {
            channels: channels.max(1),
            step: rate as f64 / OUT_RATE as f64,
            pos: 0.0,
            prev: [0.0, 0.0],
        }
    }

    fn push(&mut self, data: &[f32], out: &mut Vec<f32>) {
        for frame in data.chunks_exact(self.channels) {
            let left = frame[0];
            let right = if self.channels > 1 { frame[1] } else { frame[0] };
            while self.pos < 1.0 {
                let t = self.pos as f32;
                out.push(self.prev[0] + (left - self.prev[0]) * t);
                out.push(self.prev[1] + (right - self.prev[1]) * t);
                self.pos += self.step;
            }
            self.pos -= 1.0;
            self.prev = [left, right];
        }
    }
}

fn sink(queue: &Queue, converter: &mut Converter, data: &[f32]) {
    let mut converted = Vec::with_capacity(data.len() * 2);
    converter.push(data, &mut converted);
    let mut q = queue.lock().unwrap_or_else(|e| e.into_inner());
    q.extend(converted);
}

fn open_stream(kind: Kind, queue: Queue, log: Log) -> Result<Stream, String> {
    let host = cpal::default_host();
    let (device, supported) = match kind {
        Kind::System => {
            let device = host
                .default_output_device()
                .ok_or("no audio output device found")?;
            let supported = device
                .default_output_config()
                .map_err(|e| e.to_string())?;
            (device, supported)
        }
        Kind::Microphone => {
            let device = host.default_input_device().ok_or("no microphone found")?;
            let supported = device.default_input_config().map_err(|e| e.to_string())?;
            (device, supported)
        }
    };

    let channels = supported.channels() as usize;
    let rate = supported.sample_rate().0;
    let format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let label = kind.label();
    let on_error = move |e: cpal::StreamError| note(&log, format!("{label} audio error: {e}"));
    let mut converter = Converter::new(channels, rate);

    let stream = match format {
        SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| sink(&queue, &mut converter, data),
            on_error,
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                let floats: Vec<f32> = data.iter().map(|s| *s as f32 / 32768.0).collect();
                sink(&queue, &mut converter, &floats);
            },
            on_error,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            &config,
            move |data: &[u16], _: &cpal::InputCallbackInfo| {
                let floats: Vec<f32> = data.iter().map(|s| (*s as f32 - 32768.0) / 32768.0).collect();
                sink(&queue, &mut converter, &floats);
            },
            on_error,
            None,
        ),
        other => return Err(format!("unsupported audio format {other:?}")),
    }
    .map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}

fn accept_until(listener: &TcpListener, stop: &AtomicBool) -> Option<TcpStream> {
    let _ = listener.set_nonblocking(true);
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((conn, _)) => {
                let _ = conn.set_nonblocking(false);
                return Some(conn);
            }
            Err(_) => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    None
}

fn pump(conn: &mut TcpStream, queue: &Queue, stop: &AtomicBool) {
    let started = Instant::now();
    let mut written: u64 = 0;
    while !stop.load(Ordering::Relaxed) {
        let target = (started.elapsed().as_secs_f64() * OUT_RATE as f64) as u64;
        let due = target.saturating_sub(written) as usize;
        if due > 0 {
            let need = due * 2;
            let mut samples: Vec<f32> = Vec::with_capacity(need);
            {
                let mut q = queue.lock().unwrap_or_else(|e| e.into_inner());
                let keep = need + MAX_BACKLOG_FRAMES * 2;
                while q.len() > keep {
                    q.pop_front();
                }
                let take = need.min(q.len());
                samples.extend(q.drain(..take));
            }
            samples.resize(need, 0.0);
            let mut bytes = Vec::with_capacity(need * 4);
            for sample in &samples {
                bytes.extend_from_slice(&sample.to_le_bytes());
            }
            if conn.write_all(&bytes).is_err() {
                break;
            }
            written += due as u64;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn run_source(kind: Kind, listener: TcpListener, stop: Arc<AtomicBool>, log: Log) {
    let Some(mut conn) = accept_until(&listener, &stop) else {
        return;
    };
    let _ = conn.set_nodelay(true);
    let queue: Queue = Arc::new(Mutex::new(VecDeque::new()));
    let _stream = match open_stream(kind, queue.clone(), log.clone()) {
        Ok(stream) => Some(stream),
        Err(e) => {
            note(&log, format!("{} audio unavailable, sending silence: {e}", kind.label()));
            None
        }
    };
    pump(&mut conn, &queue, &stop);
}

pub fn start(system: bool, microphone: bool, log: &Log) -> Result<AudioSession, String> {
    let stop = Arc::new(AtomicBool::new(false));
    let mut threads = Vec::new();
    let mut inputs = Vec::new();
    for (enabled, kind) in [(system, Kind::System), (microphone, Kind::Microphone)] {
        if !enabled {
            continue;
        }
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("could not open the audio channel: {e}"))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let stop_for_thread = stop.clone();
        let log_for_thread = log.clone();
        threads.push(std::thread::spawn(move || {
            run_source(kind, listener, stop_for_thread, log_for_thread)
        }));
        inputs.push(AudioInput { port, kind });
    }
    Ok(AudioSession { stop, threads, inputs })
}
