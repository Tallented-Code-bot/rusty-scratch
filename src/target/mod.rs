#![allow(
    unused_imports,
    unused_mut,
    unused_variables,
    dead_code,
    non_snake_case
)]

extern crate rand;
use core::task::{RawWaker, RawWakerVTable, Waker};
use genawaiter::rc::{gen, Co};
use genawaiter::{yield_, GeneratorState};
use glutin_window::GlutinWindow as Window;
use graphics::types::{Matrix2d, Scalar};
use graphics::{rectangle, Context as DrawingContext, Image};
use opengl_graphics::{GlGraphics, OpenGL, Texture, TextureSettings};
use piston::event_loop::{EventLoop, EventSettings, Events};
use piston::input::{RenderArgs, RenderEvent, UpdateArgs, UpdateEvent};
use piston::window::WindowSettings;
use rand::Rng;
use std::fmt::Display;
use std::fs;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{
    boxed::Box,
    collections::HashMap,
    f32::consts::PI,
    future::{Future, IntoFuture},
    path::{Path, PathBuf},
    rc::Rc,
    sync::Mutex,
};

use blocks::*;

mod blocks {
    use rand::Rng;

    use super::{Sprite, Stage, Target, Value, Yield};
    use core::f32::consts::PI;
    use std::{rc::Rc, sync::Mutex};

    pub fn move_steps(sprite: Rc<Mutex<Sprite>>, steps: f32) {
        let mut sprite = sprite.lock().unwrap(); //shadow
        let radians = (90.0 - sprite.direction) * PI / 180.0;
        sprite.x += steps * radians.cos();
        sprite.y += steps * radians.sin();
    }

    pub fn go_to(sprite: Rc<Mutex<Sprite>>, x: f32, y: f32) {
        let mut sprite = sprite.lock().unwrap();
        sprite.x = x;
        sprite.y = y;
    }

    pub fn turn_right(sprite: Rc<Mutex<Sprite>>, degrees: f32) {
        let mut sprite = sprite.lock().unwrap();
        sprite.direction += degrees;
    }

    pub fn turn_left(sprite: Rc<Mutex<Sprite>>, degrees: f32) {
        let mut sprite = sprite.lock().unwrap();
        sprite.direction -= degrees;
    }

    pub fn point_in_direction(sprite: Rc<Mutex<Sprite>>, degrees: f32) {
        let mut sprite = sprite.lock().unwrap();
        sprite.direction = degrees;
    }

    pub fn set_x(sprite: Rc<Mutex<Sprite>>, x: f32) {
        let mut sprite = sprite.lock().unwrap();
        sprite.x = x;
    }
    pub fn set_y(sprite: Rc<Mutex<Sprite>>, y: f32) {
        let mut sprite = sprite.lock().unwrap();
        sprite.y = y;
    }

    pub fn change_x_by(sprite: Rc<Mutex<Sprite>>, x: f32) {
        let mut sprite = sprite.lock().unwrap();
        sprite.x += x;
    }
    pub fn change_y_by(sprite: Rc<Mutex<Sprite>>, y: f32) {
        let mut sprite = sprite.lock().unwrap();
        sprite.y += y;
    }

    /// Switch the backdrop.
    ///
    /// Value can either be a number or string. Numbers are treated as indexes,
    /// while strings are first treated as costume names, then
    /// previous/next/random costume, and finaly cast to numbers and tested as indexes.
    pub fn switch_backdrop(stage: Rc<Mutex<Stage>>, backdrop: Value) {
        let mut stage = stage.lock().unwrap();

        match backdrop {
            Value::Num(x) => stage.set_costume(x - 1.0),
            Value::String(name) => {
                let current_costume = stage.costume;

                let index = stage.costumes.iter().position(|c| c.name == name);
                if index.is_some() {
                    stage.set_costume(index.expect("Must be some") as f32);
                } else if name == "next backdrop" {
                    stage.set_costume(current_costume as f32 + 1.0);
                } else if name == "previous backdrop" {
                    stage.set_costume(current_costume as f32 - 1.0);
                } else if name == "random backdrop" {
                    if !stage.costumes.len() > 1 {
                        let mut newIndex: usize = stage.costume;
                        // generate a random costume, not including the current costume.
                        while newIndex == stage.costume {
                            newIndex =
                                generate_random(0.into(), stage.costumes.len().into()).into();
                        }
                        stage.set_costume(newIndex as f32);
                    }
                // try to cast the string into a number and use it as an index.
                } else if let Ok(index) = name.parse::<f32>() {
                    stage.set_costume(index);
                } else {
                    // do nothing
                }
            }
            Value::Bool(_) => unreachable!("Bool not supported"),
            Value::Null => unreachable!("Null not supported"),
        };
    }

