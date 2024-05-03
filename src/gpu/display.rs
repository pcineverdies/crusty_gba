use sdl2::pixels::{Color, PixelFormat, PixelFormatEnum};
use sdl2::render::Canvas;
use sdl2::keyboard::Keycode;
use sdl2::video::Window;
use sdl2::event::Event;
use sdl2::Sdl;

pub const GBA_SCREEN_WIDTH: u32 = 240;
pub const GBA_SCREEN_HEIGHT: u32 = 160;
const SCALE_FACTOR: u32 = 3;

pub struct Display {
    sdl_context: Sdl,
    video_subsystem: sdl2::VideoSubsystem,
    canvas: Canvas<Window>,
}

impl Display {
    pub fn new() -> Self {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let window = video_subsystem
            .window(
                "Crusty GBA",
                GBA_SCREEN_WIDTH * SCALE_FACTOR,
                GBA_SCREEN_HEIGHT * SCALE_FACTOR,
            )
            .position_centered()
            .build()
            .unwrap();

        let mut canvas = window.into_canvas().build().unwrap();

        canvas.set_draw_color(Color::RGB(0, 255, 255));
        canvas.clear();
        canvas.present();

        Self {
            sdl_context,
            video_subsystem,
            canvas,
        }
    }

    pub fn update(&mut self, frame: &Vec<u8>) {
        if frame.len() != (GBA_SCREEN_WIDTH * GBA_SCREEN_HEIGHT * 4) as usize {
            panic!("Frame is not correctly sized to be displayed!");
        }
        self.canvas.clear();

        let texture_creator = self.canvas.texture_creator();
        let mut texture = texture_creator
            .create_texture_static(
                PixelFormatEnum::RGBA8888,
                GBA_SCREEN_WIDTH,
                GBA_SCREEN_HEIGHT,
            )
            .unwrap();

        let _ = texture.update(None, &frame, 4 * GBA_SCREEN_WIDTH as usize);

        let _ = self.canvas.copy(&texture, None, None);

        self.canvas.present();

        let mut event_pump = self.sdl_context.event_pump().unwrap();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    std::process::exit(0)
                },
                _ => {}
            }
        }
    }

    pub fn clear(&mut self, color: u32) {
        self.canvas.set_draw_color(Color::from_u32(
            &PixelFormat::try_from(PixelFormatEnum::RGBA8888).unwrap(),
            color,
        ));
        self.canvas.clear();
        self.canvas.present();
    }
}
