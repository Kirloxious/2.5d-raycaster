use clock_ticks::precise_time_s;
use image::{imageops::colorops, GenericImageView, ImageReader};
use sdl2::{
    event::Event,
    keyboard::{Keycode, Scancode},
    pixels::Color,
    rect::Point,
    render::Canvas,
};

const RENDER_HEIGTH: i32 = 600;
const RENDER_WIDTH: i32 = 800;

fn color_interp(a: [u8; 3], b: [u8; 3], t: f64) -> [u8; 3] {
    let invt = 1.0 - t;
    return [
        (a[0] as f64 * invt + b[0] as f64 * t) as u8,
        (a[1] as f64 * invt + b[1] as f64 * t) as u8,
        (a[2] as f64 * invt + b[2] as f64 * t) as u8,
    ];
}

fn number_interp(a: u8, b: u8, t: f64) -> f64 {
    let invt = 1.0 - t;
    return (a as f64 * invt) + (b as f64 * t);
}

fn average(x: u8, y: u8) -> u8 {
    let rrgbx = x as f64 / 255.0;
    let rrgby = y as f64 / 255.0;
    let rsgbx = ((rrgbx + 0.055) / 1.055).powf(2.4);
    let rsgby = ((rrgby + 0.055) / 1.055).powf(2.4);
    let lin_avrg = (rsgbx + rsgby) / 2.0;
    return ((1.055 * lin_avrg).powf(1. / 2.4) * 255. / 2.) as u8;
}

fn combine_color(c1: [u8; 3], c2: [u8; 3]) -> [u8; 3] {
    return [
        average(c1[0], c2[0]),
        average(c1[1], c2[1]),
        average(c1[2], c2[2]),
    ];
}
//only samples 1 nearby pixel, too many caused black, TODO: fix this
fn sample_pixels(x: i32, y: i32, color_map: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>) -> Color {
    let offset = 1;
    let curent_color = color_map.get_pixel(x as u32, y as u32).0;
    let next_color_y = color_map
        .get_pixel(
            (x).clamp(0, 1023) as u32,
            (y + offset).clamp(0, 1023) as u32,
        )
        .0;
    let combined = combine_color(curent_color, next_color_y);
    return Color::RGB(combined[0], combined[1], combined[2]);
}

fn render(
    p: Point,
    angle: f64,
    height: i32,
    horizon: i32,
    scale_height: i32,
    distance: i32,
    screen_width: i32,
    screen_height: i32,
    heigth_map: &image::ImageBuffer<image::Luma<u8>, Vec<u8>>,
    color_map: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    canvas: &mut Canvas<sdl2::video::Window>,
) {
    let sinphi = angle.sin();
    let cosphi = angle.cos();

    let mut ybuffer = Vec::new();
    for _ in 0..screen_width {
        ybuffer.push((0, screen_height));
    }

    let mut dz = 1.;
    let mut z = 1.;

    while z < distance as f64 {
        let mut plx = (-cosphi * z - sinphi * z) + p.x as f64;
        let mut ply = -(sinphi * z - cosphi * z) + p.y as f64;
        let prx = (cosphi * z - sinphi * z) + p.x as f64;
        let pry = -(-sinphi * z - cosphi * z) + p.y as f64;

        // println!("plx: {} ply: {}", plx, ply);
        // println!("prx: {} pry: {}", prx, pry);

        let dx = (prx - plx) / screen_width as f64;
        let dy = (pry - ply) / screen_width as f64;

        for i in 0..screen_width {
            let plxt = plx - plx.floor();
            let plyt = ply - ply.floor();

            let plx_left = plx.floor() as u32 % color_map.width();
            let plx_right = (plx + 1.).floor() as u32 % color_map.width();
            let ply_upper = ply.floor() as u32 % color_map.width();
            let ply_lower = (ply + 1.).floor() as u32 % color_map.width();

            let upper_alt = number_interp(
                heigth_map.get_pixel(ply_upper, plx_left).0[0],
                heigth_map.get_pixel(ply_upper, plx_right).0[0],
                plxt,
            );
            let lower_alt = number_interp(
                heigth_map.get_pixel(ply_lower, plx_left).0[0],
                heigth_map.get_pixel(ply_lower, plx_right).0[0],
                plxt,
            );
            let allitude = number_interp(lower_alt as u8, upper_alt as u8, plyt);

            let height_on_screen =
                ((height as f64 - allitude) / z * scale_height as f64 + horizon as f64) as i32;
            if height_on_screen < RENDER_HEIGTH {
                let upper_color = color_interp(
                    color_map.get_pixel(ply_upper, plx_left).0,
                    color_map.get_pixel(ply_upper, plx_right).0,
                    plxt,
                );
                let lower_color = color_interp(
                    color_map.get_pixel(ply_lower, plx_left).0,
                    color_map.get_pixel(ply_lower, plx_right).0,
                    plxt,
                );
                let color = color_interp(lower_color, upper_color, plyt);

                // let pixel = color_map.get_pixel(offset_x as u32, offset_y as u32).0;

                canvas.set_draw_color(Color::RGB(color[0], color[1], color[2]));
                draw_vertical_line(canvas, i, height_on_screen, ybuffer[i as usize].1);

                if height_on_screen < ybuffer[i as usize].1 {
                    ybuffer[i as usize].1 = height_on_screen
                }

                plx += dx;
                ply += dy;
            }
        }
        z += dz;
        dz += 0.01;
    }
}

