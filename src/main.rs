use ggez::{
    conf::{WindowMode, WindowSetup},
    event::{self, EventHandler},
    glam::{ivec2, vec2, IVec2, Vec2},
    graphics::{self, Color, DrawParam, Drawable, Mesh, Text},
    input::keyboard::{KeyCode, KeyInput},
    timer, Context, ContextBuilder, GameError, GameResult,
};
use std::{f32::consts::PI, str::from_utf8, thread::JoinHandle, collections::VecDeque};
use crossbeam_channel::{Sender, Receiver};
mod audio;
mod texture;

const WAVE_SIZE: usize = 4410;

const MAP: &str = "#########.......\
#...............\
#.......########\
#..............#\
#......##......#\
#......##......#\
#..............#\
###............#\
##.............#\
#......####..###\
#......#.......#\
#......#.......#\
#..............#\
#......#########\
#..............#\
################";

enum Direction {
    Forward,
    Back,
    Right,
    Left,
}

struct InputState {
    x: i8,
    y: i8,
    a: i8,
}
impl InputState {
    fn new() -> Self {
        Self { x: 0, y: 0, a: 0 }
    }
    fn destruct(&self) -> (i8, i8, i8) {
        (self.x, self.y, self.a)
    }
}

struct Player {
    pos: Vec2,
    angle: f32,
    fov: f32,
    speed: f32,
    controller: InputState,
}

impl Player {
    fn new(x: f32, y: f32) -> Self {
        Self {
            pos: vec2(x, y),
            angle: 0.,
            fov: PI / 2., // 90 degrees
            speed: 2.,
            controller: InputState::new(),
        }
    }
    fn step(&mut self, dir: Direction, dt: f32) {
        let change = self.speed * dt;
        let cossin = vec2(self.angle.cos(), self.angle.sin());
        let psinmcos = vec2(self.angle.sin(), -self.angle.cos());
        match dir {
            Direction::Forward => {
                self.pos += cossin * change;
            }
            Direction::Back => {
                self.pos -= cossin * change;
            }
            Direction::Left => {
                self.pos += psinmcos * change;
            }
            Direction::Right => {
                self.pos -= psinmcos * change;
            }
        }
    }
    fn rotate(&mut self, dir: Direction, dt: f32) {
        match dir {
            Direction::Left => {
                self.angle -= self.speed * 0.75 * dt;
            }
            Direction::Right => {
                self.angle += self.speed * 0.75 * dt;
            }
            _ => {}
        }
    }
    fn handle_input(&mut self, dt: f32) {
        let (x, y, a) = self.controller.destruct();
        if x > 0 {
            self.step(Direction::Right, dt);
        } else if x < 0 {
            self.step(Direction::Left, dt);
        }
        if y > 0 {
            self.step(Direction::Forward, dt);
        } else if y < 0 {
            self.step(Direction::Back, dt);
        }
        if a > 0 {
            self.rotate(Direction::Right, dt);
        } else if a < 0 {
            self.rotate(Direction::Left, dt);
        }
    }
}

struct Game {
    size: IVec2,
    //_map: Vec<char>,
    map: Vec<u8>,
    player: Player,
    render_distance: f32,
    draw_map: bool,
    tx: Sender<audio::ToAudio>,
    rx: Receiver<audio::FromAudio>,
    wall_texture: texture::Texture,
    wave_buffer: VecDeque<f32>,
}

impl Game {
    fn new(width: i32, height: i32, render_distance: f32, tx: Sender<audio::ToAudio>, rx: Receiver<audio::FromAudio>) -> Self {
        Self {
            size: ivec2(width, height),
            render_distance,
            player: Player::new(14., 5.),
            //map: MAP.chars().collect(),
            map: MAP.as_bytes().iter().map(|x| *x).collect(),
            draw_map: false,
            tx,
            rx,
            wall_texture: texture::Texture::new(WAVE_SIZE, 32),
            wave_buffer: VecDeque::with_capacity(WAVE_SIZE)
        }
    }

