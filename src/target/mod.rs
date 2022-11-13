#![allow(
    unused_imports,
    unused_mut,
    unused_variables,
    dead_code,
    non_snake_case
)]

extern crate rand;
use genawaiter::rc::{gen, Co};
use genawaiter::{yield_, GeneratorState};
use glutin_window::GlutinWindow as Window;
use graphics::types::{Matrix2d, Scalar};
use graphics::{rectangle, Context, Image};
use opengl_graphics::{GlGraphics, OpenGL, Texture, TextureSettings};
use piston::event_loop::{EventLoop, EventSettings, Events};
use piston::input::{RenderArgs, RenderEvent, UpdateArgs, UpdateEvent};
use piston::window::WindowSettings;
use rand::Rng;
use std::fmt::Display;
use std::fs;
use std::pin::Pin;
use std::{
    boxed::Box,
    collections::HashMap,
    f32::consts::PI,
    future::{Future, IntoFuture},
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub enum RotationStyle {
    AllAround,
    LeftRight,
    DontRotate,
}

impl RotationStyle {
    pub fn from_str(input: &str) -> Result<RotationStyle, ()> {
        match input {
            "all around" => Ok(RotationStyle::AllAround),
            "left-right" => Ok(RotationStyle::LeftRight),
            "don't rotate" => Ok(RotationStyle::DontRotate),
            _ => Err(()),
        }
    }
    pub fn to_str(&self) -> &str {
        match self {
            RotationStyle::AllAround => "RotationStyle::AllAround",
            RotationStyle::LeftRight => "RotationStyle::LeftRight",
            RotationStyle::DontRotate => "RotationStyle::DontRotate",
        }
    }
}

#[derive(Clone)]
pub enum VideoState {
    On,
    Off,
    OnFlipped,
}

impl VideoState {
    pub fn from_str(input: &str) -> Result<VideoState, ()> {
        match input {
            "on" => Ok(VideoState::On),
            "off" => Ok(VideoState::Off),
            "on-flipped" => Ok(VideoState::OnFlipped),
            _ => Err(()),
        }
    }
    pub fn to_str(&self) -> &str {
        match self {
            VideoState::Off => "VideoState::Off",
            VideoState::On => "VideoState::On",
            VideoState::OnFlipped => "VideoState::OnFlipped",
        }
    }
}

/// A value, for a variable, list, or something else.
///
/// This can represent either a number or a string.
#[derive(Clone)]
pub enum Value {
    Num(f32),
    String(String),
    Bool(bool),
    Null,
}

impl Default for Value {
    fn default() -> Self {
        return Self::Num(0.0);
    }
}

impl Into<u32> for Value {
    fn into(self) -> u32 {
        match self {
            Self::String(_) => 0,
            Self::Bool(x) => x as u32,
            Self::Null => 0,
            Self::Num(x) => {
                if x.is_nan() {
                    return 0;
                }
                x as u32
            }
        }
    }
}

impl From<f32> for Value {
    fn from(item: f32) -> Self {
        Value::Num(item)
    }
}

impl From<u32> for Value {
    fn from(item: u32) -> Self {
        Value::Num(item as f32)
    }
}
impl From<i32> for Value {
    fn from(item: i32) -> Self {
        Value::Num(item as f32)
    }
}

impl From<usize> for Value {
    fn from(item: usize) -> Self {
        Value::Num(item as f32)
    }
}

impl From<String> for Value {
    fn from(item: String) -> Self {
        Value::String(item)
    }
}

impl From<&str> for Value {
    fn from(item: &str) -> Self {
        Value::String(item.to_string())
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // write!(f,"{}",self.va)
        match self {
            Self::Num(x) => {
                write!(f, "{}", x)
            }
            Self::String(x) => {
                write!(f, "{}", x)
            }
            Self::Null => {
                write!(f, "")
            }
            Self::Bool(x) => {
                write!(f, "{}", x)
            }
        }
    }
}

/// The target trait.
///
/// A target is anything that can run scratch code, meaning
/// either a sprite or the stage.
trait OldTarget {
    // fn new(&self) -> Self;
    fn set_x(&mut self, x: f32);
    fn set_y(&mut self, y: f32);
    fn get_x(&self) -> f32;
    fn get_y(&self) -> f32;
    fn go_to(&mut self, x: f32, y: f32) {
        self.set_x(x);
        self.set_y(y);
    }
    fn change_x_by(&mut self, x: f32) {
        self.set_x(self.get_x() + x);
    }
    fn change_y_by(&mut self, y: f32) {
        self.set_y(self.get_y() + y);
    }
    fn say(&self, text: String) {
        println!("{}", text);
    }
    fn direction(&self) -> f32;
    fn point_direction(&mut self, direction: f32);
    fn set_rotation_style(&mut self, style: RotationStyle);
    fn move_steps(&mut self, steps: f32) {
        let radians = self.direction() * PI / 180.0;
        let dx = steps * radians.cos();
        let dy = steps * radians.sin();
        self.go_to(self.get_x() + dx, self.get_y() + dy);
    }
    fn turn_left(&mut self, degrees: f32) {
        self.point_direction(self.direction() - degrees);
    }
    fn turn_right(&mut self, degrees: f32) {
        self.point_direction(self.direction() + degrees);
    }
    fn get_variable(&self, id: String) -> Option<&Value>;
    fn set_variable(&mut self, id: String, value: &mut Value);
    fn change_variable(&mut self, id: String, amount: f32) {
        // self.set_variable(id, self.get_variable(id).unwrap() + amount);
    }
    fn generate_random(&self, from: i32, to: i32) -> i32 {
        let mut rng = rand::thread_rng();
        return rng.gen_range(from..to);
    }
}

/// The stage.
///
/// Since the stage cannot run some blocks, some
/// of these definitions are empty.
// struct Stage {
//     /// The tempo, in BPM.
//     tempo: i32,
//     /// Determines if video is on or off.
//     videoState: VideoState,
//     /// The video transparency  Defaults to 50.  This has no effect if `videoState`
//     /// is off or the project does not use an exptension with video input.
//     videoTransparency: i32,
//     /// The text to speech language.  Defaults to the editor language.
//     textToSpeechLanguage: String,
//     variables: HashMap<String, Value>,
// }

// impl Target for Stage {
//     // fn new(&self) -> Self {
//     //     return Stage {
//     //         tempo: 60,
//     //         videoState: VideoState::Off,
//     //         videoTransparency: 50,
//     //         textToSpeechLangauge: "en".to_string(),
//     //     };
//     // }
//     fn set_x(&mut self, _x: f32) {}
//     fn set_y(&mut self, _y: f32) {}
//     /// Returns 0
//     fn get_x(&self) -> f32 {
//         return 0.0;
//     }
//     /// Returns 0
//     fn get_y(&self) -> f32 {
//         return 0.0;
//     }
//     fn direction(&self) -> f32 {
//         0.0
//     }
//     /// Does nothing; the stage cannot point in a direction.
//     fn point_direction(&mut self, direction: f32) {}
//     /// Does nothing; the stage cannot "say" anything.
//     fn say(&self, text: String) {}
//     fn set_rotation_style(&mut self, style: RotationStyle) {}
//     // fn set_variable(&mut self, id: String, value: &mut Value);
//     fn get_variable(&self, id: String) -> Option<&Value> {
//         return self.variables.get(&id);
//     }
//     fn set_variable(&mut self, id: String, value: &mut Value) {
//         if self.variables.contains_key(&id) {
//             let mut variable = self.variables.entry(id).or_default();
//             variable = value;
//         }
//     }
// }

// impl Target for Sprite {
//     // fn new(&self) -> Self {
//     //     return Sprite {
//     //         name: String::from("Sprite1"),
//     //         visible: true,
//     //         x: 0,
//     //         y: 0,
//     //         size: 100,
//     //         direction: 90,
//     //         draggable: false,
//     //         rotation_style: RotationStyle::AllAround,
//     //         blocks: Vec::new(),
//     //     };
//     // }
//     fn set_x(&mut self, x: f32) {
//         self.x = x;
//     }
//     fn set_y(&mut self, y: f32) {
//         self.y = y;
//     }
//     fn get_x(&self) -> f32 {
//         return self.x;
//     }
//     fn get_y(&self) -> f32 {
//         return self.y;
//     }
//     fn direction(&self) -> f32 {
//         return self.direction;
//     }
//     fn point_direction(&mut self, direction: f32) {
//         self.direction = direction;
//     }
//     fn set_rotation_style(&mut self, style: RotationStyle) {
//         self.rotation_style = style;
//     }
//     /// Get a variable
//     fn get_variable(&self, id: String) -> Option<&Value> {
//         let variable = self.variables.get(&id);

//         match variable {
//             Some(variable) => return Some(variable),
//             None => {}
//         }

//         let variable = self.stage.get_variable(id);

//         match variable {
//             Some(variable) => return Some(variable),
//             None => return None,
//         }
//     }
//     /// Set a variable
//     fn set_variable(&mut self, id: String, value: &mut Value) {
//         if self.variables.contains_key(&id) {
//             let mut variable = self.variables.entry(id).or_default();
//             variable = value;
//         }
//     }
// }

#[derive(Clone)]
struct Sprite {
    /// Whether the sprite is visibile.  Defaults to true.
    visible: bool,
    /// The x-coordinate.  Defaults to 0.
    x: f32,
    /// The y-coordinate.  Defaults to 0.
    y: f32,
    /// The sprite's size, as a percentage.  Defaults to 100.
    size: f32,
    /// The direction of the sprite in degrees clockwise from up.  Defaults to 90.
    direction: f32,
    /// Whether the sprite is draggable.  Defaults to false.
    draggable: bool,
    /// The rotation style.
    rotation_style: RotationStyle,
    /// The name of the sprite.
    name: String,
    /// The blocks in the sprite.
    /// This is currently only 1 stack of blocks,
    /// but this should change soon.
    // blocks: Vec<Thread<'a>>,
    /// A list of variables for the sprite
    variables: HashMap<String, (String, Value)>,
    /// The current costume. This is an index to the costumes attribute, which
    /// is itself an index!.
    costume: usize,
    /// A list of paths for costumes. Each item is an index to the program
    /// costume list.
    costumes: Vec<usize>,
}

/// A costume or backdrop
struct Costume {
    name: String,
    rotation_center_x: u32,
    rotation_center_y: u32,
    texture: Texture,
    image: Image,
}

impl Costume {
    fn new(path: PathBuf, scale: f32) -> Result<Self, &'static str> {
        let texture = get_texture_from_path(path.clone(), scale)?;
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();

        Ok(Self {
            name,
            rotation_center_x: 0,
            rotation_center_y: 0,
            texture,
            image: Image::new(),
        })
    }

    fn draw(&self, transform: Matrix2d, gl: &mut GlGraphics) {
        self.image.draw(
            &self.texture,
            &graphics::draw_state::DrawState::default(),
            transform,
            gl,
        )
    }
}

