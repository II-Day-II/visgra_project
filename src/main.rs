use ggez::{
    conf::{WindowMode, WindowSetup},
    Context, 
    ContextBuilder, 
    GameResult,
    GameError, 
    graphics::{
        self, 
        Color, DrawParam, Text,
    },
    event::{
        self, 
        EventHandler,
        KeyCode,
    }, timer,
    glam::{vec2, Vec2, ivec2, IVec2}
};
use std::{f32::{consts::PI}, str::from_utf8};


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
            },
            Direction::Back => {
                self.pos -= cossin * change;
            },
            Direction::Left => {
                self.pos += psinmcos * change;
            },
            Direction::Right => {
                self.pos -= psinmcos * change;
            },
        }
    }
    fn rotate(&mut self, dir: Direction, dt: f32) {
        match dir {
            Direction::Left => {
                self.angle -= self.speed * 0.75 * dt;
            },
            Direction::Right => {
                self.angle += self.speed * 0.75 * dt;
            },
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
        }
        else if a < 0 {
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
}

impl Game {
    fn new(width: i32, height: i32, render_distance: f32) -> Self {
        Self {
            size: ivec2(width, height),
            render_distance,
            player: Player::new(14., 5.),
            //map: MAP.chars().collect(),
            map: MAP.as_bytes().iter().map(|x| *x).collect(),
            draw_map: false,
        }
    }
}

impl EventHandler for Game {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        // store last player location on map
        let old_pos = self.player.pos;
        let delta_time = timer::delta(ctx).as_secs_f32();
        self.player.handle_input(delta_time);
        
        let mut new_pos = self.player.pos;
        if self.map[(new_pos.y as i32 * self.size.x + new_pos.x as i32) as usize] == b'#' {
            self.player.pos = old_pos;
            new_pos = self.player.pos;
        }

        if old_pos != new_pos {
            self.map[(old_pos.y as i32 * self.size.x + old_pos.x as i32) as usize] = b'.';
            self.map[(new_pos.y as i32 * self.size.x + new_pos.x as i32) as usize] = b'P';
        } 
        Ok(())
    }

    fn key_down_event(&mut self, _ctx: &mut Context, keycode: event::KeyCode, _keymods: event::KeyMods, _repeat: bool) {
        match keycode {
            KeyCode::A => {
                // left
                self.player.controller.x = -1;
            },
            KeyCode::W => {
                //forw
                self.player.controller.y = 1;
            },
            KeyCode::D => {
                //right
                self.player.controller.x = 1;
            },
            KeyCode::S => {
                //back
                self.player.controller.y = -1;
            },
            KeyCode::Left => {
                //rot left
                self.player.controller.a = -1;
            },
            KeyCode::Right => {
                //rot right
                self.player.controller.a = 1;
            },
            _ => {}
        }
    }
    
    fn key_up_event(&mut self, ctx: &mut Context, keycode: KeyCode, _keymods: event::KeyMods) {
        match keycode {
            KeyCode::Escape => event::quit(ctx),
            KeyCode::A | KeyCode::D => self.player.controller.x = 0,
            KeyCode::W | KeyCode::S => self.player.controller.y = 0,
            KeyCode::Left | KeyCode::Right => self.player.controller.a = 0,
            KeyCode::M => self.draw_map = !self.draw_map,
            _ => {}
        }
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, Color::BLACK);
        let (screen_width, screen_height) = graphics::drawable_size(ctx);
        let mut mb = graphics::MeshBuilder::new();
        for x in 0..screen_width as u32 {
            // raycasting 
            let ray_angle = self.player.angle - self.player.fov/2. + (x as f32 / screen_width) * self.player.fov;
            let ray_direction = vec2(ray_angle.cos(), ray_angle.sin()); 
            let step_size = vec2(
                (1. + (ray_direction.y / ray_direction.x) * (ray_direction.y / ray_direction.x)).sqrt(),
                (1. + (ray_direction.x / ray_direction.y) * (ray_direction.x / ray_direction.y)).sqrt() 
            );
            let mut map_check = self.player.pos.as_ivec2();
            let step = ivec2(ray_direction.x.signum() as i32, ray_direction.y.signum() as i32);
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
                }
            );
            let mut tile_found = false;
            let mut distance = 0.0;
            while !tile_found && distance < self.render_distance {
                // walk shortest path
                if ray_length1d.x < ray_length1d.y {
                    map_check.x += step.x;
                    distance = ray_length1d.x;
                    ray_length1d.x += step_size.x;
                }
                else {
                    map_check.y += step.y;
                    distance = ray_length1d.y;
                    ray_length1d.y += step_size.y;
                }
                // test map to see where/if we hit
                if map_check.x >= 0 && map_check.x < self.size.x && map_check.y >= 0 && map_check.y < self.size.y {
                    tile_found = self.map[(map_check.y * self.size.x + map_check.x) as usize] == b'#';
                }
            }
            let _tile_intersection = if tile_found {
                self.player.pos + ray_direction * distance
            }
            else {
                vec2(self.render_distance + 1e+10, self.render_distance + 1e+10)
            };
            let sh = screen_height;
            let ceil_distance = (sh / 2.) - sh / distance;
            let floor_distance = sh - ceil_distance;
            let c = 1. - distance / self.render_distance;
            let color = Color::new(c, c, c, c);

            mb.line(&[vec2(x as f32, ceil_distance), vec2(x as f32, floor_distance)], 1.0, color)?;
            
            
        }
        let mesh = mb.build(ctx)?;
        graphics::draw(ctx, &mesh, (vec2(0., 0.),))?;
        
        let mut y = 20.0;
        if self.draw_map {
            for dy in 0..self.size.y {
                let t = Text::new(from_utf8(&self.map[(dy * self.size.x) as usize..(dy * self.size.x + self.size.x) as usize]).unwrap());
                graphics::queue_text(
                    ctx, 
                    &t, 
                    vec2(20.0, y), 
                    Some(Color::WHITE)
                );
                y += t.height(ctx);
            }
        }
        graphics::queue_text(ctx, &Text::new(timer::fps(ctx).to_string() + " fps"), vec2(20., y), Some(Color::WHITE));
        
        graphics::draw_queued_text(ctx, DrawParam::default(), None, graphics::FilterMode::Linear)?;
        
        graphics::present(ctx)?;
        timer::yield_now();
        Ok(())
    }

    fn resize_event(&mut self, ctx: &mut Context, width: f32, height: f32) {
        graphics::set_screen_coordinates(ctx, graphics::Rect::new(0.0, 0.0, width, height))
            .unwrap();
    }
    fn on_error(&mut self, _ctx: &mut Context, origin: event::ErrorOrigin, e: GameError) -> bool {
        match origin {
            event::ErrorOrigin::Draw => {
                match e {
                    GameError::LyonError(_s) => {
                        false
                    }
                    _ => {true}
                }
            }
            _ => {true}
        }
    }
}

fn main() {
    let (ctx, ev_loop) = ContextBuilder::new("ggez testing :)", "Day")
        .window_setup(WindowSetup::default().title("ggez testing :)"))
        .window_mode(WindowMode::default().dimensions(1280., 720.).resizable(true))
        .build().expect("get context");
    let game = Game::new(16, 16, 20.);
    event::run(ctx, ev_loop, game);
}