    fn raycast(&self, ctx: &Context) -> GameResult<Mesh> {
        let (screen_width, screen_height) = ctx.gfx.drawable_size();
        let mut mb = graphics::MeshBuilder::new();
        for x in 0..screen_width as u32 {
            // raycasting
            let ray_angle = self.player.angle - self.player.fov / 2.
                + (x as f32 / screen_width) * self.player.fov;
            let ray_direction = vec2(ray_angle.cos(), ray_angle.sin());
            let step_size = vec2(
                (1. + (ray_direction.y / ray_direction.x) * (ray_direction.y / ray_direction.x))
                    .sqrt(),
                (1. + (ray_direction.x / ray_direction.y) * (ray_direction.x / ray_direction.y))
                    .sqrt(),
            );
            let mut map_check = self.player.pos.as_ivec2();
            let step = ivec2(
                ray_direction.x.signum() as i32,
                ray_direction.y.signum() as i32,
            );
            let mut ray_length1d = vec2(
                if ray_direction.x < 0.0 {
                    (self.player.pos.x - map_check.x as f32) * step_size.x
                } else {
                    ((map_check.x + 1) as f32 - self.player.pos.x) * step_size.x
                },
                if ray_direction.y < 0.0 {
                    (self.player.pos.y - map_check.y as f32) * step_size.y
                } else {
                    ((map_check.y + 1) as f32 - self.player.pos.y) * step_size.y
                },
            );

            let mut tile_found = false;
            let mut distance = 0.0;
            let mut texture_sample_x = -1.; // set to smth valid when hit wall
            
            while !tile_found && distance < self.render_distance {
                // walk shortest path
                if ray_length1d.x < ray_length1d.y {
                    map_check.x += step.x;
                    distance = ray_length1d.x;
                    ray_length1d.x += step_size.x;
                } else {
                    map_check.y += step.y;
                    distance = ray_length1d.y;
                    ray_length1d.y += step_size.y;
                }
                // test map to see where/if we hit
                if map_check.x >= 0
                    && map_check.x < self.size.x
                    && map_check.y >= 0
                    && map_check.y < self.size.y
                {
                    tile_found =
                        self.map[(map_check.y * self.size.x + map_check.x) as usize] == b'#';
                    let tile_midpoint = map_check.as_vec2() + 0.5;
                    let tile_intersection = self.player.pos + ray_direction * distance;
                    let intersect_angle = (tile_intersection.y - tile_midpoint.y).atan2(tile_intersection.x - tile_midpoint.x);
                    if intersect_angle >= -PI * 0.25 && intersect_angle < PI * 0.25 {
                        texture_sample_x = tile_intersection.y - map_check.y as f32;
                    }
                    if intersect_angle >= PI * 0.25 && intersect_angle < PI * 0.75 {
                        texture_sample_x = tile_intersection.x - map_check.x as f32;
                    }
                    if intersect_angle < -PI * 0.25 && intersect_angle >= -PI * 0.75 {
                        texture_sample_x = tile_intersection.x - map_check.x as f32;
                    }
                    if intersect_angle >= PI * 0.75 || intersect_angle < -PI * 0.75 {
                        texture_sample_x = tile_intersection.y - map_check.y as f32;
                    }
                }
            }

            let sh = screen_height;
            let ceil_distance = (sh / 2.) - sh / distance;
            let floor_distance = sh - ceil_distance;
            let c = 1. - distance / self.render_distance;
            let line_distance = floor_distance - ceil_distance;

            for y in 0..self.wall_texture.height() {
                let texture_sample_y = y as f32 / self.wall_texture.height() as f32;
                let next_y = (y + 1) as f32 / self.wall_texture.height() as f32;
                mb.line(
                    &[
                        vec2(x as f32, ceil_distance + texture_sample_y * line_distance),
                        vec2(x as f32, ceil_distance + next_y * line_distance),
                    ],
                    1.0,
                    self.wall_texture.sample_color_weighted(texture_sample_x, texture_sample_y, c),
                )?;
            }
        }
        Ok(Mesh::from_data(&ctx.gfx, mb.build()))
    }
}

impl EventHandler for Game {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        // store last player location on map
        let old_pos = self.player.pos;
        // update player position
        let delta_time = ctx.time.delta().as_secs_f32();
        self.player.handle_input(delta_time);

