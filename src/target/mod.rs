#![allow(
    unused_imports,
    unused_mut,
    unused_variables,
    dead_code,
    non_snake_case
)]

extern crate rand;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use uuid::Uuid;
use core::task::{RawWaker, RawWakerVTable, Waker};
use genawaiter::rc::{gen, Co};
use genawaiter::{yield_, GeneratorState};
use glutin_window::GlutinWindow;
use graphics::types::{Matrix2d, Scalar};
use graphics::{rectangle, Context as DrawingContext, Image};
use opengl_graphics::{GlGraphics, OpenGL, Texture, TextureSettings};
use piston::event_loop::{EventLoop, EventSettings, Events};
use piston::input::{Button, Key, RenderArgs, RenderEvent, UpdateArgs, UpdateEvent};
use piston::window::WindowSettings;
use piston::Window;
use piston::{MouseButton, MouseCursorEvent, PressEvent, ReleaseEvent, Size as WindowSize};
use rand::Rng;
use std::collections::VecDeque;
use std::fmt::Display;
use std::fs;
use std::io;
use std::ops::Index;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{
    boxed::Box,
    collections::{HashMap, HashSet},
    f32::consts::PI,
    future::{Future, IntoFuture},
    path::{Path, PathBuf},
    rc::Rc,
    sync::Mutex,
    time::{Duration, Instant},
};

use blocks::*;

// set the width and height of the stage here
const SCRATCH_WIDTH: Value = Value::Num(480.0);
const SCRATCH_HALF_WIDTH: Value = Value::Num(240.0);
const SCRATCH_HEIGHT: Value = Value::Num(360.0);
const SCRATCH_HALF_HEIGHT: Value = Value::Num(180.0);

const LIST_ITEM_LIMIT: Value = Value::Num(20000.0); // TODO check this

mod blocks {
    use super::{
        toNumber, Stamp, StartType, LIST_ITEM_LIMIT, SCRATCH_HALF_HEIGHT, SCRATCH_HALF_WIDTH,
    };
    use super::{Keyboard, Sprite, Stage, Value, Yield};
    use chrono::TimeZone;
    use core::f32::consts::PI;
    use rand::Rng;
    use std::io;
    use std::{
        f32::consts::E,
        rc::Rc,
        sync::{Mutex, MutexGuard},
    };

    pub fn move_steps(sprite: Rc<Mutex<Sprite>>, steps: Value) {
        let steps = toNumber(&steps);

        let mut sprite = sprite.lock().unwrap(); //shadow
        let radians = (90.0 - sprite.direction) * PI / 180.0;
        sprite.x += steps * radians.cos();
        sprite.y += steps * radians.sin();
    }

    pub fn go_to_xy(sprite: Rc<Mutex<Sprite>>, x: Value, y: Value) {
        let mut sprite = sprite.lock().unwrap();

        let x = toNumber(&x);
        let y = toNumber(&y);

        sprite.x = x;
        sprite.y = y;
    }

    pub fn get_target_xy(targetName: Value, stage: Rc<Mutex<Stage>>) -> Option<(Value, Value)> {
        let (targetX, targetY);
        let stage = stage.lock().unwrap();
        if targetName == Value::String(String::from("_mouse_")) {
            targetX = stage.mouse.x();
            targetY = stage.mouse.y();
        } else if targetName == Value::String(String::from("_random_")) {
            targetX = generate_random(-SCRATCH_HALF_WIDTH, SCRATCH_HALF_WIDTH);
            targetY = generate_random(-SCRATCH_HALF_HEIGHT, SCRATCH_HALF_HEIGHT);
        } else {
            let name: String = targetName.into();

            if let Some(sprite) = stage.get_sprite(name) {
                let sprite = sprite.lock().unwrap();
                targetX = Value::Num(sprite.x);
                targetY = Value::Num(sprite.y);
            } else {
                return None;
            }
        }
        Some((targetX, targetY))
    }

    pub fn go_to(sprite: Rc<Mutex<Sprite>>, stage: Rc<Mutex<Stage>>, to: Value) {
        let mut sprite = sprite.lock().unwrap();
        if let Some((x, y)) = get_target_xy(to, stage) {
            sprite.x = x.into();
            sprite.y = y.into();
        }
    }

    pub fn turn_right(sprite: Rc<Mutex<Sprite>>, degrees: Value) {
        let mut sprite = sprite.lock().unwrap();
        sprite.direction += toNumber(&degrees);
    }

    pub fn turn_left(sprite: Rc<Mutex<Sprite>>, degrees: Value) {
        let mut sprite = sprite.lock().unwrap();
        sprite.direction -= toNumber(&degrees);
    }

    pub fn point_in_direction(sprite: Rc<Mutex<Sprite>>, degrees: f32) {
        let mut sprite = sprite.lock().unwrap();
        sprite.direction = degrees;
    }

    pub fn set_x(sprite: Rc<Mutex<Sprite>>, x: Value) {
        let mut sprite = sprite.lock().unwrap();
        let dx = toNumber(&x);
        sprite.x = dx;
    }
    pub fn set_y(sprite: Rc<Mutex<Sprite>>, y: Value) {
        let mut sprite = sprite.lock().unwrap();
        let dy = toNumber(&y);
        sprite.y = dy;
    }

    pub fn get_x(sprite: Rc<Mutex<Sprite>>) -> Value {
        Value::Num(sprite.lock().unwrap().x)
    }

    pub fn get_y(sprite: Rc<Mutex<Sprite>>) -> Value {
        Value::Num(sprite.lock().unwrap().y)
    }

    pub fn change_x_by(sprite: Rc<Mutex<Sprite>>, x: Value) {
        let mut sprite = sprite.lock().unwrap();
        let dx = toNumber(&x);
        sprite.x += dx;
    }
    pub fn change_y_by(sprite: Rc<Mutex<Sprite>>, y: Value) {
        let mut sprite = sprite.lock().unwrap();
        let dy = toNumber(&y);
        sprite.y += dy;
    }