    pub fn next_backdrop(stage: Rc<Mutex<Stage>>) {
        switch_backdrop(stage, Value::String("next backdrop".to_string()));
    }

    /// Switch the current costume.
    ///
    /// Number values are treated as indexes. String values are first treated as
    /// costume names and then are attempted to be parsed as indexes.
    pub fn switch_costume(sprite: Rc<Mutex<Sprite>>, costume: Value) {
        let mut sprite = sprite.lock().unwrap();
        match costume {
            Value::Num(index) => sprite.set_costume(index - 1.0),
            Value::String(name) => {
                let current_costume = sprite.costume as f32;
                if let Some(index) = sprite.costumes.iter().position(|c| c.name == name) {
                    sprite.set_costume(index as f32);
                } else if name == "next costume" {
                    sprite.set_costume(current_costume + 1.0);
                } else if name == "previous costume" {
                    sprite.set_costume(current_costume - 1.0);
                } else if let Ok(index) = name.parse::<f32>() {
                    sprite.set_costume(index);
                }
            }
            Value::Bool(_) => unreachable!("Bool not supported"),
            Value::Null => unreachable!("Null not supported"),
        }
    }

    pub fn next_costume(sprite: Rc<Mutex<Sprite>>) {
        switch_costume(sprite, Value::String("next costume".to_string()));
    }
    pub fn previous_costume(sprite: Rc<Mutex<Sprite>>) {
        switch_costume(sprite, Value::String("previous costume".to_string()));
    }

    pub fn set_size(sprite: Rc<Mutex<Sprite>>, size: Value) {
        let mut sprite = sprite.lock().unwrap();

        sprite.size = size.into();
    }

    pub fn change_size(sprite: Rc<Mutex<Sprite>>, change: Value) {
        let mut sprite = sprite.lock().unwrap();
        sprite.size += <Value as Into<f32>>::into(change);
    }

    pub fn show(sprite: Rc<Mutex<Sprite>>) {
        let mut sprite = sprite.lock().unwrap();
        sprite.visible = true;
    }

    pub fn hide(sprite: Rc<Mutex<Sprite>>) {
        let mut sprite = sprite.lock().unwrap();
        sprite.visible = false;
    }

    /// Get a variable from an id.
    pub fn get_variable(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        id: &str,
    ) -> Value {
        let stage = stage.lock().unwrap();
        match sprite {
            Some(sprite) => {
                // there is a sprite
                let sprite = sprite.lock().unwrap();

                // if the variable is on the sprite, get it.
                let mut var = sprite.variables.get(id);

                // otherwise, check the stage for the variable
                if var.is_none() {
                    var = stage.variables.get(id)
                }

                // return the variable's value
                return var.expect("no such variable found").1.clone();
            }
            None => {
                let var = stage.variables.get(id).unwrap();

                return var.1.clone();
            }
        }
    }

    /// Set the costume.
    ///
    /// DEPRECATED
    fn set_costume(object: Option<&mut Sprite>, globalCostume: &mut usize, costume: usize) {
        match object {
            Some(sprite) => sprite.costume = costume,
            None => *globalCostume = costume,
        }
    }

    /// Set the costume of a target.
    pub fn set_costume_better(object: &mut Target, costume: usize) {
        match &mut object.sprite {
            Some(x) => x.costume = costume,
            None => object.stage.costume = costume,
        }
    }

    pub fn say(speech: Value) {
        println!("{}", speech);
    }

    /// Join two strings.
    pub fn join(a: String, b: String) -> String {
        format!("{a}{b}")
    }

    /// Get the length of a string.
    pub fn length(s: String) -> usize {
        s.len()
    }

    /// Round a number.
    pub fn round(s: f32) -> f32 {
        s.round()
    }

    /// Wait until condition is true.
    pub async fn wait_until(condition: bool) {
        while !condition {
            Yield::Start.await;
        }
    }

