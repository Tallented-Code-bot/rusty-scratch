#![allow(
    unused_imports,
    unused_mut,
    unused_variables,
    dead_code,
    non_snake_case
)]

extern crate rand;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use core::task::{RawWaker, RawWakerVTable, Waker};
use glium::{
    implement_vertex, uniform,
    uniforms::{AsUniformValue, DynamicUniforms},
    Frame, Surface,
};
use glium_sdl2::DisplayBuild;
use image::{GenericImageView, ImageBuffer, Rgba};
use sdl2::{event::WindowEvent, keyboard::Keycode, mouse::MouseButton, pixels::Color};
use std::{
    boxed::Box,
    collections::VecDeque,
    collections::{HashMap, HashSet},
    f32::consts::PI,
    fmt::{Debug, Display},
    fs,
    future::{Future, IntoFuture},
    hash::Hash,
    io,
    ops::Index,
    path::{Path, PathBuf},
    pin::Pin,
    rc::Rc,
    sync::Mutex,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use uuid::Uuid;

use blocks::*;

use self::glium_sdl2::SDL2Facade;

// set the width and height of the stage here
const SCRATCH_WIDTH: Value = Value::Num(480.0);
const SCRATCH_HALF_WIDTH: Value = Value::Num(240.0);
const SCRATCH_HEIGHT: Value = Value::Num(360.0);
const SCRATCH_HALF_HEIGHT: Value = Value::Num(180.0);

const LIST_ITEM_LIMIT: Value = Value::Num(20000.0); // TODO check this

mod blocks {
    use super::glium_sdl2::SDL2Facade;
    use super::{
        toNumber, Effect, Number, Stamp, StartType, String, Wait, LIST_ITEM_LIMIT,
        SCRATCH_HALF_HEIGHT, SCRATCH_HALF_WIDTH,
    };
    use super::{Keyboard, Sprite, Stage, Value, Yield};
    use chrono::TimeZone;
    use core::f32::consts::PI;
    use rand::Rng;
    use std::io;
    use std::time::Duration;
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

    pub fn point_in_direction(sprite: Rc<Mutex<Sprite>>, degrees: Value) {
        let mut sprite = sprite.lock().unwrap();
        sprite.direction = Number(&degrees);
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

    /// Move the current sprite to the front or back layer
    pub fn go_to_front_or_back(sprite: Rc<Mutex<Sprite>>, stage: Rc<Mutex<Stage>>, arg: Value) {
        let mut stage = stage.lock().unwrap();

        if String(&arg) == String::from("front") {
            // go to the front

            // get the current layer
            let current_layer = { sprite.lock().unwrap().layer };

            // find the largest layer
            let largest_layer = {
                stage
                    .sprites
                    .iter()
                    .max_by_key(|sprite| sprite.lock().unwrap().layer)
                    .unwrap()
                    .lock()
                    .unwrap()
                    .layer
            };

            for sprite in stage
                .sprites
                .iter_mut()
                .filter(|sprite| sprite.lock().unwrap().layer > current_layer)
            {
                sprite.lock().unwrap().layer -= 1;
            }

            sprite.lock().unwrap().layer = largest_layer;
        } else {
            // go to the back (smallest layer #)

            let current_layer = { sprite.lock().unwrap().layer };

            // find the smallest layer
            let smallest_layer = {
                stage
                    .sprites
                    .iter()
                    .min_by_key(|sprite| sprite.lock().unwrap().layer)
                    .unwrap()
                    .lock()
                    .unwrap()
                    .layer
            };

            // Increment layer numbers for all sprites with layers less than the current layer.
            for sprite in stage
                .sprites
                .iter_mut()
                .filter(|sprite| sprite.lock().unwrap().layer < current_layer)
            {
                sprite.lock().unwrap().layer += 1;
            }

            // set the current layer to the smallest possible
            sprite.lock().unwrap().layer = smallest_layer;
        }
        stage
            .sprites
            .sort_by(|a, b| a.lock().unwrap().layer.cmp(&b.lock().unwrap().layer));
    }

    pub fn go_forward_backwards_layers(
        sprite: Rc<Mutex<Sprite>>,
        stage: Rc<Mutex<Stage>>,
        front_back: Value,
        layers: Value,
    ) {
        if String(&front_back) == String::from("forward") {
            go_forward_layers(sprite, stage, layers);
        } else {
            go_backward_layers(sprite, stage, layers);
        }
    }

    pub fn go_forward_layers(sprite: Rc<Mutex<Sprite>>, stage: Rc<Mutex<Stage>>, layers: Value) {
        let mut stage = stage.lock().unwrap();
        let layers = toNumber(&layers) as u32;

        // retreive the current layer
        let current_layer = { sprite.lock().unwrap().layer };

        // find the future layer
        let mut future_layer = current_layer + layers;

        // make sure the future layer is not greater than the current maximum layer
        let max_layer = {
            stage
                .sprites
                .iter()
                .max_by_key(|sprite| sprite.lock().unwrap().layer)
                .unwrap()
                .lock()
                .unwrap()
                .layer
        };
        if future_layer > max_layer {
            future_layer = max_layer
        }

        // decrease items between the source and destination items
        for sprite in stage.sprites.iter_mut().filter(|sprite| {
            let x = sprite.lock().unwrap().layer;
            x > current_layer && x <= future_layer
        }) {
            sprite.lock().unwrap().layer -= 1;
        }

        // set the current layer to the destination layer
        {
            sprite.lock().unwrap().layer = future_layer;
        }

        // sort the list by layer
        stage
            .sprites
            .sort_by(|a, b| a.lock().unwrap().layer.cmp(&b.lock().unwrap().layer))
    }

    pub fn go_backward_layers(sprite: Rc<Mutex<Sprite>>, stage: Rc<Mutex<Stage>>, layers: Value) {
        let mut stage = stage.lock().unwrap();
        let layers = toNumber(&layers) as u32;

        // retreive the current layer
        let current_layer = { sprite.lock().unwrap().layer };

        // Set the future layer.  The lowest value is 0
        let mut future_layer = current_layer.checked_sub(layers).or(Some(1)).unwrap();
        if future_layer == 0 {
            future_layer = 1;
        }

        for sprite in stage.sprites.iter_mut().filter(|sprite| {
            let x = sprite.lock().unwrap().layer;
            x < current_layer && x >= future_layer
        }) {
            sprite.lock().unwrap().layer += 1;
        }

        {
            sprite.lock().unwrap().layer = future_layer;
        }

        stage
            .sprites
            .sort_by(|a, b| a.lock().unwrap().layer.cmp(&b.lock().unwrap().layer))
    }

    pub fn play_sound(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        sound_name: Value,
    ) -> f32 {
        if let Some(x) = sprite {
            let sprite = x.lock().unwrap();
            if let Some(sound) = sprite
                .sounds
                .iter()
                .find(|x| *x.name == String(&sound_name))
            {
                let seconds_duration = sound.sample_count as f32 / sound.rate as f32;
                // music::play_sound(
                //     &sound_name,
                //     music::Repeat::Times(0),
                //     (sprite.volume as f64) / 100.0,
                // );
                return seconds_duration;
            }
        } else {
            let stage = stage.lock().unwrap();
            if let Some(sound) = stage.sounds.iter().find(|x| *x.name == String(&sound_name)) {
                let seconds_duration = sound.sample_count as f32 / sound.rate as f32;
                // music::play_sound(
                //     &sound_name,
                //     music::Repeat::Times(0),
                //     (stage.volume as f64) / 100.0,
                // );
                return seconds_duration;
            }
        }
        return 0.0;
    }

    pub fn set_volume(sprite: Option<Rc<Mutex<Sprite>>>, stage: Rc<Mutex<Stage>>, volume: Value) {
        if let Some(sprite) = sprite {
            let mut sprite = sprite.lock().unwrap();

            sprite.volume = Into::<f32>::into(volume).clamp(0.0, 100.0);
        } else {
            let mut stage = stage.lock().unwrap();

            stage.volume = Into::<f32>::into(volume).clamp(0.0, 100.0);
        }
    }

    pub fn change_volume(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        changeBy: Value,
    ) {
        if let Some(sprite) = sprite {
            let mut sprite = sprite.lock().unwrap();

            sprite.volume += Into::<f32>::into(changeBy);
            sprite.volume = sprite.volume.clamp(0.0, 100.0);
        } else {
            let mut stage = stage.lock().unwrap();

            stage.volume += Into::<f32>::into(changeBy);
            stage.volume = stage.volume.clamp(0.0, 100.0);
        }
    }

    pub fn get_volume(sprite: Option<Rc<Mutex<Sprite>>>, stage: Rc<Mutex<Stage>>) -> Value {
        if let Some(sprite) = sprite {
            let sprite = sprite.lock().unwrap();
            return Value::Num(sprite.volume);
        } else {
            let stage = stage.lock().unwrap();
            return Value::Num(stage.volume);
        }
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

        if let Ok(x) = index {
            if x > LIST_ITEM_LIMIT.into() {
                return;
            }

            list.insert(x - 1, item);

            if list.len() > LIST_ITEM_LIMIT.into() {
                list.pop();
            }

            replace_list(sprite, stage, list, (name, id));
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

        if let Ok(x) = index {
            list[x - 1] = item;
            replace_list(sprite, stage, list, (name, id));
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

    pub fn set_effect(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        effect: Value,
        amount: Value,
    ) {
        let e = match &*String(&effect).to_lowercase() {
            "color" => (Effect::Color, Number(&amount)),
            "fisheye" => (Effect::Fisheye, Number(&amount)),
            "whirl" => (Effect::Whirl, Number(&amount)),
            "pixelate" => (Effect::Pixelate, Number(&amount)),
            "mosaic" => (Effect::Mosaic, Number(&amount)),
            "brightness" => (Effect::Brightness, Number(&amount)),
            "ghost" => (Effect::Ghost, Number(&amount)),
            _ => return,
        };

        if let Some(sprite) = sprite {
            let mut sprite = sprite.lock().unwrap();

            let effects_len = sprite.effects.len();

            if sprite.effects.insert(e.0, e.1).is_none() {
                // if this is a new effect, we need to recompile the shaders.
                sprite.need_to_recompile_shaders = true;
            }
        } else {
            let mut stage = stage.lock().unwrap();

            let effects_len = stage.effects.len();

            if stage.effects.insert(e.0, e.1).is_none() {
                stage.need_to_recompile_shaders = true;
            }
        }
    }

    pub fn change_effect(
        sprite: Option<Rc<Mutex<Sprite>>>,
        stage: Rc<Mutex<Stage>>,
        effect: Value,
        amount: Value,
    ) {
        let e = match &*String(&effect).to_lowercase() {
            "color" => (Effect::Color, Number(&amount)),
            "fisheye" => (Effect::Fisheye, Number(&amount)),
            "whirl" => (Effect::Whirl, Number(&amount)),
            "pixelate" => (Effect::Pixelate, Number(&amount)),
            "mosaic" => (Effect::Mosaic, Number(&amount)),
            "brightness" => (Effect::Brightness, Number(&amount)),
            "ghost" => (Effect::Ghost, Number(&amount)),
            _ => return,
        };

        if let Some(sprite) = sprite {
            let mut sprite = sprite.lock().unwrap();

            if let Some(x) = sprite.effects.get_mut(&e.0) {
                *x += e.1;
            } else {
                sprite.effects.insert(e.0, e.1);
                sprite.need_to_recompile_shaders = true;
            }
        } else {
            let mut stage = stage.lock().unwrap();

            if let Some(x) = stage.effects.get_mut(&e.0) {
                *x += e.1;
            } else {
                stage.effects.insert(e.0, e.1);
                stage.need_to_recompile_shaders = true;
            }
        }
    }

    pub fn clear_effects(sprite: Option<Rc<Mutex<Sprite>>>, stage: Rc<Mutex<Sprite>>) {
        if let Some(sprite) = sprite {
            let mut sprite = sprite.lock().unwrap();
            sprite.effects.clear();
            sprite.need_to_recompile_shaders = true;
        } else {
            let mut stage = stage.lock().unwrap();
            stage.effects.clear();
            stage.need_to_recompile_shaders = true;
        }
    }

    /// Join two strings.
    pub fn join(a: Value, b: Value) -> Value {
        Value::String(format!("{}{}", a, b))
    }

    pub fn letter_of(letter: Value, string: Value) -> Value {
        let mut index: usize = letter.into();

        if let Some(x)=index.checked_sub(1){
            index = x;
        }
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
        window: &SDL2Facade,
        to_clone: Value,
    ) -> String {
        let mut stage = stage.lock().unwrap();

        let to_clone = to_clone.to_string();

        // define the reference to the sprite to be cloned
        let clone_target = match &*to_clone {
            "_myself_" => sprite,
            x => stage.get_sprite(x.to_string()).unwrap(),
        };

        let mut clone = clone_target.lock().unwrap().clone(window);

        for s in stage
            .sprites
            .iter_mut()
            .filter(|sprite| sprite.lock().unwrap().layer >= clone.layer)
        {
            s.lock().unwrap().layer += 1;
        }

        let old_name = clone.name.clone();

        clone.name += "_clone";
        clone.clone = true;
        stage.add_sprite(Rc::new(Mutex::new(clone)));

        stage
            .sprites
            .sort_by_key(|sprite| sprite.lock().unwrap().layer);

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

    pub fn mousex(stage: Rc<Mutex<Stage>>) -> Value {
        let stage = stage.lock().unwrap();

        stage.mouse.x()
    }

    pub fn mousey(stage: Rc<Mutex<Stage>>) -> Value {
        let stage = stage.lock().unwrap();

        stage.mouse.y()
    }

    pub fn mouse_down(stage: Rc<Mutex<Stage>>) -> Value {
        let stage = stage.lock().unwrap();

        Value::Bool(stage.mouse.mouse_down())
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

mod glium_sdl2 {
    // Copyright (c) 2016 glium_sdl2 developers
    // Licensed under the Apache License, Version 2.0
    // <LICENSE-APACHE or
    // http://www.apache.org/licenses/LICENSE-2.0> or the MIT
    // license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
    // at your option. All files in the project carrying such
    // notice may not be copied, modified, or distributed except
    // according to those terms.

    //! An SDL2 backend for [Glium](https://github.com/tomaka/glium) - a high-level
    //! OpenGL wrapper for the Rust language.
    //!
    //! # Example
    //! ```no_run
    //! # #[macro_use] extern crate glium;
    //! # extern crate glium_sdl2;
    //! # extern crate sdl2;
    //! # fn main() {
    //! use glium_sdl2::DisplayBuild;
    //!
    //! let sdl_context = sdl2::init().unwrap();
    //! let video_subsystem = sdl_context.video().unwrap();
    //!
    //! let display = video_subsystem.window("My window", 800, 600)
    //!     .resizable()
    //!     .build_glium()
    //!     .unwrap();
    //!
    //! let mut running = true;
    //! let mut event_pump = sdl_context.event_pump().unwrap();
    //!
    //! while running {
    //!     let mut target = display.draw();
    //!     // do drawing here...
    //!     target.finish().unwrap();
    //!
    //!     // Event loop: includes all windows
    //!
    //!     for event in event_pump.poll_iter() {
    //!         use sdl2::event::Event;
    //!
    //!         match event {
    //!             Event::Quit { .. } => {
    //!                 running = false;
    //!             },
    //!             _ => ()
    //!         }
    //!     }
    //! }
    //! # }
    //! ```

    extern crate glium;
    extern crate sdl2;

    use std::cell::UnsafeCell;
    use std::mem;
    use std::ops::Deref;
    use std::os::raw::c_void;
    use std::rc::Rc;

    use glium::backend::{Backend, Context, Facade};
    use glium::debug;
    use glium::IncompatibleOpenGl;
    use glium::SwapBuffersError;
    use sdl2::video::{Window, WindowBuildError};
    use sdl2::VideoSubsystem;

    pub type Display = SDL2Facade;

    #[derive(Debug)]
    pub enum GliumSdl2Error {
        WindowBuildError(WindowBuildError),
        ContextCreationError(String),
    }

    impl From<String> for GliumSdl2Error {
        fn from(s: String) -> GliumSdl2Error {
            return GliumSdl2Error::ContextCreationError(s);
        }
    }

    impl From<WindowBuildError> for GliumSdl2Error {
        fn from(err: WindowBuildError) -> GliumSdl2Error {
            return GliumSdl2Error::WindowBuildError(err);
        }
    }

    impl From<IncompatibleOpenGl> for GliumSdl2Error {
        fn from(err: IncompatibleOpenGl) -> GliumSdl2Error {
            GliumSdl2Error::ContextCreationError(err.to_string())
        }
    }

    impl std::error::Error for GliumSdl2Error {
        fn description(&self) -> &str {
            return match *self {
                GliumSdl2Error::WindowBuildError(ref err) => err.description(),
                GliumSdl2Error::ContextCreationError(ref s) => s,
            };
        }

        fn cause(&self) -> Option<&dyn std::error::Error> {
            match *self {
                GliumSdl2Error::WindowBuildError(ref err) => err.source(),
                GliumSdl2Error::ContextCreationError(_) => None,
            }
        }
    }

    impl std::fmt::Display for GliumSdl2Error {
        fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
            match *self {
                GliumSdl2Error::WindowBuildError(ref err) => err.fmt(formatter),
                GliumSdl2Error::ContextCreationError(ref err) => err.fmt(formatter),
            }
        }
    }

    /// Facade implementation for an SDL2 window.
    #[derive(Clone)]
    pub struct SDL2Facade {
        // contains everything related to the current context and its state
        context: Rc<Context>,

        backend: Rc<SDL2WindowBackend>,
    }

    impl Facade for SDL2Facade {
        fn get_context(&self) -> &Rc<Context> {
            &self.context
        }
    }

    impl Deref for SDL2Facade {
        type Target = Context;

        fn deref(&self) -> &Context {
            &self.context
        }
    }

    impl SDL2Facade {
        pub fn window(&self) -> &Window {
            self.backend.window()
        }

        pub fn window_mut(&mut self) -> &mut Window {
            self.backend.window_mut()
        }

        /// Start drawing on the backbuffer.
        ///
        /// This function returns a `Frame`, which can be used to draw on it.
        /// When the `Frame` is destroyed, the buffers are swapped.
        ///
        /// Note that destroying a `Frame` is immediate, even if vsync is enabled.
        pub fn draw(&self) -> glium::Frame {
            glium::Frame::new(
                self.context.clone(),
                self.backend.get_framebuffer_dimensions(),
            )
        }
    }

    /// An object that can build a facade object.
    ///
    /// This trait is different from `glium::DisplayBuild` because Rust doesn't allow trait
    /// implementations on types from external crates, unless the trait is in the same crate as the impl.
    /// To clarify, both `glium::DisplayBuild` and `sdl2::video::WindowBuilder` are in different crates
    /// than `glium_sdl2`.
    pub trait DisplayBuild {
        /// The object that this `DisplayBuild` builds.
        type Facade: glium::backend::Facade;

        /// The type of error that initialization can return.
        type Err;

        /// Build a context and a facade to draw on it.
        ///
        /// Performs a compatibility check to make sure that all core elements of glium
        /// are supported by the implementation.
        fn build_glium(self) -> Result<Self::Facade, Self::Err>
        where
            Self: Sized,
        {
            self.build_glium_debug(Default::default())
        }

        /// Build a context and a facade to draw on it.
        ///
        /// Performs a compatibility check to make sure that all core elements of glium
        /// are supported by the implementation.
        fn build_glium_debug(
            self,
            _: debug::DebugCallbackBehavior,
        ) -> Result<Self::Facade, Self::Err>;

        /// Build a context and a facade to draw on it
        ///
        /// This function does the same as `build_glium`, except that the resulting context
        /// will assume that the current OpenGL context will never change.
        unsafe fn build_glium_unchecked(self) -> Result<Self::Facade, Self::Err>
        where
            Self: Sized,
        {
            self.build_glium_unchecked_debug(Default::default())
        }

        /// Build a context and a facade to draw on it
        ///
        /// This function does the same as `build_glium`, except that the resulting context
        /// will assume that the current OpenGL context will never change.
        unsafe fn build_glium_unchecked_debug(
            self,
            _: debug::DebugCallbackBehavior,
        ) -> Result<Self::Facade, Self::Err>;

        // TODO
        // Changes the settings of an existing facade.
        // fn rebuild_glium(self, &Self::Facade) -> Result<(), Self::Err>;
    }

    impl<'a> DisplayBuild for &'a mut sdl2::video::WindowBuilder {
        type Facade = SDL2Facade;
        type Err = GliumSdl2Error;

        fn build_glium_debug(
            self,
            debug: debug::DebugCallbackBehavior,
        ) -> Result<SDL2Facade, GliumSdl2Error> {
            let backend = Rc::new(SDL2WindowBackend::new(self)?);
            let context = unsafe { Context::new(backend.clone(), true, debug) }?;

            let display = SDL2Facade { context, backend };

            Ok(display)
        }

        unsafe fn build_glium_unchecked_debug(
            self,
            debug: debug::DebugCallbackBehavior,
        ) -> Result<SDL2Facade, GliumSdl2Error> {
            let backend = Rc::new(SDL2WindowBackend::new(self)?);
            let context = Context::new(backend.clone(), false, debug)?;

            let display = SDL2Facade { context, backend };

            Ok(display)
        }
    }

    pub struct SDL2WindowBackend {
        window: UnsafeCell<sdl2::video::Window>,
        context: sdl2::video::GLContext,
    }

    impl SDL2WindowBackend {
        fn subsystem(&self) -> &VideoSubsystem {
            let ptr = self.window.get();
            let window: &Window = unsafe { mem::transmute(ptr) };
            window.subsystem()
        }

        fn window(&self) -> &Window {
            let ptr = self.window.get();
            let window: &Window = unsafe { mem::transmute(ptr) };
            window
        }

        fn window_mut(&self) -> &mut Window {
            let ptr = self.window.get();
            let window: &mut Window = unsafe { mem::transmute(ptr) };
            window
        }

        pub fn new(
            window_builder: &mut sdl2::video::WindowBuilder,
        ) -> Result<SDL2WindowBackend, GliumSdl2Error> {
            let window = window_builder.opengl().build()?;
            let context = window.gl_create_context()?;

            Ok(SDL2WindowBackend {
                window: UnsafeCell::new(window),
                context,
            })
        }
    }

    unsafe impl Backend for SDL2WindowBackend {
        fn swap_buffers(&self) -> Result<(), SwapBuffersError> {
            self.window().gl_swap_window();

            // AFAIK, SDL or `SDL_GL_SwapWindow` doesn't have any way to detect context loss.
            // TODO: Find out if context loss is an issue in SDL2 (especially for the Android port).

            Ok(())
        }

        unsafe fn get_proc_address(&self, symbol: &str) -> *const c_void {
            // Assumes the appropriate context for the window has been set before this call.

            self.subsystem().gl_get_proc_address(symbol) as *const c_void
        }

        fn get_framebuffer_dimensions(&self) -> (u32, u32) {
            let (width, height) = self.window().drawable_size();
            (width as u32, height as u32)
        }

        fn is_current(&self) -> bool {
            self.context.is_current()
        }

        unsafe fn make_current(&self) {
            self.window().gl_make_current(&self.context).unwrap()
        }

        fn resize(&self, new_size: (u32, u32)) {}
    }
}

/// An openGL vertex.
#[derive(Copy, Clone)]
struct Vertex {
    a_position: [f32; 2],
    a_texCoord: [f32; 2],
}

impl Vertex {
    fn new(position: [f32; 2], tex_coords: [f32; 2]) -> Self {
        Self {
            a_position: position,
            a_texCoord: tex_coords,
        }
    }
}

implement_vertex!(Vertex, a_position, a_texCoord);

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
    pub fn to_str(self) -> &'static str {
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

// Implement hash for value.
impl Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Value::Num(x) => (*x as i32).hash(state),
            Value::String(x) => x.hash(state),
            Value::Bool(x) => x.hash(state),
            Value::Null => ().hash(state),
        }
    }
}

impl Value {
    fn to_list_index(&self, length: usize, acceptAll: bool) -> Result<usize, ()> {
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
        let index: usize = Number(self) as usize;
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

impl Eq for Value {}

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
            return Some(s1.cmp(&s2));
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
    sounds: Vec<Sound>,
    volume: f32,
    layer: u32,
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
            sounds: Vec::new(),
            volume: 100.0,
            layer: 999,
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
            sounds: self.sounds,
            volume: self.volume,
            costumes: self.costumes,
            clone: false, // we never build a clone
            uuid: Uuid::new_v4(),
            to_be_deleted: false,
            layer: self.layer,
            effects: HashMap::new(),
            need_to_recompile_shaders: false,
        }
    }

    pub fn layer(mut self, layer: u32) -> Self {
        self.layer = layer;
        self
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
    pub fn add_sound(mut self, sound: Sound) -> Self {
        self.sounds.push(sound);
        self
    }
    pub fn set_volume(mut self, volume: f32) -> Self {
        self.volume = volume;
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

    /// A list of sounds belonging to this sprite.
    sounds: Vec<Sound>,

    /// The volume of this sprite.
    volume: f32,

    /// Whether or not this sprite is a clone.
    ///
    /// This influences whether clone blocks can run.
    clone: bool,

    uuid: Uuid,
    to_be_deleted: bool,
    /// The current layer of the sprite. Sprites with higher layers are
    /// displayed on top of sprites with lower layers.
    layer: u32,

    /// The sprite effects
    effects: HashMap<Effect, f32>,
    need_to_recompile_shaders: bool,
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

    fn clone(&self, window: &SDL2Facade) -> Self {
        Self {
            visible: self.visible,
            x: self.x,
            y: self.y,
            size: self.size,
            direction: self.direction,
            draggable: self.draggable,
            rotation_style: self.rotation_style,
            name: self.name.clone(),
            variables: self.variables.clone(),
            lists: self.lists.clone(),
            costume: self.costume,
            costumes: self
                .costumes
                .iter()
                .map(|c| {
                    Costume::new(window, c.name.clone(), c.path.clone(), c.scale)
                        .expect("Creating costume will not fail")
                })
                .collect(),
            sounds: self.sounds.clone(),
            volume: self.volume,
            clone: self.clone,
            uuid: Uuid::new_v4(),
            to_be_deleted: self.to_be_deleted,
            layer: self.layer,
            effects: self.effects.clone(),
            need_to_recompile_shaders: self.need_to_recompile_shaders,
        }
    }

    /// Get the rendered direction. 0 is forward, 90 is up, 180 backwards, 270
    /// down, etc.
    ///
    /// Formula from <https://math.stackexchange.com/questions/1589793/a-formula-to-convert-a-counter-clockwise-angle-to-clockwise-angle-with-an-offset>
    fn get_rendered_direction(&self) -> f32 {
        (-self.direction + 90.0).rem_euclid(360.0)
    }
}

fn recompile_shaders(stage: Option<&mut Stage>, sprite: Option<&mut Sprite>, window: &SDL2Facade) {
    if let Some(sprite) = sprite {
        let mut defines = sprite
            .effects
            .iter()
            .map(|x| x.0.define_string())
            .collect::<Vec<_>>();

        defines.push(String::from("#define DRAW_MODE_default"));

        let define_string = format!("#version 140\n{}", defines.join("\n"));

        for costume in &mut sprite.costumes {
            costume.program = glium::Program::from_source(
                window,
                &format!("{}\n{}", define_string, Program::get_vertex_shader()),
                &format!("{}\n{}", define_string, Program::get_fragment_shader()),
                None,
            )
            .unwrap();
        }
    } else if let Some(stage) = stage {
        let mut defines = stage
            .effects
            .iter()
            .map(|x| x.0.define_string())
            .collect::<Vec<_>>();

        defines.push(String::from("#define DRAW_MODE_default"));

        let define_string = format!("#version 140\n{}", defines.join("\n"));

        for costume in &mut stage.costumes {
            costume.program = glium::Program::from_source(
                window,
                &format!("{}\n{}", define_string, Program::get_vertex_shader()),
                &format!("{}\n{}", define_string, Program::get_fragment_shader()),
                None,
            )
            .unwrap();
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
    texture: glium::texture::Texture2d,
    program: glium::Program,
    vertices: glium::VertexBuffer<Vertex>,
    indices: glium::index::NoIndices,
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
    fn new(
        window: &SDL2Facade,
        name: String,
        path: PathBuf,
        scale: f32,
    ) -> Result<Self, &'static str> {
        let texture = get_texture_from_path(window, path.clone(), scale)?;

        let (width, height) = texture.dimensions();
        let top_left = [-(width as f32 / 2.0), height as f32 / 2.0];
        let bottom_right = [width as f32 / 2.0, -(height as f32 / 2.0)];

        let vertices_vec = Program::rect(top_left, bottom_right);
        let vertices = glium::VertexBuffer::new(window, &vertices_vec).unwrap();
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

        let program = glium::Program::from_source(
            window,
            &*format!("#version 140\n{}", Program::get_vertex_shader()),
            &*format!("#version 140\n{}", Program::get_fragment_shader()),
            None,
        )
        .unwrap();

        Ok(Self {
            name,
            rotation_center_x: 0,
            rotation_center_y: 0,
            texture,
            vertices,
            indices,
            program,
            path,
            scale,
        })
    }

    fn draw(
        &self,
        target: &mut glium::Frame,
        transform: [[f32; 4]; 4],
        effects: &HashMap<Effect, f32>,
    ) {
        let mut uniforms = DynamicUniforms::new();
        uniforms.add("u_modelMatrix", &transform);
        let binding = Program::ortho_matrix();
        uniforms.add("u_projectionMatrix", &binding);
        uniforms.add("u_skin", &self.texture);

        let effect_uniform_names: Vec<_> = effects.iter().map(|x| x.0.uniforms()).collect();
        let values: Vec<_> = effects.iter().map(|x| x.1).collect();
        let calc_values: Vec<_> = values
            .iter()
            .zip(effect_uniform_names)
            .map(|(val, (name, f))| (name, f(**val)))
            .collect();

        for (name, val) in &calc_values {
            uniforms.add(name, val);
        }

        // let uniforms = uniform! {
        //     u_modelMatrix: transform,
        //     u_projectionMatrix: Program::identity_matrix(),
        //     u_skin: &self.texture,
        // };

        target
            .draw(
                &self.vertices,
                &self.indices,
                &self.program,
                &uniforms,
                &glium::DrawParameters {
                    blend: glium::Blend::alpha_blending(),
                    ..Default::default()
                },
            )
            .unwrap();
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    Color,
    Fisheye,
    Whirl,
    Pixelate,
    Brightness,
    Ghost,
    Mosaic,
}

impl Effect {
    fn define_string(&self) -> String {
        String::from(match self {
            Effect::Color => "#define ENABLE_color",
            Effect::Fisheye => "#define ENABLE_fisheye",
            Effect::Whirl => "#define ENABLE_whirl",
            Effect::Pixelate => "#define ENABLE_pixelate",
            Effect::Brightness => "#define ENABLE_brightness",
            Effect::Ghost => "#define ENABLE_ghost",
            Effect::Mosaic => "#define ENABLE_mosaic",
        })
    }

    fn uniforms(&self) -> (&str, Box<dyn Fn(f32) -> f32>) {
        match self {
            Effect::Color => ("u_color", Box::new(|x| (x / 200.0) % 1.0)),
            Effect::Fisheye => ("u_fisheye", Box::new(|x| 0f32.max((x + 100.0) / 100.0))),
            Effect::Whirl => ("u_whirl", Box::new(|x| -x * PI / 180.0)),
            Effect::Pixelate => ("u_pixelate", Box::new(|x| x.abs() / 10.0)),
            Effect::Brightness => (
                "u_brightness",
                Box::new(|x| -100f32.max(x.min(100.0)) / 100.0),
            ),
            Effect::Ghost => (
                "u_ghost",
                Box::new(|x| 1.0 - (0f32.max(x.min(100.0)) / 100.0)),
            ),
            Effect::Mosaic => (
                "u_mosaic",
                Box::new(|x| {
                    let x = ((x.abs() + 10.0) / 10.0).round();

                    1f32.max(x.min(512.0))
                }),
            ),
        }
    }
}

/// A sound that can be played.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Sound {
    name: String,
    format: String,
    rate: u32,
    sample_count: u32,
}

impl Sound {
    fn new(name: String, format: String, rate: u32, sample_count: u32) -> Self {
        Self {
            name,
            format,
            rate,
            sample_count,
        }
    }
}

fn initialize_sounds(stage: Rc<Mutex<Stage>>) {
    let stage = stage.lock().unwrap();

    for sprite in &stage.sprites {
        let sprite = sprite.lock().unwrap();

        for sound in &sprite.sounds {
            let val = Value::from(sound.name.clone());
            let path = format!("assets/{}/{}.{}", sprite.name, sound.name, sound.format);

            // bind_sound_file(val, path);
        }
    }

    for sound in &stage.sounds {
        // bind_sound_file(
        //     Value::from(sound.name.clone()),
        //     format!("assets/Stage/{}.{}", sound.name, sound.format),
        // );
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

    // Create a new RawWaker.
    RawWaker::new(std::ptr::null::<()>(), vtable)
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
                // println!("Wait being polled...");
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
pub struct Program<'a> {
    threads: Vec<Thread>,
    //objects: Vec<Rc<Mutex<Sprite>>>,
    window: &'a SDL2Facade,
    costumes: Vec<Costume>,
}

impl<'a> Program<'a> {
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

    fn new(window: &'a SDL2Facade) -> Self {
        Program {
            threads: Vec::new(),
            window,
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

    /// Render the stage, all sprites, and everything else that needs to be
    /// rendered.
    fn render(&mut self, stage: Rc<Mutex<Stage>>) {
        let mut stage = stage.lock().unwrap();

        let mut target = self.window.draw();

        target.clear_color(1.0, 1.0, 1.0, 1.0); // Clear the background color to white
        let transform = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0f32],
        ];

        if stage.need_to_recompile_shaders {
            recompile_shaders(Some(&mut stage), None, self.window);
            stage.need_to_recompile_shaders = false;
        }

        stage.costumes[stage.costume].draw(&mut target, transform, &stage.effects);

        for stamp in &stage.stamps {
            let transform = [
                [1.0 * (stamp.size / 100.0), 0.0, 0.0, 0.0],
                [0.0, 1.0 * (stamp.size / 100.0), 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [
                    stamp.x / f32::from(SCRATCH_HALF_WIDTH),
                    stamp.y / f32::from(SCRATCH_HALF_HEIGHT),
                    0.0,
                    1.0f32,
                ],
            ];

            let stamp_sprite = stamp.sprite.lock().unwrap();
            stamp_sprite.costumes[stamp.costume].draw(&mut target, transform, &HashMap::new());
        }

        for sprite in stage.sprites.clone() {
            let mut sprite = sprite.lock().unwrap();

            if sprite.need_to_recompile_shaders {
                recompile_shaders(None, Some(&mut sprite), self.window);
                sprite.need_to_recompile_shaders = false;
            }

            if !sprite.visible {
                continue;
            }

            let transform = [
                [
                    sprite.get_rendered_direction().to_radians().cos() * (sprite.size / 100.0),
                    (sprite.get_rendered_direction().to_radians().sin()),
                    0.0,
                    0.0,
                ],
                [
                    -(sprite.get_rendered_direction().to_radians().sin()),
                    sprite.get_rendered_direction().to_radians().cos() * (sprite.size / 100.0),
                    0.0,
                    0.0,
                ],
                [0.0, 0.0, 1.0, 0.0],
                [
                    sprite.x, // f32::from(SCRATCH_HALF_WIDTH),
                    sprite.y, // f32::from(SCRATCH_HALF_HEIGHT),
                    0.0, 1.0f32,
                ],
            ];

            sprite.costumes[sprite.costume].draw(&mut target, transform, &sprite.effects);
        }

        target.finish().unwrap();
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

    /// Create openGL vertexes for a rectangle.
    fn rect(top_left: [f32; 2], bottom_right: [f32; 2]) -> Vec<Vertex> {
        let top_right = [bottom_right[0], top_left[1]];
        let bottom_left = [top_left[0], bottom_right[1]];

        vec![
            Vertex::new(bottom_left, [0.0, 0.0]),
            Vertex::new(bottom_right, [1.0, 0.0]),
            Vertex::new(top_right, [1.0, 1.0]),
            Vertex::new(top_right, [1.0, 1.0]),
            Vertex::new(top_left, [0.0, 1.0]),
            Vertex::new(bottom_left, [0.0, 0.0]),
        ]
    }

    fn identity_matrix() -> [[f32; 4]; 4] {
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }

    fn ortho_matrix() -> [[f32; 4]; 4] {
        let left = -f32::from(SCRATCH_HALF_WIDTH);
        let right = f32::from(SCRATCH_HALF_WIDTH);
        let top = f32::from(SCRATCH_HALF_HEIGHT);
        let bottom = -f32::from(SCRATCH_HALF_HEIGHT);

        let near = -1.0;
        let far = 1.0;

        [
            [2.0 / (right - left), 0.0, 0.0, 0.0],
            [0.0, 2.0 / (top - bottom), 0.0, 0.0],
            [0.0, 0.0, 2.0 / (near - far), 0.0],
            [
                (right + left) / (left - right),
                (top + bottom) / (bottom - top),
                (far + near) / (near - far),
                1.0,
            ],
        ]
    }

    fn get_vertex_shader() -> &'static str {
        r#"

#ifdef DRAW_MODE_line
    uniform vec2 u_stageSize;
    uniform float u_lineThickness;
    uniform float u_lineLength;
    // The X and Y components of u_penPoints hold the first pen point. The Z and W components hold the difference between
    // the second pen point and the first. This is done because calculating the difference in the shader leads to floating-
    // point error when both points have large-ish coordinates.
    uniform vec4 u_penPoints;

    // Add this to divisors to prevent division by 0, which results in NaNs propagating through calculations.
    // Smaller values can cause problems on some mobile devices.
    const float epsilon = 1e-3;
#endif

#if !(defined(DRAW_MODE_line) || defined(DRAW_MODE_background))
    uniform mat4 u_projectionMatrix;
    uniform mat4 u_modelMatrix;
    in vec2 a_texCoord;
#endif

in vec2 a_position;

out vec2 v_texCoord;

void main() {
	#ifdef DRAW_MODE_line
        // Calculate a rotated ("tight") bounding box around the two pen points.
        // Yes, we're doing this 6 times (once per vertex), but on actual GPU hardware,
        // it's still faster than doing it in JS combined with the cost of uniformMatrix4fv.

        // Expand line bounds by sqrt(2) / 2 each side-- this ensures that all antialiased pixels
        // fall within the quad, even at a 45-degree diagonal
        vec2 position = a_position;
        float expandedRadius = (u_lineThickness * 0.5) + 1.4142135623730951;

        // The X coordinate increases along the length of the line. It's 0 at the center of the origin point
        // and is in pixel-space (so at n pixels along the line, its value is n).
        v_texCoord.x = mix(0.0, u_lineLength + (expandedRadius * 2.0), a_position.x) - expandedRadius;
        // The Y coordinate is perpendicular to the line. It's also in pixel-space.
        v_texCoord.y = ((a_position.y - 0.5) * expandedRadius) + 0.5;

        position.x *= u_lineLength + (2.0 * expandedRadius);
        position.y *= 2.0 * expandedRadius;

        // 1. Center around first pen point
        position -= expandedRadius;

        // 2. Rotate quad to line angle
        vec2 pointDiff = u_penPoints.zw;
        // Ensure line has a nonzero length so it's rendered properly
        // As long as either component is nonzero, the line length will be nonzero
        // If the line is zero-length, give it a bit of horizontal length
        pointDiff.x = (abs(pointDiff.x) < epsilon && abs(pointDiff.y) < epsilon) ? epsilon : pointDiff.x;
        // The `normalized` vector holds rotational values equivalent to sine/cosine
        // We're applying the standard rotation matrix formula to the position to rotate the quad to the line angle
        // pointDiff can hold large values so we must divide by u_lineLength instead of calling GLSL's normalize function:
        // https://asawicki.info/news_1596_watch_out_for_reduced_precision_normalizelength_in_opengl_es
        vec2 normalized = pointDiff / max(u_lineLength, epsilon);
        position = mat2(normalized.x, normalized.y, -normalized.y, normalized.x) * position;

        // 3. Translate quad
        position += u_penPoints.xy;

        // 4. Apply view transform
        position *= 2.0 / u_stageSize;
        gl_Position = vec4(position, 0, 1);
	#elif defined(DRAW_MODE_background)
        gl_Position = vec4(a_position * 2.0, 0, 1);
	#else
        gl_Position = u_projectionMatrix * u_modelMatrix * vec4(a_position, 0, 1);
        v_texCoord = a_texCoord;
	#endif
}
"#
    }

    fn get_fragment_shader() -> &'static str {
        r#"

#ifdef DRAW_MODE_silhouette
    uniform vec4 u_silhouetteColor;
#else // DRAW_MODE_silhouette
    # ifdef ENABLE_color
        uniform float u_color;
    # endif // ENABLE_color
    # ifdef ENABLE_brightness
        uniform float u_brightness;
    # endif // ENABLE_brightness
#endif // DRAW_MODE_silhouette

#ifdef DRAW_MODE_colorMask
    uniform vec3 u_colorMask;
    uniform float u_colorMaskTolerance;
#endif // DRAW_MODE_colorMask

#ifdef ENABLE_fisheye
    uniform float u_fisheye;
#endif // ENABLE_fisheye
#ifdef ENABLE_whirl
    uniform float u_whirl;
#endif // ENABLE_whirl
#ifdef ENABLE_pixelate
    uniform float u_pixelate;
    uniform vec2 u_skinSize;
#endif // ENABLE_pixelate
#ifdef ENABLE_mosaic
    uniform float u_mosaic;
#endif // ENABLE_mosaic
#ifdef ENABLE_ghost
    uniform float u_ghost;
#endif // ENABLE_ghost

#ifdef DRAW_MODE_line
    uniform vec4 u_lineColor;
    uniform float u_lineThickness;
    uniform float u_lineLength;
#endif // DRAW_MODE_line

#ifdef DRAW_MODE_background
    uniform vec4 u_backgroundColor;
#endif // DRAW_MODE_background

uniform sampler2D u_skin;

#ifndef DRAW_MODE_background
    in vec2 v_texCoord;
#endif

// Add this to divisors to prevent division by 0, which results in NaNs propagating through calculations.
// Smaller values can cause problems on some mobile devices.
const float epsilon = 1e-3;

#if !defined(DRAW_MODE_silhouette) && (defined(ENABLE_color))
    // Branchless color conversions based on code from:
    // http://www.chilliant.com/rgb2hsv.html by Ian Taylor
    // Based in part on work by Sam Hocevar and Emil Persson
    // See also: https://en.wikipedia.org/wiki/HSL_and_HSV#Formal_derivation


    // Convert an RGB color to Hue, Saturation, and Value.
    // All components of input and output are expected to be in the [0,1] range.
    vec3 convertRGB2HSV(vec3 rgb) {
        // Hue calculation has 3 cases, depending on which RGB component is largest, and one of those cases involves a "mod"
        // operation. In order to avoid that "mod" we split the M==R case in two: one for G<B and one for B>G. The B>G case
        // will be calculated in the negative and fed through abs() in the hue calculation at the end.
        // See also: https://en.wikipedia.org/wiki/HSL_and_HSV#Hue_and_chroma
        const vec4 hueOffsets = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);

        // temp1.xy = sort B & G (largest first)
        // temp1.z = the hue offset we'll use if it turns out that R is the largest component (M==R)
        // temp1.w = the hue offset we'll use if it turns out that R is not the largest component (M==G or M==B)
        vec4 temp1 = rgb.b > rgb.g ? vec4(rgb.bg, hueOffsets.wz) : vec4(rgb.gb, hueOffsets.xy);

        // temp2.x = the largest component of RGB ("M" / "Max")
        // temp2.yw = the smaller components of RGB, ordered for the hue calculation (not necessarily sorted by magnitude!)
        // temp2.z = the hue offset we'll use in the hue calculation
        vec4 temp2 = rgb.r > temp1.x ? vec4(rgb.r, temp1.yzx) : vec4(temp1.xyw, rgb.r);

        // m = the smallest component of RGB ("min")
        float m = min(temp2.y, temp2.w);

        // Chroma = M - m
        float C = temp2.x - m;

        // Value = M
        float V = temp2.x;

        return vec3(
            abs(temp2.z + (temp2.w - temp2.y) / (6.0 * C + epsilon)), // Hue
            C / (temp2.x + epsilon), // Saturation
            V); // Value
    }

    vec3 convertHue2RGB(float hue) {
        float r = abs(hue * 6.0 - 3.0) - 1.0;
        float g = 2.0 - abs(hue * 6.0 - 2.0);
        float b = 2.0 - abs(hue * 6.0 - 4.0);
        return clamp(vec3(r, g, b), 0.0, 1.0);
    }

    vec3 convertHSV2RGB(vec3 hsv) {
        vec3 rgb = convertHue2RGB(hsv.x);
        float c = hsv.z * hsv.y;
        return rgb * c + hsv.z - c;
    }
#endif // !defined(DRAW_MODE_silhouette) && (defined(ENABLE_color))

const vec2 kCenter = vec2(0.5, 0.5);

void main() {
	#if !(defined(DRAW_MODE_line) || defined(DRAW_MODE_background))
        vec2 texcoord0 = v_texCoord;

        #ifdef ENABLE_mosaic
            texcoord0 = fract(u_mosaic * texcoord0);
        #endif // ENABLE_mosaic

        #ifdef ENABLE_pixelate
            {
                // TODO: clean up "pixel" edges
                vec2 pixelTexelSize = u_skinSize / u_pixelate;
                texcoord0 = (floor(texcoord0 * pixelTexelSize) + kCenter) / pixelTexelSize;
            }
        #endif // ENABLE_pixelate

        #ifdef ENABLE_whirl
            {
                const float kRadius = 0.5;
                vec2 offset = texcoord0 - kCenter;
                float offsetMagnitude = length(offset);
                float whirlFactor = max(1.0 - (offsetMagnitude / kRadius), 0.0);
                float whirlActual = u_whirl * whirlFactor * whirlFactor;
                float sinWhirl = sin(whirlActual);
                float cosWhirl = cos(whirlActual);
                mat2 rotationMatrix = mat2(
                    cosWhirl, -sinWhirl,
                    sinWhirl, cosWhirl
                );

                texcoord0 = rotationMatrix * offset + kCenter;
            }
        #endif // ENABLE_whirl

        #ifdef ENABLE_fisheye
            {
                vec2 vec = (texcoord0 - kCenter) / kCenter;
                float vecLength = length(vec);
                float r = pow(min(vecLength, 1.0), u_fisheye) * max(1.0, vecLength);
                vec2 unit = vec / vecLength;

                texcoord0 = kCenter + r * unit * kCenter;
            }
        #endif // ENABLE_fisheye

        gl_FragColor = texture2D(u_skin, texcoord0);

        #if defined(ENABLE_color) || defined(ENABLE_brightness)
            // Divide premultiplied alpha values for proper color processing
            // Add epsilon to avoid dividing by 0 for fully transparent pixels
            gl_FragColor.rgb = clamp(gl_FragColor.rgb / (gl_FragColor.a + epsilon), 0.0, 1.0);

            #ifdef ENABLE_color
                {
                    vec3 hsv = convertRGB2HSV(gl_FragColor.xyz);

                    // this code forces grayscale values to be slightly saturated
                    // so that some slight change of hue will be visible
                    const float minLightness = 0.11 / 2.0;
                    const float minSaturation = 0.09;
                    if (hsv.z < minLightness) hsv = vec3(0.0, 1.0, minLightness);
                    else if (hsv.y < minSaturation) hsv = vec3(0.0, minSaturation, hsv.z);

                    hsv.x = mod(hsv.x + u_color, 1.0);
                    if (hsv.x < 0.0) hsv.x += 1.0;

                    gl_FragColor.rgb = convertHSV2RGB(hsv);
                }
            #endif // ENABLE_color

            #ifdef ENABLE_brightness
                gl_FragColor.rgb = clamp(gl_FragColor.rgb + vec3(u_brightness), vec3(0), vec3(1));
            #endif // ENABLE_brightness

            // Re-multiply color values
            gl_FragColor.rgb *= gl_FragColor.a + epsilon;

        #endif // defined(ENABLE_color) || defined(ENABLE_brightness)

        #ifdef ENABLE_ghost
            gl_FragColor *= u_ghost;
        #endif // ENABLE_ghost

        #ifdef DRAW_MODE_silhouette
            // Discard fully transparent pixels for stencil test
            if (gl_FragColor.a == 0.0) {
                discard;
            }
            // switch to u_silhouetteColor only AFTER the alpha test
            gl_FragColor = u_silhouetteColor;
        #else // DRAW_MODE_silhouette

            #ifdef DRAW_MODE_colorMask
                vec3 maskDistance = abs(gl_FragColor.rgb - u_colorMask);
                vec3 colorMaskTolerance = vec3(u_colorMaskTolerance, u_colorMaskTolerance, u_colorMaskTolerance);
                if (any(greaterThan(maskDistance, colorMaskTolerance)))
                {
                    discard;
                }
            #endif // DRAW_MODE_colorMask
        #endif // DRAW_MODE_silhouette

        #ifdef DRAW_MODE_straightAlpha
            // Un-premultiply alpha.
            gl_FragColor.rgb /= gl_FragColor.a + epsilon;
        #endif

	#endif // !(defined(DRAW_MODE_line) || defined(DRAW_MODE_background))

	#ifdef DRAW_MODE_line
        // Maaaaagic antialiased-line-with-round-caps shader.

        // "along-the-lineness". This increases parallel to the line.
        // It goes from negative before the start point, to 0.5 through the start to the end, then ramps up again
        // past the end point.
        float d = ((v_texCoord.x - clamp(v_texCoord.x, 0.0, u_lineLength)) * 0.5) + 0.5;

        // Distance from (0.5, 0.5) to (d, the perpendicular coordinate). When we're in the middle of the line,
        // d will be 0.5, so the distance will be 0 at points close to the line and will grow at points further from it.
        // For the "caps", d will ramp down/up, giving us rounding.
        // See https://www.youtube.com/watch?v=PMltMdi1Wzg for a rough outline of the technique used to round the lines.
        float line = distance(vec2(0.5), vec2(d, v_texCoord.y)) * 2.0;
        // Expand out the line by its thickness.
        line -= ((u_lineThickness - 1.0) * 0.5);
        // Because "distance to the center of the line" decreases the closer we get to the line, but we want more opacity
        // the closer we are to the line, invert it.
        gl_FragColor = u_lineColor * clamp(1.0 - line, 0.0, 1.0);
	#endif // DRAW_MODE_line

	#ifdef DRAW_MODE_background
        gl_FragColor = u_backgroundColor;
	#endif
}
"#
    }
}

fn get_texture_from_path(
    window: &SDL2Facade,
    path: PathBuf,
    scale: f32,
) -> Result<glium::texture::Texture2d, &'static str> {
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

    let image =
        image::load_from_memory_with_format(&pixmap.encode_png().unwrap(), image::ImageFormat::Png)
            .or(Err("Cannot load rendered svg file."))?
            .to_rgba8();
    let image_dimensions = image.dimensions();
    let image =
        glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);

    let texture = glium::texture::Texture2d::new(window, image).unwrap();

    Ok(texture)
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
    sounds: Vec<Sound>,
    volume: f32,
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
            sounds: Vec::new(),
            volume: 100.0,
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
            sounds: self.sounds,
            volume: self.volume,
            sprites: Vec::new(),
            keyboard: Keyboard::new(),
            mouse: Mouse::new(),
            stamps: Vec::new(),
            answer: Value::from(String::new()),
            threads_to_add: VecDeque::new(),
            effects: HashMap::new(),
            need_to_recompile_shaders: false,
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
    pub fn add_sound(mut self, sound: Sound) -> Self {
        self.sounds.push(sound);
        self
    }
    pub fn set_volume(mut self, volume: f32) -> Self {
        self.volume = volume;
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
    /// A list of sounds owned by the stage.
    sounds: Vec<Sound>,
    /// The volume for the stage.
    volume: f32,
    keyboard: Keyboard,
    mouse: Mouse,
    /// A list of references to sprites.
    sprites: Vec<Rc<Mutex<Sprite>>>,

    stamps: Vec<Stamp>,
    /// The current value of answer.
    answer: Value,

    threads_to_add: VecDeque<Thread>,

    effects: HashMap<Effect, f32>,

    need_to_recompile_shaders: bool,
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
        self.sprites
            .clone()
            .into_iter()
            .find(|sprite| sprite.lock().unwrap().name == name)
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
    keys_pressed: HashSet<Keycode>,
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

    fn press_key(&mut self, key: Keycode) {
        self.keys_pressed.insert(key);
    }

    fn release_key(&mut self, key: Keycode) {
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

    fn scratch_key_to_Key(&self, arg: String) -> Option<Keycode> {
        Some(match &*arg {
            "A" => Keycode::A,
            "B" => Keycode::B,
            "C" => Keycode::C,
            "D" => Keycode::D,
            "E" => Keycode::E,
            "F" => Keycode::F,
            "G" => Keycode::G,
            "H" => Keycode::H,
            "I" => Keycode::I,
            "J" => Keycode::J,
            "K" => Keycode::K,
            "L" => Keycode::L,
            "M" => Keycode::M,
            "N" => Keycode::N,
            "O" => Keycode::O,
            "P" => Keycode::P,
            "Q" => Keycode::Q,
            "R" => Keycode::R,
            "S" => Keycode::S,
            "T" => Keycode::T,
            "U" => Keycode::U,
            "V" => Keycode::V,
            "W" => Keycode::W,
            "X" => Keycode::X,
            "Y" => Keycode::Y,
            "Z" => Keycode::Z,
            "space" => Keycode::Space,
            "left arrow" => Keycode::Left,
            "right arrow" => Keycode::Right,
            "up arrow" => Keycode::Up,
            "down arrow" => Keycode::Down,
            "SPACE" => Keycode::Space,
            "LEFT" => Keycode::Left,
            "RIGHT" => Keycode::Right,
            "UP" => Keycode::Up,
            "DOWN" => Keycode::Down,
            "0" => Keycode::Num0,
            "1" => Keycode::Num1,
            "2" => Keycode::Num2,
            "3" => Keycode::Num3,
            "4" => Keycode::Num4,
            "5" => Keycode::Num5,
            "6" => Keycode::Num6,
            "7" => Keycode::Num7,
            "8" => Keycode::Num8,
            "9" => Keycode::Num9,
            "!" => Keycode::Exclaim,
            "@" => Keycode::At,
            "#" => Keycode::Hash,
            "$" => Keycode::Dollar,
            "%" => Keycode::Percent,
            "^" => Keycode::Caret,
            "&" => Keycode::Ampersand,
            "*" => Keycode::Asterisk,
            "(" => Keycode::LeftParen,
            ")" => Keycode::RightParen,
            "`" => Keycode::Backquote,
            "-" => Keycode::Minus,
            "+" => Keycode::Plus,
            "_" => Keycode::Underscore,
            "=" => Keycode::Equals,
            "[" => Keycode::LeftBracket,
            "]" => Keycode::RightBracket,
            "\\" => Keycode::Backslash,
            ";" => Keycode::Semicolon,
            ":" => Keycode::Colon,
            "'" => Keycode::Quote,
            "\"" => Keycode::Quotedbl,
            "/" => Keycode::Slash,
            "?" => Keycode::Question,
            "." => Keycode::Period,
            "," => Keycode::Comma,
            _ => return None,
        })
    }

    fn get_key_down(&self, arg: Value) -> bool {
        if let Value::String(x) = &arg {
            if x == &String::from("any") {
                return !self.keys_pressed.is_empty();
            }
        }

        if let Some(scratchKey) = self.scratch_key_to_Key(self.key_arg_to_scratch_key(arg)) {
            self.keys_pressed.get(&scratchKey).is_some()
        } else {
            false
        }
    }
}

/// The mouse struct.
///
/// This holds the position of the mouse, both in scratch coordinates and sdl coordinates.
/// It also holds the keys that are pressed or not.
pub struct Mouse {
    scratch_position: (f32, f32),
    sdl_position: (f32, f32),
    buttons: Vec<MouseButton>,
}

impl Mouse {
    fn new() -> Self {
        Self {
            scratch_position: (0.0, 0.0),
            sdl_position: (0.0, 0.0),
            buttons: Vec::new(),
        }
    }

    /// Set the position of the mouse, in sdl coordinates.
    /// This also sets the scratch position to the correct coordinates.
    pub fn set_sdl_position(&mut self, pos: [f64; 2], window: &SDL2Facade) {
        self.sdl_position = (pos[0] as f32, pos[1] as f32);
        self.scratch_position = Mouse::sdl2scratch((pos[0] as f32, pos[1] as f32), window);
    }

    pub fn set_button_down(&mut self, button: MouseButton) {
        if self.buttons.iter().find(|x| **x == button).is_none() {
            self.buttons.push(button);
        }
    }

    pub fn set_button_up(&mut self, button: MouseButton) {
        if let Some((i, x)) = self.buttons.iter().enumerate().find(|x| *x.1 == button) {
            self.buttons.remove(i);
        }
    }

    pub fn mouse_down(&self) -> bool {
        self.buttons.contains(&MouseButton::Left)
            || self.buttons.contains(&MouseButton::Right)
            || self.buttons.contains(&MouseButton::Middle)
    }

    /// Convert piston coordinates to scratch coordinates
    fn sdl2scratch(coords: (f32, f32), window: &SDL2Facade) -> (f32, f32) {
        let (mut x, mut y) = coords;

        let (width, height) = window.window().size();

        // find the ratio of scratch size to actual size
        let ratio_x: f32 = Into::<f32>::into(SCRATCH_WIDTH) / width as f32;
        let ratio_y: f32 = Into::<f32>::into(SCRATCH_HEIGHT) / height as f32;

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