fn draw_vertical_line(
    canvas: &mut Canvas<sdl2::video::Window>,
    mut x: i32,
    mut ytop: i32,
    mut ybottom: i32,
) {
    x = x | 0;
    ytop = ytop | 0;
    ybottom = ybottom | 0;
    if ytop < 0 {
        ytop = 0
    };
    if ytop > ybottom {
        return;
    };

    canvas
        .draw_line(Point::new(x, ytop), Point::new(x, ybottom))
        .unwrap();
}

fn main() {
    let color_map = ImageReader::open("maps/C17w.png")
        .unwrap()
        .decode()
        .unwrap();
    let height_map = ImageReader::open("maps/D17.png").unwrap().decode().unwrap();

    println!("{}", color_map.height());
    println!("{}", color_map.width());

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Raycaster", RENDER_WIDTH as u32, RENDER_HEIGTH as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut time = precise_time_s();
    let mut time_last_frame = precise_time_s();
    let mut frames = 0.;

    let mut posy = 800;
    let mut posx = 500;
    let mut rot: f64 = 0.0;

    'running: loop {
        let current_time = precise_time_s();
        let fps = frames / (current_time - time_last_frame);
        time_last_frame = current_time;
        frames = 0.;
        println!("{}", fps);

        let keyboard_state = event_pump.keyboard_state();
        for pressed in keyboard_state.pressed_scancodes() {
            match pressed {
                Scancode::W => {
                    posy += (5. * (current_time - time) * 3.).ceil() as i32;
                }
                Scancode::S => {
                    posy -= (5. * (current_time - time) * 3.).ceil() as i32;
                }
                Scancode::A => {
                    posx -= (5. * (current_time - time) * 3.).ceil() as i32;
                }
                Scancode::D => {
                    posx += (5. * (current_time - time) * 3.).ceil() as i32;
                }
                Scancode::Q => {
                    rot += 1.0 * (current_time - time) * 3.;
                }
                Scancode::E => {
                    rot -= 1.0 * (current_time - time) * 3.;
                }
                _ => {}
            }
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        canvas.set_draw_color(Color::RGB(112, 206, 235));
        canvas.clear();

        render(
            Point::new(posx, posy),
            rot,
            78,
            120,
            220,
            300,
            RENDER_WIDTH,
            RENDER_HEIGTH,
            height_map.as_luma8().unwrap(),
            color_map.as_rgb8().unwrap(),
            &mut canvas,
        );
        canvas.present();

        time = current_time;
        frames += 1.;
    }
}