    pub fn generate_random(from: Value, to: Value) -> Value {
        let from: u32 = from.into();
        let to: u32 = to.into();
        let r = rand::thread_rng().gen_range(from..=to);
        Value::Num(r as f32)
    }
}

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
#[derive(Clone, PartialEq, PartialOrd)]
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

/// A macro to convert Values to numbers
macro_rules! value_into {
    ($t:ident) => {
        impl Into<$t> for Value {
            fn into(self) -> $t {
                match self {
                    Self::String(_) => 0 as $t,
                    Self::Bool(x) => match x {
                        true => 1 as $t,
                        false => 0 as $t,
                    },
                    Self::Null => 0 as $t,
                    Self::Num(x) => {
                        if x.is_nan() {
                            return 0 as $t;
                        }
                        x as $t
                    }
                }
            }
        }
    };
}

value_into!(u32);
value_into!(usize);
value_into!(f32);
value_into!(i32);

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

pub struct SpriteBuilder {
    visible: bool,
    x: f32,
    y: f32,
    size: f32,
    direction: f32,
    draggable: bool,
    rotation_style: RotationStyle,
    name: String,
    variables: HashMap<String, (String, Value)>,
    costume: usize,
    costumes: Vec<Costume>,
}

impl SpriteBuilder {
    /// Create a default new SpriteBuilder.
    pub fn new(name: String) -> Self {
        Self {
            visible: true,
            x: 0.0,
            y: 0.0,
            size: 100.0,
            direction: 90.0,
            draggable: false,
            rotation_style: RotationStyle::AllAround,
            name,
            variables: HashMap::new(),
            costume: 0,
            costumes: Vec::new(),
        }
    }

    pub fn build(self) -> Sprite {
        Sprite {
            visible: self.visible,
            x: self.x,
            y: self.y,
            size: self.size,
            direction: self.direction,
            draggable: self.draggable,
            rotation_style: self.rotation_style,
            name: self.name,
            variables: self.variables,
            costume: self.costume,
            costumes: self.costumes,
        }
    }

    pub fn position(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn direction(mut self, direction: f32) -> Self {
        self.direction = direction;
        self
    }
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }
    pub fn rotation_style(mut self, rotation_style: RotationStyle) -> Self {
        self.rotation_style = rotation_style;
        self
    }
    pub fn add_variable(mut self, id: String, value: (String, Value)) -> Self {
        self.variables.insert(id, value);
        self
    }
    pub fn add_costume(mut self, costume: Costume) -> Self {
        self.costumes.push(costume);
        self
    }
    pub fn costume(mut self, costume: usize) -> Self {
        self.costume = costume;
        self
    }
}

pub struct Sprite {
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
    costumes: Vec<Costume>,
}

impl Sprite {
    fn set_costume(&mut self, mut index: f32) {
        // round the index to a whole number
        index = index.round();

        if index.is_infinite() || index.is_nan() {
            index = 0.0;
        }

        index = index.clamp(0.0, self.costumes.len() as f32 - 1.0); // make sure the index is valid

        self.costume = index as usize;
    }
}

/// A costume or backdrop
pub struct Costume {
    name: String,
    rotation_center_x: u32,
    rotation_center_y: u32,
    texture: Texture,
    image: Image,
}

impl Costume {
    fn new(name: String, path: PathBuf, scale: f32) -> Result<Self, &'static str> {
        let texture = get_texture_from_path(path.clone(), scale)?;
        //let name = path.file_stem().unwrap().to_str().unwrap().to_string();

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

/// Return an empty RawWaker which does nothing.
///
/// See https://os.phil-opp.com/async-await/#simple-executor
fn dummy_raw_waker() -> RawWaker {
    // A function that does nothing.
    fn no_op(_: *const ()) {}

    // the clone function just creates a new RawWaker. This is fine because the
    // RawWaker does nothing.
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }

    // Create a vtable with the clone function, and no_op for wake and drop.
    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);

    // Create a new RawWaker. The 0 is simply a null pointer that is unused.
    RawWaker::new(0 as *const (), vtable)
}

/// Return an empty waker.
fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}

async fn yield_fn() -> () {
    ()
}

/// This is a basic Future for other functions to yield. It returns Pending the
/// first time it is polled and Ready the second time.
enum Yield {
    Start,
    Middle,
    End,
}