fn move_steps(object: &mut Target, steps: f32) {
    //unwrap option
    if let Some(uo) = &mut object.sprite {
        let radians = (90.0 - uo.direction) * PI / 180.0;
        uo.x += steps * radians.cos();
        uo.y += steps * radians.sin();
    }
}

fn go_to(object: &mut Target, x: f32, y: f32) {
    if let Some(uo) = &mut object.sprite {
        uo.x = x;
        uo.y = y;
    }
}

fn turn_right(object: &mut Target, degrees: f32) {
    if let Some(uo) = &mut object.sprite {
        uo.direction += degrees;
    }
}

fn turn_left(object: &mut Target, degrees: f32) {
    if let Some(uo) = &mut object.sprite {
        uo.direction -= degrees;
    }
}

fn point_in_direction(object: &mut Target, degrees: f32) {
    if let Some(uo) = &mut object.sprite {
        uo.direction = degrees;
    }
}

fn set_x(object: &mut Target, x: f32) {
    if let Some(uo) = &mut object.sprite {
        uo.x = x;
    }
}
fn set_y(object: &mut Target, y: f32) {
    if let Some(uo) = &mut object.sprite {
        uo.y = y;
    }
}

fn change_x_by(object: &mut Target, x: f32) {
    if let Some(uo) = &mut object.sprite {
        uo.x += x;
    }
}
fn change_y_by(object: &mut Target, y: f32) {
    if let Some(uo) = &mut object.sprite {
        uo.y += y;
    }
}