        let mut new_pos = self.player.pos;
        // check map bounds
        if !(0..self.size.x).contains(&(new_pos.x as i32))
            || !(0..self.size.y).contains(&(new_pos.y as i32))
        {
            self.player.pos = old_pos;
            new_pos = self.player.pos;
        }
        // check wall collision
        if self.map[(new_pos.y as i32 * self.size.x + new_pos.x as i32) as usize] == b'#' {
            self.player.pos = old_pos;
            new_pos = self.player.pos;
        }
        // update map
        if old_pos != new_pos {
            self.map[(old_pos.y as i32 * self.size.x + old_pos.x as i32) as usize] = b'.';
            self.map[(new_pos.y as i32 * self.size.x + new_pos.x as i32) as usize] = b'P';
        }
        // update wall texture with data from audio thread
        // get new data
        for _ in 0..WAVE_SIZE {
            if let Ok(audio::FromAudio::Data(data)) = self.rx.try_recv() {
                if self.wave_buffer.len() == WAVE_SIZE {
                    self.wave_buffer.pop_front(); // remove what's already off screen
                }
                self.wave_buffer.push_back(data);
            } else {
                break;
            }
        }
        self.wall_texture.clear();
        // move data into texture
        for (i, val) in self.wave_buffer.iter().enumerate() {
            self.wall_texture.set_color(i, (val * 32. + 16.) as usize, Color::BLACK);
        }
        Ok(())
    }

    fn key_down_event(&mut self, _ctx: &mut Context, input: KeyInput, _repeat: bool) -> GameResult {
        if let Some(keycode) = input.keycode {
            match keycode {
                KeyCode::A => {
                    // left
                    self.player.controller.x = -1;
                }
                KeyCode::W => {
                    //forw
                    self.player.controller.y = 1;
                }
                KeyCode::D => {
                    //right
                    self.player.controller.x = 1;
                }
                KeyCode::S => {
                    //back
                    self.player.controller.y = -1;
                }
                KeyCode::Left => {
                    //rot left
                    self.player.controller.a = -1;
                }
                KeyCode::Right => {
                    //rot right
                    self.player.controller.a = 1;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn key_up_event(&mut self, ctx: &mut Context, input: KeyInput) -> GameResult {
        if let Some(keycode) = input.keycode {
            match keycode {
                KeyCode::Escape => ctx.request_quit(),
                KeyCode::A | KeyCode::D => self.player.controller.x = 0,
                KeyCode::W | KeyCode::S => self.player.controller.y = 0,
                KeyCode::Left | KeyCode::Right => self.player.controller.a = 0,
                KeyCode::M => self.draw_map = !self.draw_map,
                _ => {}
            }
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        let mut canvas = graphics::Canvas::from_frame(ctx, Color::BLACK);
        let mesh = self.raycast(&ctx)?;
        canvas.draw(&mesh, DrawParam::default());

        let mut y = 20.0;
        if self.draw_map {
            for dy in 0..self.size.y {
                let t = Text::new(
                    from_utf8(
                        &self.map[(dy * self.size.x) as usize
                            ..(dy * self.size.x + self.size.x) as usize],
                    )
                    .unwrap(),
                );
                canvas.draw(
                    &t,
                    DrawParam::default().dest(vec2(20., y)).color(Color::WHITE),
                );
                y += t.dimensions(ctx).unwrap_or(graphics::Rect::default()).h;
            }
        }
        let fps_txt = Text::new(ctx.time.fps().to_string() + " fps");
        canvas.draw(
            &fps_txt,
            DrawParam::default().dest(vec2(20., y)).color(Color::WHITE),
        );

        canvas.finish(ctx)?;
        timer::yield_now();
        Ok(())
    }

    fn on_error(&mut self, _ctx: &mut Context, origin: event::ErrorOrigin, e: GameError) -> bool {
        match origin {
            event::ErrorOrigin::Draw => match e {
                GameError::LyonError(_s) => false,
                _ => true,
            },
            _ => true,
        }
    }

}

fn main() {
    let (ctx, ev_loop) = ContextBuilder::new("DD2258 Bonus Project", "Day")
        .window_setup(WindowSetup::default().title("DD2258 Bonus Project"))
        .window_mode(
            WindowMode::default()
                .dimensions(1280., 720.)
                .resizable(true),
        )
        .build()
        .expect("get context");
    let (tx, rx) = audio::audio_thread();
    let game = Game::new(16, 16, 20., tx, rx);
    event::run(ctx, ev_loop, game);
}