impl Future for Yield {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match *self.as_mut() {
            Yield::Start => {
                *self = Yield::Middle;
                Poll::Pending
            }
            Yield::Middle => {
                *self = Yield::End;
                Poll::Ready(())
            }
            Yield::End => {
                panic!("poll called after Poll::Ready was returned");
            }
        }
    }
}

/// A thread object. This is a "virtual thread"; that is, it is not run in a
/// separate thread, but in an async loop.
struct Thread {
    /// The function to be called for the thread. This is a generator function
    /// that can have yields in it.
    function: Pin<Box<dyn Future<Output = ()>>>,
    /// The object that this thread works on. The number represents the index of
    /// the object in the program vector. If this is None, it represents the
    /// stage.
    // obj_index: Option<usize>,
    /// Whether or not the thread is complete.  If this is true, the thread
    /// is ok to be deleted.
    complete: bool,
    running: bool,
    /// When the thread should start
    start: StartType,
}

impl Thread {
    /// Create a new thread from a future.
    fn new(
        future: impl Future<Output = ()> + 'static, /*, obj_index: Option<usize>*/
        start: StartType,
    ) -> Thread {
        Thread {
            function: Box::pin(future),
            complete: false,
            running: false,
            start, // obj_index,
        }
    }

    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.function.as_mut().poll(context)
    }
}

/// The main project class.  This is in charge of running threads and
/// redrawing the screen.
struct Program {
    threads: Vec<Thread>,
    objects: Vec<Rc<Mutex<Sprite>>>,
    gl: GlGraphics,
    costumes: Vec<Costume>,
}