/// Get a variable from an id.
fn get_variable(object: &Target, id: &str) -> Value {
    match &object.sprite {
        Some(sprite) => {
            // there is a sprite

            // if the variable is on the sprite, get it.
            let mut var = sprite.variables.get(id);

            // otherwise, check the stage for the variable
            if var.is_none() {
                var = object.stage.variables.get(id)
            }

            // return the variable's value
            return var.unwrap().1.clone();
        }
        None => {
            let var = object.stage.variables.get(id).unwrap();

            return var.1.clone();
        }
    }
}

fn set_costume(object: Option<&mut Sprite>, globalCostume: &mut usize, costume: usize) {
    match object {
        Some(sprite) => sprite.costume = costume,
        None => *globalCostume = costume,
    }
}

/// Set the costume of a target.
fn set_costume_better(object: &mut Target, costume: usize) {
    match &mut object.sprite {
        Some(x) => x.costume = costume,
        None => object.stage.costume = costume,
    }
}

fn say(speech: Value) {
    println!("{}", speech);
}

// impl Sprite {}

// /// A stack of blocks
// struct Stack {
//     stack_type: StackType,
//     reference: String,
// }

// enum StackType {
//     FlagClicked,
// }

/// An emty type that implements future.
struct EmptyFuture {}

/// A future that always returns `()` immediately.
impl Future for EmptyFuture {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::task::Poll::Ready(())
    }
}