    pub fn set_rotation_style(sprite: Rc<Mutex<Sprite>>, style: String) {
        use super::RotationStyle;
        let mut sprite = sprite.lock().unwrap();

        sprite.rotation_style = match &*style {
            "left-right" => RotationStyle::LeftRight,
            "don\'t rotate" => RotationStyle::DontRotate,
            "all around" => RotationStyle::AllAround,
            _ => unreachable!("Unknown rotation option"),
        }
    }

    /// Switch the backdrop.
    ///
    /// Value can either be a number or string. Numbers are treated as indexes,
    /// while strings are first treated as costume names, then
    /// previous/next/random costume, and finally cast to numbers and tested as indexes.
    pub fn switch_backdrop(stage: Rc<Mutex<Stage>>, backdrop: Value) {
        let mut stage = stage.lock().unwrap();

        match backdrop {
            Value::Num(x) => stage.set_costume(x - 1.0),
            Value::String(name) => {
                let current_costume = stage.costume;

                let index = stage.costumes.iter().position(|c| c.name == name);
                if let Some(i) = index {
                    stage.set_costume(i as f32);
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
                var.expect("no such variable found").1.clone()
            }
            None => {
                let var = stage.variables.get(id).unwrap();

                var.1.clone()
            }
        }
    }

    pub fn set_variable(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        (name, id): (String, String),
        value: Value,
    ) {
        let mut stage = stage.lock().unwrap();
        match sprite {
            Some(sprite) => {
                let mut sprite = sprite.lock().unwrap();
                if let Some(variable) = sprite.variables.get_mut(&id) {
                    variable.1 = value;
                } else if let Some(variable) = stage.variables.get_mut(&id) {
                    variable.1 = value;
                } else {
                    sprite.variables.insert(id, (name, value));
                }
            }
            None => {
                if let Some(variable) = stage.variables.get_mut(&id) {
                    variable.1 = value;
                } else {
                    stage.variables.insert(id, (name, value));
                }
            }
        }
    }

    pub fn change_variable(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        (name, id): (String, String),
        value: Value,
    ) {
        set_variable(
            sprite.clone(),
            stage.clone(),
            (name, id.clone()),
            get_variable(sprite, stage, &id) + value,
        );
    }

    /// Get a list.
    fn get_list(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        (name, id): (String, String),
    ) -> Vec<Value> {
        let mut stage = stage.lock().unwrap();
        let list;
        match sprite {
            Some(sprite) => {
                let mut sprite = sprite.lock().unwrap();
                if let Some(l) = sprite.lists.get(&id) {
                    list = l.clone();
                } else if let Some(l) = stage.lists.get(&id) {
                    list = l.clone();
                } else {
                    stage.lists.insert(id.clone(), (name, Vec::new()));
                    list = stage.lists.get(&id).expect("Just created id").clone();
                }
            }
            None => {
                if let Some(l) = stage.lists.get(&id) {
                    list = l.clone();
                } else {
                    stage.lists.insert(id.clone(), (name, Vec::new()));
                    list = stage.lists.get(&id).expect("Just created id").clone();
                }
            }
        }

        list.1
    }

    fn replace_list(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        to_replace: Vec<Value>,
        (name, id): (String, String),
    ) {
        let mut stage = stage.lock().unwrap();

        match sprite {
            Some(sprite) => {
                let mut sprite = sprite.lock().unwrap();
                if let Some(l) = sprite.lists.get_mut(&id) {
                    l.1 = to_replace;
                } else if let Some(l) = stage.lists.get_mut(&id) {
                    l.1 = to_replace;
                } else {
                    stage.lists.insert(id.clone(), (name, Vec::new()));
                    stage.lists.get_mut(&id).expect("Just created id").1 = to_replace;
                }
            }
            None => {
                if let Some(l) = stage.lists.get_mut(&id) {
                    l.1 = to_replace;
                } else {
                    stage.lists.insert(id.clone(), (name, Vec::new()));
                    stage.lists.get_mut(&id).expect("Just created id").1 = to_replace;
                }
            }
        }
    }

    /// Get a list in a single value, suitable for printing.
    pub fn get_list_contents(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        (name, id): (String, String),
    ) -> Value {
        let list = get_list(sprite, stage, (name, id));

        // check if the list is all single letters.  If it is, return
        // the list without separators.  If it is not, join them with a space.
        let mut all_single_letters = true;
        for i in &list {
            if let Value::String(x) = i {
                if x.len() > 1 {
                    all_single_letters = false;
                    break;
                }
            }
        }

        if all_single_letters {
            // join the list items together with no space
            Value::String(
                list.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>()
                    .join(""),
            )
        } else {
            // join the list items together with a space in between
            Value::String(
                list.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>()
                    .join(" "),
            )
        }
    }

    pub fn get_item_of_list(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        item: Value,
        (name, id): (String, String),
    ) -> Value {
        let list = get_list(sprite, stage, (name, id));

        let index = item.to_list_index(list.len(), false);

        match index {
            Ok(i) => list[i - 1].clone(),
            Err(_) => Value::String(String::from("")),
        }
    }

    pub fn length_of_list(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        (name, id): (String, String),
    ) -> Value {
        let list = get_list(sprite, stage, (name, id));
        Value::Num(list.len() as f32)
    }

    pub fn get_item_num_in_list(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        item: Value,
        (name, id): (String, String),
    ) -> Value {
        let list = get_list(sprite, stage, (name, id));

        for (i, value) in list.iter().enumerate() {
            if *value == item {
                return Value::Num((i - 1) as f32);
            }
        }
        Value::Num(0 as f32)
    }

    pub fn list_contains_item(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        (name, id): (String, String),
        search: Value,
    ) -> Value {
        let list = get_list(sprite, stage, (name, id));

        for item in list {
            if item == search {
                return Value::Bool(true);
            }
        }
        Value::Bool(false)
    }

    /// Get a mutable reference to a list in the sprite or stage.
    /*fn get_mut_list<'a>(sprite:Option<&'a mut MutexGuard<'a, Sprite>>,stage:&'a mut MutexGuard<'a, Stage>,(name,id):(String,String))-> &'a mut (String,Vec<Value>){
        match sprite{
            // if a sprite is specified:
            Some(mut sprite) => {
                if let Some(l) = sprite.lists.get_mut(&id){
                    // if the list exists in the sprite, return it.
                    return l;
                }else if let Some(l) =stage.lists.get_mut(&id){
                    // otherwise, look in the stage, and if the stage has it, return it.
                    return l;
                }else{
                    // else create a new list in the sprite, and return it.
                    stage.lists.insert(id.clone(),(name,Vec::new()));
                    return stage.lists.get_mut(&id).expect("Just created id");
                }
            },
            None => {
                // if a sprite is not specified:
                if let Some(l) = stage.lists.get_mut(&id){
                    // check if the list exists in the stage, and if so, return it.
                    return l;
                }else{
                    // otherwise create a new list in the stage and return it.
                    stage.lists.insert(id.clone(),(name,Vec::new()));
                    return stage.lists.get_mut(&id).expect("Just created id");
                }
            },
        }

    }*/

    pub fn add_to_list(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        to_add: Value,
        (name, id): (String, String),
    ) {
        let mut stage = stage.lock().unwrap();
        match sprite {
            Some(sprite) => {
                let mut sprite = sprite.lock().unwrap();
                if let Some(l) = sprite.lists.get_mut(&id) {
                    l.1.push(to_add);
                } else if let Some(l) = stage.lists.get_mut(&id) {
                    l.1.push(to_add);
                } else {
                    stage.lists.insert(id.clone(), (name, Vec::new()));
                    let l = stage.lists.get_mut(&id).expect("Just created id");
                    l.1.push(to_add);
                }
            }
            None => {
                if let Some(l) = stage.lists.get_mut(&id) {
                    l.1.push(to_add);
                } else {
                    stage.lists.insert(id.clone(), (name, Vec::new()));
                    let l = stage.lists.get_mut(&id).expect("Just created id");
                    l.1.push(to_add);
                }
            }
        }
    }

    /// Delete an item from a list
    pub fn delete_from_list(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        item: Value,
        (name, id): (String, String),
    ) {
        let mut list = get_list(sprite.clone(), stage.clone(), (name.clone(), id.clone()));
        let index = item.to_list_index(list.len(), true);

        // TODO handle deleting all of the list. (This is not supported yet in the .to_list_index() function).

        if let Ok(x) = index {
            list.remove(x - 1);
            replace_list(sprite, stage, list, (name, id));
        }
    }

    pub fn delete_all_of_list(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        (name, id): (String, String),
    ) {
        replace_list(sprite, stage, Vec::new(), (name, id));
    }

    pub fn insert_item_in_list(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        item: Value,
        position: Value,
        (name, id): (String, String),
    ) {
        let mut list = get_list(sprite.clone(), stage.clone(), (name.clone(), id.clone()));
        let index = position.to_list_index(list.len() + 1, false);

        match index {
            Ok(x) => {
                if x > LIST_ITEM_LIMIT.into() {
                    return;
                }

                list.insert(x - 1, item);

                if list.len() > LIST_ITEM_LIMIT.into() {
                    list.pop();
                }

                replace_list(sprite, stage, list, (name, id));
            }
            Err(_) => {}
        }
    }

    pub fn replace_item_in_list(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        item: Value,
        position: Value,
        (name, id): (String, String),
    ) {
        let mut list = get_list(sprite.clone(), stage.clone(), (name.clone(), id.clone()));
        let index = position.to_list_index(list.len(), false);

        match index {
            Ok(x) => {
                list[x - 1] = item;
                replace_list(sprite, stage, list, (name, id));
            }
            Err(_) => {}
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

    pub fn say(speech: Value) {
        println!("{}", speech);
    }

    /// Join two strings.
    pub fn join(a: Value, b: Value) -> Value {
        Value::String(format!("{}{}", a, b))
    }

    pub fn letter_of(letter: Value, string: Value) -> Value {
        let mut index: usize = letter.into();
        index -= 1;
        let s: String = string.into();

        Value::from(match s.chars().nth(index) {
            Some(x) => x.to_string(),
            None => String::new(),
        })
    }

    pub fn length(string: Value) -> Value {
        Value::Num(string.to_string().len() as f32)
    }

    pub fn contains(string1: Value, string2: Value) -> Value {
        Value::Bool(
            string1
                .to_string()
                .to_lowercase()
                .contains(string2.to_string().to_lowercase().as_str()),
        )
    }

    pub fn round(num: Value) -> Value {
        let n: f32 = num.into();
        Value::Num(n.round())
    }

    pub fn modulus(num1: Value, num2: Value) -> Value {
        let n: f32 = num1.into();
        let modulus: f32 = num2.into();
        Value::Num(n % modulus)
    }

    pub fn mathop(operator: Value, num: Value) -> Value {
        let n: f32 = num.into();
        let op: String = operator.into();

        let output = match &*op {
            "abs" => n.abs(),
            "floor" => n.floor(),
            "ceiling" => n.ceil(),
            "sqrt" => n.sqrt(),
            "sin" => n.to_radians().sin(),
            "cos" => n.to_radians().cos(),
            "tan" => n.to_radians().tan(),
            "asin" => n.asin().to_degrees(),
            "acos" => n.acos().to_degrees(),
            "atan" => n.atan().to_degrees(),
            "ln" => n.ln(),
            "log" => n.log(10.0),
            "e ^" => n.exp(),
            "10 ^" => 10f32.powf(n),
            _ => 0.0,
        };

        Value::Num(output)
    }

    /// Wait until condition is true.
    pub async fn wait_until<F>(condition: F)
    where
        F: Fn() -> Value,
    {
        if let Value::Bool(c) = condition() {
            println!("START WAITING");
            while !c {
                println!("STILL WAITING");
                Yield::Start.await;
            }
        } else {
            println!("INVALID WAIT");
        }
    }

    /// Create a clone of a sprite
    ///
    /// Procedure:
    /// - First, clone the sprite and add it to the list of sprites.
    /// - Then, create new instances of all scripts, attached to the new sprite.
    ///   This has to be done outside of this function
    pub fn create_clone(
        stage: Rc<Mutex<Stage>>,
        sprite: Rc<Mutex<Sprite>>,
        to_clone: Value,
    ) -> String {
        let mut stage = stage.lock().unwrap();

        let to_clone = to_clone.to_string();

        // define the reference to the sprite to be cloned
        let clone_target = match &*to_clone {
            "_myself_" => sprite,
            x => stage.get_sprite(x.to_string()).unwrap(),
        };

        let mut clone = clone_target.lock().unwrap().clone();
        // TODO make new clone go behind old sprite


        let old_name = clone.name.clone();

        clone.name = clone.name + "_clone";
        clone.clone = true;
        stage.add_sprite(Rc::new(Mutex::new(clone)));

        old_name
    }

    pub fn delete_this_clone(stage: Rc<Mutex<Stage>>, sprite: Rc<Mutex<Sprite>>) {
        let mut stage = stage.lock().unwrap();
        let mut sprite = sprite.lock().unwrap();

        if !sprite.clone {
            return;
        }

        sprite.to_be_deleted = true;
    }

    pub fn generate_random(from: Value, to: Value) -> Value {
        let from: u32 = from.into();
        let to: u32 = to.into();
        let r = rand::thread_rng().gen_range(from..=to);
        Value::Num(r as f32)
    }

    pub fn key_pressed(stage: Rc<Mutex<Stage>>, key: Value) -> Value {
        let mut stage = stage.lock().unwrap();
        let to_return = stage.keyboard.get_key_down(key);
        Value::Bool(to_return)
    }

    pub fn ask(stage: Rc<Mutex<Stage>>, string: Value) {
        println!("{}", string);
        let mut buffer = String::new();
        let stdin = io::stdin();
        stdin.read_line(&mut buffer).expect("Input should not fail");

        buffer = buffer.replace('\r', "");
        buffer = buffer.replace('\n', "");

        let mut stage = stage.lock().unwrap();
        stage.set_answer(Value::from(buffer));
    }

    pub fn answer(stage: Rc<Mutex<Stage>>) -> Value {
        let stage = stage.lock().unwrap();
        stage.get_answer()
    }

    /// Calculate the days since 2000.
    ///
    /// BUG Is not completely precise, seems to be off by a few days.
    pub fn days_since_2000() -> Value {
        //let start = chrono::Utc;
        let start = chrono::Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
        let now = chrono::Utc::now();
        let diff = now - start;

        let days = diff.num_days();
        let remainder = diff - chrono::Duration::days(days);

        let to_return = days as f32
            + (remainder.num_hours() as f32 / 24.0)
            + (remainder.num_minutes() as f32 / (24.0 * 60.0))
            + (remainder.num_seconds() as f32 / (24.0 * 60.0 * 60.0))
            + (remainder.num_milliseconds() as f32 / (24.0 * 60.0 * 60.0 * 1000.0));

        Value::Num(to_return)
    }

    pub fn username() -> Value {
        Value::String(String::from("Test username"))
    }

    pub fn clear_pen(stage: Rc<Mutex<Stage>>) {
        let mut stage = stage.lock().unwrap();

        stage.stamps.clear();
    }

    pub fn stamp(sprite: Rc<Mutex<Sprite>>, stage: Rc<Mutex<Stage>>) {
        let mut stage = stage.lock().unwrap();

        let mut sprite_lock = sprite.lock().unwrap();

        stage.stamps.push(Stamp {
            x: sprite_lock.x,
            y: sprite_lock.y,
            costume: sprite_lock.costume,
            size: sprite_lock.size,
            sprite: sprite.clone(),
        });
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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
#[derive(Debug, Clone)]
pub enum Value {
    Num(f32),
    String(String),
    Bool(bool),
    Null,
}

impl Value {
    fn to_list_index(self, length: usize, acceptAll: bool) -> Result<usize, ()> {
        if let Value::String(x) = &self {
            if x == "all" {
                if acceptAll {
                    unimplemented!("Return all not implemented yet");
                } else {
                    return Err(());
                }
            }
            if x == "last" {
                if length > 0 {
                    return Ok(length);
                }
                return Err(());
            }
            if x == "random" || x == "any" {
                if length > 0 {
                    return Ok(generate_random(1.into(), length.into()).into());
                }
                return Err(());
            }
        }
        let index: usize = self.into();
        if index < 1 || index > length {
            return Err(());
        }
        Ok(index)
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        if self.partial_cmp(other) == Some(std::cmp::Ordering::Equal) {
            return true;
        }
        false
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering as O;

        let mut n1 = Number(self);
        let mut n2 = Number(other);

        if let Value::String(x) = self {
            if x.trim().is_empty() {
                n1 = f32::NAN;
            }
        }
        if let Value::String(x) = other {
            if x.trim().is_empty() {
                n2 = f32::NAN;
            }
        }

        if n1.is_nan() || n2.is_nan() {
            let s1 = String(self).to_lowercase();
            let s2 = String(other).to_lowercase();
            if s1 < s2 {
                return Some(O::Less);
            } else if s1 > s2 {
                return Some(O::Greater);
            }
            return Some(O::Equal);
        }

        if n1.is_infinite() && n2.is_infinite() {
            return Some(O::Equal);
        }

        if n1 > n2 {
            return Some(O::Greater);
        } else if n1 < n2 {
            return Some(O::Less);
        }
        Some(O::Equal)
    }
}

impl Default for Value {
    fn default() -> Self {
        Self::Num(0.0)
    }
}

/// A macro to convert Values to numbers
macro_rules! value_into {
    ($t:ident) => {
        impl From<Value> for $t {
            fn from(val: Value) -> $t {
                match val {
                    Value::String(_) => 0 as $t,
                    Value::Bool(x) => match x {
                        true => 1 as $t,
                        false => 0 as $t,
                    },
                    Value::Null => 0 as $t,
                    Value::Num(x) => {
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

/// Cast a value into a number, following javascript's casting rules.
fn Number(input: &Value) -> f32 {
    match input {
        Value::Num(x) => *x,
        Value::Null => 0.0,
        Value::Bool(x) => match x {
            true => 1.0,
            false => 0.0,
        },
        Value::String(x) => {
            if let Ok(try_parse) = x.parse() {
                try_parse
            } else {
                f32::NAN
            }
        }
    }
}

fn toNumber(input: &Value) -> f32 {
    if let Value::Num(x) = input {
        if x.is_nan() {
            return 0.0;
        }
        return *x;
    }

    let n = Number(input);
    if n.is_nan() {
        return 0.0;
    }
    n
}

fn String(input: &Value) -> String {
    match input {
        Value::String(x) => x.clone(),
        Value::Null => String::from("null"),
        Value::Bool(x) => match x {
            true => String::from("true"),
            false => String::from("false"),
        },
        Value::Num(x) => format!("{}", x), // Possibly make sure this conforms to javascript?
                                           // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/toString
    }
}

value_into!(u32);
value_into!(u64);
value_into!(usize);
value_into!(f32);
value_into!(f64);
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

impl From<char> for Value {
    fn from(item: char) -> Self {
        Value::String(item.to_string())
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

impl From<Value> for String {
    fn from(val: Value) -> String {
        match val {
            Value::Num(x) => format!("{}", x),
            Value::String(x) => x,
            Value::Bool(x) => {
                if x {
                    String::from("true")
                } else {
                    String::from("false")
                }
            }
            Value::Null => String::new(),
        }
    }
}

impl From<Value> for bool {
    fn from(val: Value) -> bool {
        match val {
            Value::Num(x) => !matches!(x as i32, 0),
            Value::String(x) => !matches!(&*x.to_lowercase(), "" | "0" | "false"),
            Value::Bool(x) => x,
            Value::Null => false,
        }
    }
}

impl std::ops::Add for Value {
    type Output = Value;

    fn add(self, rhs: Self) -> Self::Output {
        let lhs_num: f32 = self.into();
        let rhs_num: f32 = rhs.into();

        Value::Num(lhs_num + rhs_num)
    }
}

impl std::ops::Sub for Value {
    type Output = Value;

    fn sub(self, rhs: Self) -> Self::Output {
        let lhs_num: f32 = self.into();
        let rhs_num: f32 = rhs.into();

        Value::Num(lhs_num - rhs_num)
    }
}

impl std::ops::Mul for Value {
    type Output = Value;

    fn mul(self, rhs: Self) -> Self::Output {
        let lhs_num: f32 = self.into();
        let rhs_num: f32 = rhs.into();

        Value::Num(lhs_num * rhs_num)
    }
}

impl std::ops::Div for Value {
    type Output = Value;

    fn div(self, rhs: Self) -> Self::Output {
        let lhs_num: f32 = self.into();
        let rhs_num: f32 = rhs.into();

        Value::Num(lhs_num / rhs_num)
    }
}

impl std::ops::Not for Value {
    type Output = Value;

    fn not(self) -> Self::Output {
        let x: bool = self.into();
        Value::Bool(!x)
    }
}

impl std::ops::Neg for Value {
    type Output = Value;

    fn neg(self) -> Self::Output {
        let num: f32 = self.into();
        Value::Num(-num)
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
    lists: HashMap<String, (String, Vec<Value>)>,
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
            lists: HashMap::new(),
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
            lists: self.lists,
            variables: self.variables,
            costume: self.costume,
            costumes: self.costumes,
            clone: false, // we never build a clone
            uuid: Uuid::new_v4(),
            to_be_deleted: false,
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
    pub fn add_list(mut self, id: String, value: (String, Vec<Value>)) -> Self {
        self.lists.insert(id, value);
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

#[derive(Debug)]
pub struct Sprite {
    /// Whether the sprite is visible.  Defaults to true.
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
    lists: HashMap<String, (String, Vec<Value>)>,
    /// The current costume. This is an index to the costumes attribute, which
    /// is itself an index!.
    costume: usize,
    /// A list of paths for costumes. Each item is an index to the program
    /// costume list.
    costumes: Vec<Costume>,

    /// Whether or not this sprite is a clone.
    ///
    /// This influences whether clone blocks can run.
    clone: bool,

    uuid: Uuid,
    to_be_deleted: bool,
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

impl Clone for Sprite {
    fn clone(&self) -> Self {
        Self {
            visible: self.visible.clone(),
            x: self.x.clone(),
            y: self.y.clone(),
            size: self.size.clone(),
            direction: self.direction.clone(),
            draggable: self.draggable.clone(),
            rotation_style: self.rotation_style.clone(),
            name: self.name.clone(),
            variables: self.variables.clone(),
            lists: self.lists.clone(),
            costume: self.costume.clone(),
            costumes: self
                .costumes
                .iter()
                .map(|c| {
                    Costume::new(c.name.clone(), c.path.clone(), c.scale)
                        .expect("Creating costume will not fail")
                })
                .collect(),
            clone: self.clone.clone(),
            uuid: Uuid::new_v4(),
            to_be_deleted: self.to_be_deleted
        }
    }
}

impl PartialEq for Sprite {
    fn eq(&self, other: &Self) -> bool {
        self.visible == other.visible
            && self.x == other.x
            && self.y == other.y
            && self.size == other.size
            && self.direction == other.direction
            && self.draggable == other.draggable
            && self.rotation_style == other.rotation_style
            && self.name == other.name
            && self.variables == other.variables
            && self.lists == other.lists
            && self.costume == other.costume
            // && self.costumes == other.costumes
            && self.clone == other.clone
    }
}

/// A costume or backdrop
pub struct Costume {
    name: String,
    rotation_center_x: u32,
    rotation_center_y: u32,
    texture: Texture,
    image: Image,
    path: PathBuf,
    scale: f32,
}

impl std::fmt::Debug for Costume {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Costume")
            .field("name", &self.name)
            .field("rotation_center_x", &self.rotation_center_x)
            .field("rotation_center_y", &self.rotation_center_y)
            .field("path", &self.path)
            .field("scale", &self.scale)
            .finish()
    }
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
            path,
            scale,
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

/// An empty type that implements future.
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

async fn yield_fn() {}

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

/// An enum to represent a wait.  This yields once at the beginning,
/// and then continues to yield until the wait is over.
enum Wait {
    Start { start: Instant, duration: Duration },
    Middle { start: Instant, duration: Duration },
    End,
}

impl Wait {
    /// Create a new wait.
    fn new(duration: Duration) -> Self {
        Self::Start {
            start: Instant::now(),
            duration,
        }
    }
}

impl Future for Wait {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match *self.as_mut() {
            Wait::Start { start, duration } => {
                *self = Wait::Middle { start, duration };
                Poll::Pending
            }
            Wait::Middle { start, duration } => {
                let now = Instant::now();
                if now >= start + duration {
                    *self = Wait::End;
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }
            Wait::End => panic!("poll called after Poll::Ready was returned"),
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

    sprite_uuid: Option<Uuid>,
}

impl std::fmt::Debug for Thread {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Thread")
            .field("complete", &self.complete)
            .field("running", &self.running)
            .field("start", &self.start)
            .finish()
    }
}

impl Thread {
    /// Create a new thread from a future.
    fn new(
        future: impl Future<Output = ()> + 'static, /*, obj_index: Option<usize>*/
        start: StartType,
        uuid: Option<Uuid>,
    ) -> Thread {
        Thread {
            function: Box::pin(future),
            complete: false,
            running: false,
            start, // obj_index,
            sprite_uuid: uuid,
        }
    }

    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.function.as_mut().poll(context)
    }
}

/// The main project class.  This is in charge of running threads and
/// redrawing the screen.
pub struct Program {
    threads: Vec<Thread>,
    //objects: Vec<Rc<Mutex<Sprite>>>,
    gl: GlGraphics,
    costumes: Vec<Costume>,
}

impl Program {
    /// Run 1 tick
    fn tick(&mut self, stage: Rc<Mutex<Stage>>) {
        self.add_threads_from_stage(stage.clone());

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
        self.delete_sprites(stage);
    }

    fn new() -> Self {
        Program {
            threads: Vec::new(),
            gl: GlGraphics::new(OpenGL::V3_2),
            costumes: Vec::new(),
        }
    }

    /// Simulate the flag being clicked by starting all threads with FlagClicked hat blocks.
    fn click_flag(&mut self) {
        for thread in &mut self.threads {
            if thread.start == StartType::FlagClicked {
                thread.running = true;
            }
        }
    }

    fn add_threads_from_stage(&mut self, stage: Rc<Mutex<Stage>>) {
        let mut stage = stage.lock().unwrap();

        for thread in stage.threads_to_add.drain(0..) {
            self.add_thread(thread);
        }
    }

    /// Check all sprites and see if they need to be deleted; if they do, delete
    /// them.
    fn delete_sprites(&mut self, stage: Rc<Mutex<Stage>>) {
        let mut stage = stage.lock().unwrap();

        stage.sprites.retain(|sprite| {
            let sprite = sprite.lock().unwrap();

            if sprite.to_be_deleted {
                self.threads
                    .retain(|thread| thread.sprite_uuid != Some(sprite.uuid))
            }

            !sprite.to_be_deleted
        })
    }

    /// Renders a red square.
    fn render(&mut self, args: &RenderArgs, stage: Rc<Mutex<Stage>>, size: WindowSize) {
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

            for stamp in &stage.stamps {
                let stamp_sprite = stamp.sprite.lock().unwrap();
                stamp_sprite.costumes[stamp.costume].draw(
                    c.transform
                        .trans(size.width / 2.0, size.height / 2.0)
                        .trans(stamp.x.into(), <f32 as Into<f64>>::into(stamp.y * -1.0)),
                    gl,
                );
            }

            // TODO sort by order
            // Draw the sprites
            for sprite in stage.sprites.clone() {
                let sprite = sprite.lock().unwrap();
                if !sprite.visible {
                    // if the sprite is hidden, don't draw it.
                    continue;
                }
                sprite.costumes[sprite.costume].draw(
                    c.transform
                        //.trans(240.0, 180.0)
                        .trans(size.width / 2.0, size.height / 2.0)
                        .trans(sprite.x.into(), <f32 as Into<f64>>::into(sprite.y * -1.0)),
                    gl,
                );
            }
        })

        // self.gl.draw(args.viewport(), |c, gl| {
        //     clear(BLACK, gl);
        // })
    }

    // /// Add threads from a sprite to the program.
    // /// This _moves_ the threads out of the sprite.
    // fn add_threads(&mut self, mut threads: Vec<Thread>) {
    //     // self.threads.append(&mut sprite.blocks);
    //     self.threads.append(&mut threads)
    // }

    /// Get a sprite by a name, or return null
    fn get_sprite(&self, name: String) -> Option<Rc<Mutex<Sprite>>> {
        None
        //for sprite in self.objects.clone(){
        //    let locked_sprite = sprite.lock().unwrap();
        //    if locked_sprite.name == name{
        //        return Some(sprite.clone());
        //    }
        //}
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

    fn add_threads<T: Iterator<Item = Thread>>(&mut self, threads: T) {
        for thread in threads {
            self.add_thread(thread);
        }
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

    texture
}

pub struct StageBuilder {
    tempo: i32,
    video_state: VideoState,
    video_transparency: i32,
    text_to_speech_language: String,
    variables: HashMap<String, (String, Value)>,
    lists: HashMap<String, (String, Vec<Value>)>,
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
            lists: HashMap::new(),
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
            lists: self.lists,
            costume: self.costume,
            costumes: self.costumes,
            sprites: Vec::new(),
            keyboard: Keyboard::new(),
            mouse: Mouse::new(),
            stamps: Vec::new(),
            answer: Value::from(String::new()),
            threads_to_add: VecDeque::new(),
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
    pub fn add_list(mut self, id: String, value: (String, Vec<Value>)) -> Self {
        self.lists.insert(id, value);
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
    /// is off or the project does not use an extension with video input.
    video_transparency: i32,
    /// The text to speech language.  Defaults to the editor language.
    text_to_speech_language: String,
    variables: HashMap<String, (String, Value)>,
    lists: HashMap<String, (String, Vec<Value>)>,
    /// The current costume.  An index to the stage costumes list.
    costume: usize,
    /// The costumes in the stage. These are indexes to the list of costumes in
    /// the program.
    costumes: Vec<Costume>,
    keyboard: Keyboard,
    mouse: Mouse,
    /// A list of references to sprites.
    sprites: Vec<Rc<Mutex<Sprite>>>,

    stamps: Vec<Stamp>,
    /// The current value of answer.
    answer: Value,

    threads_to_add: VecDeque<Thread>,
}

impl Stage {
    fn set_answer(&mut self, v: Value) {
        self.answer = v;
    }

    fn get_answer(&self) -> Value {
        self.answer.clone()
    }

    fn set_costume(&mut self, mut index: f32) {
        // round the index to a whole number
        index = index.round();

        if index.is_infinite() || index.is_nan() {
            index = 0.0;
        }

        index = index.clamp(0.0, self.costumes.len() as f32 - 1.0); // make sure the index is valid

        self.costume = index as usize;
    }

    fn add_sprite(&mut self, sprite: Rc<Mutex<Sprite>>) {
        self.sprites.push(sprite);
    }

    /// Get a sprite by a name, or return null
    fn get_sprite(&self, name: String) -> Option<Rc<Mutex<Sprite>>> {
        for sprite in self.sprites.clone() {
            // let locked_sprite = sprite.lock().unwrap();
            if sprite.lock().unwrap().name == name {
                return Some(sprite);
            }
        }
        None
    }

    fn add_threads<T: Iterator<Item = Thread>>(&mut self, threads: T) {
        for thread in threads {
            self.threads_to_add.push_back(thread);
        }
    }
}

/// The type of thread starter, such as flagClick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartType {
    FlagClicked,
    KeyPressed,
    SpriteClicked,
    BackdropSwitches,
    LoudnessGreater,
    ReceiveMessage,
    StartAsClone(String),
    CustomBlock,
    NoStart,
}

impl Display for StartType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                StartType::FlagClicked => "StartType::FlagClicked".to_string(),
                StartType::KeyPressed => "StartType::KeyPressed".to_string(),
                StartType::SpriteClicked => "StartType::SpriteClicked".to_string(),
                StartType::BackdropSwitches => "StartType::BackdropSwitches".to_string(),
                StartType::LoudnessGreater => "StartType::LoudnessGreater".to_string(),
                StartType::ReceiveMessage => "StartType::ReceiveMessage".to_string(),
                StartType::StartAsClone(sprite) =>
                    format!("StartType::StartAsClone(String::from(\"{sprite}\"))"),
                StartType::CustomBlock => "StartType::CustomBlock".to_string(),
                StartType::NoStart => "StartType::NoStart".to_string(),
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

#[derive(Debug, Clone)]
pub struct Keyboard {
    keys_pressed: HashSet<Key>,
    key_name: HashMap<&'static str, &'static str>,
}

impl Keyboard {
    fn new() -> Self {
        Keyboard {
            keys_pressed: HashSet::new(),
            key_name: HashMap::from([
                ("space", "SPACE"),
                ("left arrow", "LEFT"),
                ("up arrow", "UP"),
                ("right arrow", "RIGHT"),
                ("down arrow", "DOWN"),
                ("enter", "ENTER"),
            ]),
        }
    }

    fn press_key(&mut self, key: Key) {
        self.keys_pressed.insert(key);
    }

    fn release_key(&mut self, key: Key) {
        self.keys_pressed.remove(&key);
    }

    fn key_arg_to_scratch_key(&self, arg: Value) -> String {
        if let Value::Num(x) = arg {
            if (48.0..=90.0).contains(&x) {
                return String::from_utf16(&[x as u16]).expect("Proper range.");
            }
            return match x as u16 {
                32 => self.key_name.get("space").unwrap().to_string(),
                37 => self.key_name.get("left arrow").unwrap().to_string(),
                38 => self.key_name.get("up arrow").unwrap().to_string(),
                39 => self.key_name.get("right arrow").unwrap().to_string(),
                40 => self.key_name.get("down arrow").unwrap().to_string(),
                _ => unreachable!("Unknown key"),
            };
        }

        // match arg {
        //     Value::Num(_) => todo!(),
        //     Value::String(_) => todo!(),
        //     Value::Bool(_) => todo!(),
        //     Value::Null => todo!(),
        // }
        let mut keyArg: String = arg.into();

        if let Some(a) = self.key_name.get(&*keyArg) {
            return a.to_string();
        }

        if keyArg.len() > 1 {
            keyArg = keyArg.chars().next().unwrap().into();
        }

        if keyArg == *" " {
            return self.key_name.get("SPACE").unwrap().to_string();
        }

        keyArg.to_uppercase()
    }

    fn scratch_key_to_Key(&self, arg: String) -> Key {
        match &*arg {
            "A" => Key::A,
            "B" => Key::B,
            "C" => Key::C,
            "D" => Key::D,
            "E" => Key::E,
            "F" => Key::F,
            "G" => Key::G,
            "H" => Key::H,
            "I" => Key::I,
            "J" => Key::J,
            "K" => Key::K,
            "L" => Key::L,
            "M" => Key::M,
            "N" => Key::N,
            "O" => Key::O,
            "P" => Key::P,
            "Q" => Key::Q,
            "R" => Key::R,
            "S" => Key::S,
            "T" => Key::T,
            "U" => Key::U,
            "V" => Key::V,
            "W" => Key::W,
            "X" => Key::X,
            "Y" => Key::Y,
            "Z" => Key::Z,
            "space" => Key::Space,
            "left arrow" => Key::Left,
            "right arrow" => Key::Right,
            "up arrow" => Key::Up,
            "down arrow" => Key::Down,
            "SPACE" => Key::Space,
            "LEFT" => Key::Left,
            "RIGHT" => Key::Right,
            "UP" => Key::Up,
            "DOWN" => Key::Down,
            "0" => Key::D0,
            "1" => Key::D1,
            "2" => Key::D2,
            "3" => Key::D3,
            "4" => Key::D4,
            "5" => Key::D5,
            "6" => Key::D6,
            "7" => Key::D7,
            "8" => Key::D8,
            "9" => Key::D9,
            "!" => Key::Exclaim,
            "@" => Key::At,
            "#" => Key::Hash,
            "$" => Key::Dollar,
            "%" => Key::Percent,
            "^" => Key::Caret,
            "&" => Key::Ampersand,
            "*" => Key::Asterisk,
            "(" => Key::LeftParen,
            ")" => Key::RightParen,
            "`" => Key::Backquote,
            "-" => Key::Minus,
            "+" => Key::Plus,
            "_" => Key::Underscore,
            "=" => Key::Equals,
            "[" => Key::LeftBracket,
            "]" => Key::RightBracket,
            "\\" => Key::Backslash,
            ";" => Key::Semicolon,
            ":" => Key::Colon,
            "'" => Key::Quote,
            "\"" => Key::Quotedbl,
            "/" => Key::Slash,
            "?" => Key::Question,
            "." => Key::Period,
            "," => Key::Comma,
            // "<"=>Key::Angle
            _ => Key::Unknown,
        }
    }

    fn get_key_down(&self, arg: Value) -> bool {
        if let Value::String(x) = &arg {
            if x == &String::from("any") {
                return !self.keys_pressed.is_empty();
            }
        }

        let scratchKey = self.scratch_key_to_Key(self.key_arg_to_scratch_key(arg));

        self.keys_pressed.get(&scratchKey).is_some()
    }
}

/// The mouse struct.
///
/// This holds the position of the mouse, both in scratch coordinates and piston coordinates.
/// It also holds the keys that are pressed or not.
pub struct Mouse {
    scratch_position: (f32, f32),
    piston_position: (f32, f32),
}

impl Mouse {
    fn new() -> Self {
        Self {
            scratch_position: (0.0, 0.0),
            piston_position: (0.0, 0.0),
        }
    }

    /// Set the position of the mouse, in piston coordinates.
    /// This also sets the scratch position to the correct coordinates.
    pub fn set_piston_position(&mut self, pos: [f64; 2], size: WindowSize) {
        self.piston_position = (pos[0] as f32, pos[1] as f32);
        self.scratch_position = Mouse::piston2scratch((pos[0] as f32, pos[1] as f32), size);
    }

    /// Convert piston coordinates to scratch coordinates
    fn piston2scratch(coords: (f32, f32), size: WindowSize) -> (f32, f32) {
        let (mut x, mut y) = coords;

        // find the ratio of scratch size to actual size
        let ratio_x: f32 = Into::<f32>::into(SCRATCH_WIDTH) / size.width as f32;
        let ratio_y: f32 = Into::<f32>::into(SCRATCH_HEIGHT) / size.height as f32;

        // resize the coordinates to the correct size
        x *= ratio_x;
        y *= ratio_y;

        // convert coordinates from origin at top-left to origin at center.
        x -= Into::<f32>::into(SCRATCH_HALF_WIDTH);
        y = -(y - (Into::<f32>::into(SCRATCH_HALF_HEIGHT)));

        (x, y)
    }

    fn x(&self) -> Value {
        Value::Num(self.scratch_position.0)
    }
    fn y(&self) -> Value {
        Value::Num(self.scratch_position.1)
    }
}

impl Display for Mouse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({},{})",
            self.scratch_position.0, self.scratch_position.1
        )
    }
}

/// A stamp of a sprite.
struct Stamp {
    x: f32,
    y: f32,
    size: f32,
    costume: usize,
    sprite: Rc<Mutex<Sprite>>,
}