impl Program {
    /// Run 1 tick
    fn tick(&mut self /*, stage: Rc<Mutex<Stage>>*/) {
        // for (i, thread) in &mut self.threads.iter_mut().enumerate() {
        //     let returned = match thread.obj_index {
        //         Some(obj_index_unwrapped) => thread.function.resume_with(Target::new(
        //             self.objects[obj_index_unwrapped].clone(),
        //             stage.clone(),
        //         )),

        //         None => thread
        //             .function
        //             .resume_with(Target::new_stage(stage.clone())),
        //     };

        //     match returned {
        //         GeneratorState::Yielded(yielded_sprite) => {
        //             if let Some(zz) = yielded_sprite {
        //                 match zz.sprite {
        //                     Some(y) => {
        //                         if let Some(obj_index_unwrapped) = thread.obj_index {
        //                             self.objects[obj_index_unwrapped] = y;
        //                             *stage = zz.stage;
        //                         }
        //                     }
        //                     None => *stage = zz.stage,
        //                 }
        //             }
        //         }
        //         GeneratorState::Complete(x) => thread.complete = true,
        //     }
        // }

        // // remove all threads that are complete.
        // self.threads.retain(|x| !x.complete);
        for thread in &mut self.threads {
            // if the thread has not started yet, go to the next one.
            if !thread.running {
                continue;
            }
            let waker = dummy_waker();
            let mut context = Context::from_waker(&waker);
            match thread.poll(&mut context) {
                Poll::Pending => { /*The task is not done, so do nothing.*/ }
                Poll::Ready(()) => {
                    /*The task is done, so set it to complete.*/
                    thread.complete = true;
                }
            }
        }
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

    /// Simulate the flag being clicked by starting all threads with FlagClicked hat blocks.
    fn click_flag(&mut self) {
        for thread in &mut self.threads {
            if thread.start == StartType::FlagClicked {
                thread.running = true;
            }
        }
    }

    /// Renders a red square.
    fn render(&mut self, args: &RenderArgs, stage: Rc<Mutex<Stage>>) {
        use graphics::*;

        let stage = stage.lock().unwrap();

        const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
        const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
        const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

        let square = rectangle::square(00.0, 0.0, 100.0);
        // let rotation: Scalar = 2.0;
        let (x, y) = (args.window_size[0] / 2.0, args.window_size[1] / 2.0);

        self.gl.draw(args.viewport(), |c, gl| {
            // Clear the screen
            clear(BLACK, gl);

            // self.costumes[stage.costumes[stage.costume]].draw(c.transform, gl);
            stage.costumes[stage.costume].draw(c.transform, gl);

            // TODO sort by order
            for object in self.objects.clone() {
                let object = object.lock().unwrap();
                object.costumes[object.costume].draw(
                    c.transform
                        .trans(240.0, 180.0)
                        .trans(object.x.into(), <f32 as Into<f64>>::into(object.y * -1.0)),
                    gl,
                );
            }
        })

        // self.gl.draw(args.viewport(), |c, gl| {
        //     clear(BLACK, gl);
        // })
    }

    /// Add threads from a sprite to the program.
    /// This _moves_ the threads out of the sprite.
    fn add_threads(&mut self, mut threads: Vec<Thread>) {
        // self.threads.append(&mut sprite.blocks);
        self.threads.append(&mut threads)
    }

    fn add_object(&mut self, object: Rc<Mutex<Sprite>>) {
        self.objects.push(object);
    }

    /// Add a costume to the program
    fn add_costume_sprite(&mut self, costume: Costume, sprite: &mut Sprite) {
        todo!();
        // sprite.costumes.push(self.costumes.len());
        // self.costumes.push(costume);
    }

    fn add_costume_stage(&mut self, costume: Costume, stage: &mut Stage) {
        todo!();
        // stage.costumes.push(self.costumes.len());
        // self.costumes.push(costume);
    }

    /// Add a thread.
    fn add_thread(&mut self, thread: Thread) {
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

pub struct StageBuilder {
    tempo: i32,
    video_state: VideoState,
    video_transparency: i32,
    text_to_speech_language: String,
    variables: HashMap<String, (String, Value)>,
    costume: usize,
    costumes: Vec<Costume>,
}

impl StageBuilder {
    pub fn new() -> Self {
        Self {
            tempo: 60, //BPM
            video_state: VideoState::Off,
            video_transparency: 0,
            text_to_speech_language: "en".to_string(),
            variables: HashMap::new(),
            costume: 0,
            costumes: Vec::new(),
        }
    }
    pub fn build(self) -> Stage {
        Stage {
            tempo: self.tempo,
            video_state: self.video_state,
            video_transparency: self.video_transparency,
            text_to_speech_language: self.text_to_speech_language,
            variables: self.variables,
            costume: self.costume,
            costumes: self.costumes,
        }
    }
    pub fn tempo(mut self, tempo: i32) -> Self {
        self.tempo = tempo;
        self
    }
    pub fn video_state(mut self, video_state: VideoState) -> Self {
        self.video_state = video_state;
        self
    }
    pub fn video_transparency(mut self, transparency: i32) -> Self {
        self.video_transparency = transparency;
        self
    }
    pub fn add_variable(mut self, id: String, value: (String, Value)) -> Self {
        self.variables.insert(id, value);
        self
    }
    pub fn add_costume(mut self, costume: Costume) -> Self {
        self.costumes.push(costume);
        self
    }
    pub fn costume(mut self, costume: usize) -> Self {
        self.costume = costume;
        self
    }
    pub fn text_to_speech_language(mut self, ttsl: String) -> Self {
        self.text_to_speech_language = ttsl;
        self
    }
}

/// This is the stage object.
pub struct Stage {
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
    costumes: Vec<Costume>,
}

impl Stage {
    fn set_costume(&mut self, mut index: f32) {
        // round the index to a whole number
        index = index.round();

        if index.is_infinite() || index.is_nan() {
            index = 0.0;
        }

        index = index.clamp(0.0, self.costumes.len() as f32 - 1.0); // make sure the index is valid

        self.costume = index as usize;
    }
}

/// This is a target.
///
/// A target is a combination of a sprite(or not) and the stage. If the sprite
/// is absent(None), the target is assumed to be the stage.
pub struct Target {
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

/// The type of thread starter, such as flagClick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartType {
    FlagClicked,
    KeyPressed,
    SpriteClicked,
    BackdropSwitches,
    LoudnessGreater,
    RecieveMessage,
    StartAsClone,
    CustomBlock,
    NoStart,
}

impl Display for StartType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                StartType::FlagClicked => "StartType::FlagClicked",
                StartType::KeyPressed => "StartType::KeyPressed",
                StartType::SpriteClicked => "StartType::SpriteClicked",
                StartType::BackdropSwitches => "StartType::BackdropSwitches",
                StartType::LoudnessGreater => "StartType::LoudnessGreater",
                StartType::RecieveMessage => "StartType::RecieveMessage",
                StartType::StartAsClone => "StartType::StartAsClone",
                StartType::CustomBlock => "StartType::CustomBlock",
                StartType::NoStart => "StartType::NoStart",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitType {
    /// A specific time.
    Time(u32),
    CustomBlock,
    None,
}