struct SpriteFuture {}

/// A thread object.
struct Thread<T: Future> {
    // function: fn(object: Option<&mut Sprite>),
    // function: genawaiter::rc::Gen<(), Option<Sprite>, Pin<Box<dyn Future<Output = ()>>>>,
    function: genawaiter::rc::Gen<Option<Target>, Target, T>,
    // function: genawaiter::rc::Gen<Option<Sprite>, Option<Sprite>, EmptyFuture>,
    // object: &mut Sprite,
    /// The object that this thread works on. The number represents the index of
    /// the object in the program vector. If this is None, it represents the
    /// stage.
    obj_index: Option<usize>,
    /// Whether or not the thread is complete.  If this is true, the thread
    /// is ok to be deleted.
    complete: bool,
}

/// The main project class.  This is in charge of running threads and
/// redrawing the screen.
struct Program<T: Future> {
    threads: Vec<Thread<T>>,
    objects: Vec<Sprite>,
    gl: GlGraphics,
    costumes: Vec<Costume>,
}

impl<T: Future> Program<T> {
    /// Run 1 tick
    fn tick(&mut self, stage: &mut Stage) {
        for (i, thread) in &mut self.threads.iter_mut().enumerate() {
            let returned = match thread.obj_index {
                Some(obj_index_unwrapped) => thread.function.resume_with(Target::new(
                    self.objects[obj_index_unwrapped].clone(),
                    stage.clone(),
                )),

                None => thread
                    .function
                    .resume_with(Target::new_stage(stage.clone())),
            };

            match returned {
                GeneratorState::Yielded(yielded_sprite) => {
                    if let Some(zz) = yielded_sprite {
                        match zz.sprite {
                            Some(y) => {
                                if let Some(obj_index_unwrapped) = thread.obj_index {
                                    self.objects[obj_index_unwrapped] = y;
                                    *stage = zz.stage;
                                }
                            }
                            None => *stage = zz.stage,
                        }
                    }
                }
                GeneratorState::Complete(x) => thread.complete = true,
            }
        }

        // remove all threads that are complete.
        self.threads.retain(|x| !x.complete);
    }

    fn new() -> Self {
        return Program {
            threads: Vec::new(),
            objects: Vec::new(),
            gl: GlGraphics::new(OpenGL::V3_2),
            costumes: Vec::new(),
        };
    }

