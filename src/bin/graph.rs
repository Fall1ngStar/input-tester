use minifb::{Key, KeyRepeat, Window, WindowOptions};
use plotters::prelude::*;
use plotters_bitmap::bitmap_pixel::BGRXPixel;
use plotters_bitmap::BitMapBackend;
use rdev::{listen, Event};
use std::borrow::{Borrow, BorrowMut};
use std::collections::VecDeque;
use std::error::Error;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};
const W: usize = 800;
const H: usize = 600;

const BUFFER_SIZE: usize = 250;

const SAMPLE_RATE: f64 = 10000.0;
const FRAME_RATE: f64 = 120.0;

struct BufferWrapper(Vec<u32>);
impl Borrow<[u8]> for BufferWrapper {
    fn borrow(&self) -> &[u8] {
        // Safe for alignment: align_of(u8) <= align_of(u32)
        // Safe for cast: u32 can be thought of as being transparent over [u8; 4]
        unsafe { std::slice::from_raw_parts(self.0.as_ptr() as *const u8, self.0.len() * 4) }
    }
}
impl BorrowMut<[u8]> for BufferWrapper {
    fn borrow_mut(&mut self) -> &mut [u8] {
        // Safe for alignment: align_of(u8) <= align_of(u32)
        // Safe for cast: u32 can be thought of as being transparent over [u8; 4]
        unsafe { std::slice::from_raw_parts_mut(self.0.as_mut_ptr() as *mut u8, self.0.len() * 4) }
    }
}
impl Borrow<[u32]> for BufferWrapper {
    fn borrow(&self) -> &[u32] {
        self.0.as_slice()
    }
}
impl BorrowMut<[u32]> for BufferWrapper {
    fn borrow_mut(&mut self) -> &mut [u32] {
        self.0.as_mut_slice()
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut buf = BufferWrapper(vec![0u32; W * H]);
    let mut previous_time: SystemTime = SystemTime::now();
    let data = Arc::new(Mutex::new(VecDeque::with_capacity(BUFFER_SIZE)));

    // let callback = move |event: Event| {
    //     if let Ok(duration) = event.time.duration_since(previous_time) {
    //         // println!("{:?}", duration);
    //         let data = &mut data;
    //         data.push_back(duration);
    //         previous_time = event.time.clone();
    //     }
    // };

    let mut window = Window::new("", W, H, WindowOptions::default())?;
    let mut last_update = Instant::now();

    let cs = {
        let root = BitMapBackend::<BGRXPixel>::with_buffer_and_format(
            buf.borrow_mut(),
            (W as u32, H as u32),
        )?
        .into_drawing_area();
        root.fill(&BLACK)?;

        let mut chart = ChartBuilder::on(&root)
            .margin(10)
            .set_all_label_area_size(30)
            .build_cartesian_2d(0..250u128, 0u128..40_000u128)?;

        chart
            .configure_mesh()
            .label_style(("sans-serif", 15).into_font().color(&GREEN))
            .axis_style(&GREEN)
            .draw()?;

        let cs = chart.into_chart_state();
        root.present()?;
        cs
    };
    let cloned_data = data.clone();
    thread::spawn(move || {
        let data = cloned_data;
        let _result = listen(move |event| {
            match event.event_type {
                rdev::EventType::KeyPress(key) if key == rdev::Key::Escape => {
                    exit(0);
                }
                rdev::EventType::MouseMove { .. } => {
                    if let Ok(duration) = event.time.duration_since(previous_time) {
                        // println!("{:?}", duration);
                        if let Ok(mut data) = data.lock() {
                            data.push_back(duration);
                            if data.len() > BUFFER_SIZE {
                                data.pop_front();
                            }
                        }
                        previous_time = event.time.clone();
                    }
                }
                _ => {}
            }
        });
    });

    while window.is_open() {
        {
            let root = BitMapBackend::<BGRXPixel>::with_buffer_and_format(
                buf.borrow_mut(),
                (W as u32, H as u32),
            )
            .unwrap()
            .into_drawing_area();
            {
                let mut chart = cs.clone().restore(&root);
                chart.plotting_area().fill(&BLACK).unwrap();

                chart
                    .configure_mesh()
                    .bold_line_style(&GREEN.mix(0.2))
                    .light_line_style(&TRANSPARENT)
                    .draw()
                    .unwrap();

                let data_clone = data.lock().unwrap().clone();
                chart
                    .draw_series(LineSeries::new(
                        data_clone
                            .iter()
                            .enumerate()
                            .map(|(i, duration)| return (i as u128 % 1000, duration.as_micros())),
                        RED,
                    ))
                    .unwrap();
            }
        }

        if last_update.elapsed() > Duration::from_millis((1000.0 / FRAME_RATE) as u64) {
            last_update = Instant::now();
            window.update_with_buffer(buf.borrow(), W, H).unwrap();
            // Update the buffer and window
            // ...
        }
    }

    Ok(())
}
