use std::cmp;

use ui_audio::{AudioReader, FftWindow, analyze_vowel};
use ui_graphics::*;
use egui::plot;

struct Data {
    x_min: usize,
    last_x_min: usize,
}
fn main() {
    let buffer = AudioReader::read("assets/blip.ogg");

    //let buffer = AudioReader::read("assets/bud.ogg");
    //let buffer = AudioReader::read("assets/pod.ogg");
    //let buffer = AudioReader::read("assets/my.ogg");
    //let buffer = AudioReader::read("assets/kite.ogg");
    //let buffer = AudioReader::read("assets/booed.ogg");
    let buffer = AudioReader::read("assets/above.ogg");
    //let buffer = AudioReader::read("assets/bead.ogg");
    //let buffer = AudioReader::read("assets/rye.ogg");
    //let buffer = AudioReader::read("assets/boy.ogg");
    //let buffer = AudioReader::read("assets/bid.ogg");
    let fft_len = 1024;
    let samples = 14410;
    let fft = FftWindow::new(fft_len);

    let main_loop = main_loop::MainLoop::new();
    main_loop.run(move |ui| {
        let offset = 4000;

        //let wave: plot::PlotPoints = (offset..offset + len).map(|i| {
        let wave: plot::PlotPoints = (0..buffer.len()).map(|i| {
            let x = 1000.0 * i as f64 / samples as f64;
            [x, buffer.get(i) as f64]
        }).collect();

        let line = plot::Line::new(wave);

        ui.vertical(|ui| {
            ui.label("Waveform");

            let mut bounds = [0.0f64, 0.0];

            plot::Plot::new("waveform")
                .height(0.5 * ui.available_height())
                .show(ui, |plot_ui| {
                plot_ui.line(line);

                bounds = plot_ui.plot_bounds().min();
            });

            let fft_offset: usize = (bounds[0] * samples as f64 / 1000.0) as usize;

            let fft_offset = cmp::min(buffer.len() - fft_len, fft_offset);
            let fft_offset = cmp::max(0, fft_offset);

            let mut vec: Vec<f32> = (fft_offset..fft_offset + fft_len).map(|i| {
                buffer.get(i)
            }).collect();
    
            fft.process_in_place(&mut vec);
            let vec = &mut vec[0..fft_len / 2];
            fft.normalize(vec);

            let gram = analyze_vowel(
                &buffer[fft_offset.. fft_offset + fft_len], 
                vec,
                samples,
                fft_len
            );
    
            let fft_plot: plot::PlotPoints = vec.iter().enumerate().map(|(i, data)| {
                let x = i as f64 * samples as f64 / fft_len as f64;
                [x, *data as f64]
            }).collect();
    
            let fft_line = plot::Line::new(fft_plot);
    
            ui.label(format!("FFT '{}'", gram));
            plot::Plot::new("fft")
                .height(ui.available_height())
                .show(ui, |plot_ui| {
                //plot_ui.set_plot_bounds([plot_ui.plot_bounds().0, 1.2]);
                plot_ui.line(fft_line);
            });
        });
    }).unwrap();
}