    /// Renders a red square.
    fn render(&mut self, args: &RenderArgs, stage: &Stage) {
        use graphics::*;

        const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
        const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
        const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

        let square = rectangle::square(00.0, 0.0, 100.0);
        // let rotation: Scalar = 2.0;
        let (x, y) = (args.window_size[0] / 2.0, args.window_size[1] / 2.0);

        self.gl.draw(args.viewport(), |c, gl| {
            // Clear the screen
            clear(BLACK, gl);

            self.costumes[stage.costumes[stage.costume]].draw(c.transform, gl);

            // TODO sort by order
            for object in &self.objects {
                self.costumes[object.costumes[object.costume]].draw(
                    c.transform
                        .trans(240.0, 180.0)
                        .trans(object.x.into(), <f32 as Into<f64>>::into(object.y * -1.0)),
                    gl,
                );
            }
        })
    }

    /// Add threads from a sprite to the program.
    /// This _moves_ the threads out of the sprite.
    fn add_threads(&mut self, mut threads: Vec<Thread<T>>) {
        // self.threads.append(&mut sprite.blocks);
        self.threads.append(&mut threads)
    }

    fn add_object(&mut self, object: Sprite) {
        self.objects.push(object);
    }

    /// Add a costume to the program
    fn add_costume_sprite(&mut self, costume: Costume, sprite: &mut Sprite) {
        sprite.costumes.push(self.costumes.len());
        self.costumes.push(costume);
    }

    fn add_costume_stage(&mut self, costume: Costume, stage: &mut Stage) {
        stage.costumes.push(self.costumes.len());
        self.costumes.push(costume);
    }

    /// Add a thread.
    fn add_thread(&mut self, thread: Thread<T>) {
        self.threads.push(thread);
    }
}

fn get_texture_from_path(
    path: PathBuf,
    scale: f32,
) -> Result<opengl_graphics::Texture, &'static str> {
    use opengl_graphics::{CreateTexture, Format, Texture, TextureSettings};
    use resvg::tiny_skia::{Pixmap, Transform};
    use resvg::usvg::{FitTo, Options, Tree};

    let tree = Tree::from_str(
        &fs::read_to_string(path).or(Err("Cannot read file"))?,
        &Options::default().to_ref(),
    )
    .or(Err("Not a readable svg file"))?;

    let fit_to = FitTo::Zoom(scale);
    let transform = Transform::default();
    let size = fit_to.fit_to(tree.size.to_screen_size()).unwrap();
    let mut pixmap = Pixmap::new(size.width(), size.height()).ok_or("Could not create pixmap")?;
    let pixmapmut = pixmap.as_mut();

    resvg::render(&tree, fit_to, transform, pixmapmut);

    let texture = Texture::create(
        &mut (),
        Format::Rgba8,
        pixmap.data(),
        [pixmap.width(), pixmap.height()],
        &TextureSettings::new(),
    )
    .or(Err("Could not create texture"));

    return texture;
}

/// This is the stage object.
#[derive(Clone)]
struct Stage {
    /// The tempo, in BPM.
    tempo: i32,
    /// Determines if video is on or off.
    video_state: VideoState,
    /// The video transparency  Defaults to 50.  This has no effect if `videoState`
    /// is off or the project does not use an exptension with video input.
    video_transparency: i32,
    /// The text to speech language.  Defaults to the editor language.
    text_to_speech_language: String,
    variables: HashMap<String, (String, Value)>,
    /// The current costume.  An index to the stage costumes list.
    costume: usize,
    /// The costumes in the stage. These are indexes to the list of costumes in
    /// the program.
    costumes: Vec<usize>,
}

/// This is a target.
///
/// A target is a combination of a sprite(or not) and the stage. If the sprite
/// is absent(None), the target is assumed to be the stage.
struct Target {
    /// The stage.
    ///
    /// This is an owned variable, meaning it is a copy of the real stage. The
    /// real stage should be set to this copy when the target is returned from a
    /// thread.
    stage: Stage,
    sprite: Option<Sprite>,
}

impl Target {
    /// Create a new sprite target.
    fn new(sprite: Sprite, stage: Stage) -> Self {
        Self {
            sprite: Some(sprite),
            stage,
        }
    }

    /// Create a new stage target.
    fn new_stage(stage: Stage) -> Self {
        Self {
            sprite: None,
            stage,
        }
    }
}

// enum Target {
//     Sprite {
//         /// Whether the sprite is visibile.  Defaults to true.
//         visible: bool,
//         /// The x-coordinate.  Defaults to 0.
//         x: f32,
//         /// The y-coordinate.  Defaults to 0.
//         y: f32,
//         /// The sprite's size, as a percentage.  Defaults to 100.
//         size: f32,
//         /// The direction of the sprite in degrees clockwise from up.  Defaults to 90.
//         direction: f32,
//         /// Whether the sprite is draggable.  Defaults to false.
//         draggable: bool,
//         /// The rotation style.
//         rotation_style: RotationStyle,
//         /// The name of the sprite.
//         name: String,
//         /// The blocks in the sprite.
//         /// This is currently only 1 stack of blocks,
//         /// but this should change soon.
//         blocks: Vec<Thread>,
//         /// A list of variables for the sprite
//         variables: HashMap<String, Value>,
//     },
//     Stage {
//         /// The tempo, in BPM.
//         tempo: i32,
//         /// Determines if video is on or off.
//         video_state: VideoState,
//         /// The video transparency  Defaults to 50.  This has no effect if `videoState`
//         /// is off or the project does not use an exptension with video input.
//         video_transparency: i32,
//         /// The text to speech language.  Defaults to the editor language.
//         text_to_speech_language: String,
//         variables: HashMap<String, Value>,
//     },
// }

// impl Target {
//     // fn new(&self) -> Self;
//     fn set_x(&mut self, xin: f32) {
//         match self {
//             Target::Sprite { ref mut x, .. } => *x = xin,
//             Target::Stage { .. } => {}
//         }
//     }
//     fn set_y(&mut self, yin: f32) {
//         match self {
//             Target::Sprite { ref mut y, .. } => *y = yin,
//             Target::Stage { .. } => {}
//         }
//     }
//     fn get_x(&self) -> f32 {
//         // TODO document case for stage.
//         match self {
//             Target::Sprite { x, .. } => *x,
//             _ => 0.0,
//         }
//     }
//     fn get_y(&self) -> f32 {
//         match self {
//             Target::Sprite { y, .. } => *y,
//             _ => 0.0,
//         }
//     }
//     fn go_to(&mut self, x: f32, y: f32) {
//         self.set_x(x);
//         self.set_y(y);
//     }
//     fn change_x_by(&mut self, x: f32) {
//         self.set_x(self.get_x() + x);
//     }
//     fn change_y_by(&mut self, y: f32) {
//         self.set_y(self.get_y() + y);
//     }
//     fn say(&self, text: String) {
//         println!("{}", text);
//     }
//     fn direction(&self) -> f32 {
//         match self {
//             Self::Sprite { direction, .. } => *direction,
//             _ => 0.0,
//         }
//     }
//     fn point_direction(&mut self, direction_in: f32) {
//         match self {
//             Self::Sprite { direction, .. } => *direction = direction_in,
//             Self::Stage { .. } => {}
//         }
//     }
//     fn set_rotation_style(&mut self, style: RotationStyle) {
//         match self {
//             Self::Sprite { rotation_style, .. } => *rotation_style = style,
//             Self::Stage { .. } => {}
//         }
//     }
//     fn move_steps(&mut self, steps: f32) {
//         let radians = self.direction() * PI / 180.0;
//         let dx = steps * radians.cos();
//         let dy = steps * radians.sin();
//         self.go_to(self.get_x() + dx, self.get_y() + dy);
//     }
//     fn turn_left(&mut self, degrees: f32) {
//         self.point_direction(self.direction() - degrees);
//     }
//     fn turn_right(&mut self, degrees: f32) {
//         self.point_direction(self.direction() + degrees);
//     }
//     fn get_variable(&self, id: String) -> Option<&Value> {
//         // TODO implement
//         unimplemented!();
//         //None
//     }
//     fn set_variable(&mut self, id: String, value: &mut Value) {}
//     fn change_variable(&mut self, id: String, amount: f32) {
//         // self.set_variable(id, self.get_variable(id).unwrap() + amount);
//     }
//     fn generate_random(&self, from: i32, to: i32) -> i32 {
//         let mut rng = rand::thread_rng();
//         return rng.gen_range(from..to);
//     }
// }
