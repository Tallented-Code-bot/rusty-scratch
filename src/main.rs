use crate::target::{RotationStyle, VideoState};
use json::{self, JsonValue};
use rand::Rng;
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Output};
use target::StartType;

use clap::Parser;

mod target;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The id of the scratch project to compile.
    id: String,
    /// Keep intermediate files (such as project.json and file.sb3)
    #[arg(short, long)]
    keep_intermediate_files: bool,
    /// Directory to put the project in. Defaults to `./output/`
    #[arg(short, long, value_name = "DIR")]
    output: Option<PathBuf>,
}

/// Parse a scratch id. This can either be a plain number,
/// or a full scratch url ("https://scratch.mit.edu/projects/ID").
fn parse_id(s: &str) -> Result<u64, ()> {
    use nom::{
        branch::alt,
        bytes::complete::tag,
        character::complete::{alphanumeric0, digit1},
        sequence::tuple,
        IResult,
    };

    /// Parse a scratch url and extract the id
    fn parse_url(s: &str) -> IResult<&str, u64> {
        tuple((alphanumeric0, tag("://scratch.mit.edu/projects/"), digit1))(s)
            .map(|x| (x.0, x.1 .2.parse().unwrap()))
    }

    fn parse_id(s: &str) -> IResult<&str, u64> {
        digit1(s).map(|x| (x.0, x.1.parse().unwrap()))
    }

    Ok(match alt((parse_url, parse_id))(s) {
        Ok((_, y)) => y,
        Err(_) => return Err(()),
    })
}

/// Creates a block reference hashmap.
/// This translates scratch code to rust code.
///
/// The function that are used here are defined in
/// `src/target/mod.rs`, in the `Sprite` and `Stage` structs,
/// and the `Target` trait.
///
/// # Usage
/// ```rust
/// // Generate the reference
/// let block_reference=make_blocks_lookup();
///
/// // Now we can use the reference in other functions.
/// follow_stack(block,block_id,block_reference);
/// ```
fn make_blocks_lookup() -> HashMap<&'static str, &'static str> {
    let mut blocks: HashMap<&str, &str> = HashMap::new();
    blocks.insert("motion_setx", "set_x(sprite.clone().unwrap(),X);");
    blocks.insert("motion_sety", "set_y(sprite.clone().unwrap(),Y);");
    blocks.insert(
        "motion_changexby",
        "change_x_by(sprite.clone().unwrap(),DX);",
    );
    blocks.insert(
        "motion_changeyby",
        "change_y_by(sprite.clone().unwrap(),DY);",
    );
    blocks.insert("motion_xposition", "get_x(sprite.clone().unwrap())");
    blocks.insert("motion_yposition", "get_y(sprite.clone().unwrap())");
    // blocks.insert("motion_movesteps", "object.move_steps(STEPSf32);");
    blocks.insert(
        "motion_movesteps",
        "move_steps(sprite.clone().unwrap(),STEPS);",
    );
    blocks.insert(
        "motion_turnleft",
        "turn_left(sprite.clone().unwrap(),DEGREES);",
    );
    blocks.insert(
        "motion_turnright",
        "turn_right(sprite.clone().unwrap(),DEGREES);",
    );
    blocks.insert(
        "motion_goto",
        "go_to(sprite.clone().unwrap(),stage.clone(),TO);",
    );
    blocks.insert("motion_gotoxy", "go_to_xy(sprite.clone().unwrap(),X,Y);");
    blocks.insert(
        "motion_pointindirection",
        "point_in_direction(sprite.clone().unwrap(),DIRECTION);",
    );
    blocks.insert(
        "motion_setrotationstyle",
        "set_rotation_style(sprite.clone().unwrap(),STYLE);",
    );
    blocks.insert(
        "sound_playuntildone",
        "Wait::new(Duration::from_secs_f32(play_sound(sprite.clone(), stage.clone(), SOUND_MENU))).await;",
    );
    blocks.insert(
        "sound_play",
        "play_sound(sprite.clone(), stage.clone(), SOUND_MENU);",
    );
    blocks.insert(
        "sound_setvolumeto",
        "set_volume(sprite.clone(), stage.clone(), VOLUME);",
    );
    blocks.insert(
        "sound_changevolumeby",
        "change_volume(sprite.clone(), stage.clone(), VOLUME);",
    );
    blocks.insert("sound_volume", "get_volume(sprite.clone(), stage.clone())");
    blocks.insert("sound_sounds_menu", "SOUND_MENU");
    blocks.insert("event_whenflagclicked", "flag_clicked();");
    blocks.insert(
        "control_repeat",
        "for z in 0..TIMES.into(){SUBSTACK\nYield::Start.await;}",
    );
    blocks.insert(
        "control_wait",
        "Wait::new(Duration::from_secs(DURATION.into())).await;",
    );
    blocks.insert("control_forever", "loop{SUBSTACK\nYield::Start.await;}");
    blocks.insert(
        "control_if",
        "if <Value as Into<bool>>::into(Value::from(CONDITION)) {SUBSTACK}",
    );
    blocks.insert(
        "control_if_else",
        "if <Value as Into<bool>>::into(Value::from(CONDITION)) {SUBSTACK}else{SUBSTACK2}",
    );
    // blocks.insert(
    //     "control_wait_until",
    //     "wait_until(Value::from(CONDITION)).await;",
    // );
    blocks.insert(
        "control_wait_until",
        "while !<Value as Into<bool>>::into(Value::from(CONDITION)){
            Yield::Start.await;
        }",
    );
    blocks.insert(
        "control_repeat_until",
        "while !<Value as Into<bool>>::into(CONDITION){SUBSTACK\nYield::Start.await;}",
    );
    blocks.insert(
        "control_create_clone_of",
        "{
            let index = stage.lock().unwrap().sprites.len();
            let sprite_name = create_clone(stage.clone(),sprite.clone().unwrap(),CLONE_OPTION);
            let mut new_threads = clone_sprite(sprite_name.clone(),stage.lock().unwrap().sprites[index].clone(),stage.clone());
            for thread in &mut new_threads{
                if thread.start == StartType::StartAsClone(format!(\"{}_clone\",sprite_name.clone())){
                    thread.running=true;
                }
            }
            stage.lock().unwrap().add_threads(new_threads.into_iter());
}",
    );
    blocks.insert("control_create_clone_of_menu", "Value::from(CLONE_OPTION)");
    blocks.insert("control_start_as_clone", "");
    blocks.insert(
        "control_delete_this_clone",
        "delete_this_clone(stage.clone(),sprite.clone().unwrap());",
    );

    // blocks.insert("looks_say", "object.say(String::from(\"MESSAGE\"));");
    blocks.insert("looks_say", "say(MESSAGE);");
    blocks.insert(
        "looks_switchbackdropto",
        "switch_backdrop(stage.clone(),BACKDROP);",
    );
    blocks.insert(
        "looks_backdrops",
        "switch_backdrop(stage.clone(),BACKDROP);",
    ); // legacy? not included in scratch 3 opcodes
    blocks.insert(
        "looks_nextcostume",
        "next_costume(sprite.clone().unwrap());",
    );
    blocks.insert(
        "looks_switchcostumeto",
        "switch_costume(sprite.clone().unwrap(),COSTUME);",
    );
    blocks.insert("looks_setsizeto", "set_size(sprite.clone().unwrap(),SIZE);");
    blocks.insert("looks_show", "show(sprite.clone().unwrap());");
    blocks.insert("looks_hide", "hide(sprite.clone().unwrap());");
    blocks.insert(
        "looks_gotofrontback",
        "go_to_front_or_back(sprite.clone().unwrap(),stage.clone(),FRONT_BACK);",
    );
    blocks.insert(
        "looks_goforwardbackwardlayers",
        "go_forward_backwards_layers(sprite.clone().unwrap(),stage.clone(),FORWARD_BACKWARD,NUM);",
    );
    blocks.insert(
        "looks_seteffectto",
        "set_effect(sprite.clone(), stage.clone(), EFFECT, VALUE);",
    );
    blocks.insert(
        "looks_changeeffectby",
        "change_effect(sprite.clone(), stage.clone(), EFFECT, CHANGE);",
    );
    blocks.insert(
        "looks_cleargraphiceffects",
        "clear_effects(sprite.clone(), stage.clone());",
    );
    blocks.insert("event_whenflagclicked", "");
    // blocks.insert(
    //     "data_variable",
    //     "get_variable(Some(sprite.clone()),stage.clone(),VARIABLE)",
    // );
    blocks.insert(
        "data_setvariableto",
        "set_variable(sprite.clone(),stage.clone(),VARIABLE,VALUE);",
    );
    blocks.insert(
        "data_changevariableby",
        "change_variable(sprite.clone(),stage.clone(),VARIABLE,VALUE);",
    );
    blocks.insert(
        "data_addtolist",
        "add_to_list(sprite.clone(),stage.clone(),ITEM,LIST);",
    );
    blocks.insert(
        "data_deleteoflist",
        "delete_from_list(sprite.clone(),stage.clone(),INDEX,LIST);",
    );
    blocks.insert(
        "data_deletealloflist",
        "delete_all_of_list(sprite.clone(),stage.clone(),LIST);",
    );
    blocks.insert(
        "data_insertatlist",
        "insert_item_in_list(sprite.clone(),stage.clone(),ITEM,INDEX,LIST);",
    );
    blocks.insert(
        "data_replaceitemoflist",
        "replace_item_in_list(sprite.clone(),stage.clone(),ITEM,INDEX,LIST);",
    );
    blocks.insert(
        "data_itemoflist",
        "get_item_of_list(sprite.clone(),stage.clone(),INDEX,LIST)",
    );
    blocks.insert(
        "data_itemnumoflist",
        "get_item_num_in_list(Some(sprite.clone()),stage.clone(),ITEM,LIST)",
    );
    blocks.insert(
        "data_lengthoflist",
        "length_of_list(sprite.clone(),stage.clone(),LIST)",
    );
    blocks.insert(
        "data_listcontainsitem",
        "list_contains_item(sprite.clone(),stage.clone(),LIST,ITEM)",
    );
    blocks.insert("operator_add", "NUM1+NUM2");
    blocks.insert("operator_subtract", "NUM1-NUM2");
    blocks.insert("operator_multiply", "NUM1*NUM2");
    blocks.insert("operator_divide", "NUM1/NUM2");
    blocks.insert("operator_random", "generate_random(FROM,TO)");
    blocks.insert("operator_lt", "Value::Bool(OPERAND1<OPERAND2)");
    blocks.insert("operator_equals", "Value::Bool(OPERAND1==OPERAND2)");
    blocks.insert("operator_gt", "Value::Bool(OPERAND1>OPERAND2)");
    blocks.insert(
        "operator_and",
        "Value::Bool(OPERAND1.into() && OPERAND2.into())",
    );
    blocks.insert(
        "operator_or",
        "Value::Bool(OPERAND1.into()||OPERAND2.into())",
    );
    blocks.insert("operator_not", "!OPERAND");
    blocks.insert("operator_join", "join(STRING1,STRING2)");
    blocks.insert("operator_letter_of", "letter_of(LETTER,STRING)");
    blocks.insert("operator_length", "length(STRING)");
    blocks.insert("operator_contains", "contains(STRING1,STRING2)");
    blocks.insert("operator_round", "round(NUM)");
    blocks.insert("operator_mod", "modulus(NUM1,NUM2)");
    blocks.insert("operator_mathop", "mathop(OPERATOR,NUM)");
    blocks.insert("argument_reporter_string_number", "VALUE");
    blocks.insert(
        "sensing_keypressed",
        "key_pressed(stage.clone(),KEY_OPTION)",
    );
    blocks.insert("sensing_mousex", "mousex(stage.clone())");
    blocks.insert("sensing_mousey", "mousey(stage.clone())");
    blocks.insert("sensing_mousedown", "mouse_down(stage.clone())");
    blocks.insert("sensing_askandwait", "ask(stage.clone(),QUESTION);");
    blocks.insert("sensing_answer", "answer(stage.clone())");
    blocks.insert("sensing_username", "username()");
    blocks.insert("sensing_keyoptions", "Value::from(KEY_OPTION)");
    blocks.insert("motion_goto_menu", "Value::from(TO)");

    blocks.insert("sensing_dayssince2000", "days_since_2000()");
    blocks.insert("pen_stamp", "stamp(sprite.clone().unwrap(),stage.clone());");
    blocks.insert("pen_clear", "clear_pen(stage.clone());");

    blocks
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let id = parse_id(&cli.id).or(Err(
        "Cannot parse id. Must be either a full scratch url or a number.",
    ))?;

    let project = get_project_online(id as u32)?;
    let project_details = get_project_details(id)?;
    let output = match cli.output {
        Some(o) => o,
        None => PathBuf::from("./output/"),
    };

    println!(
        "Compiling project https://scratch.mit.edu/projects/{} ({} by {})",
        id, project_details["title"], project_details["author"]["username"]
    );

    if cli.keep_intermediate_files {
        std::fs::write("project.json", project.to_string())?;
    }
    let block_reference = make_blocks_lookup();
    create_project(&output)?; //.expect("Could not create new rust project"); // create a new cargo project
                              //510186917
    let readme = {
        let mut f = output.clone();
        f.push("README.md");
        f
    };

    fs::write(readme, create_readme(&project_details)?)?;

    let filename = {
        let mut f = output.clone();
        f.push("src");
        f.push("main.rs");
        f
    };

    // Get the library file to include
    let lib = include_str!("../target/target.rs");

    let mut targets: Vec<String> = Vec::new();

    let mut target_clone_fns = Vec::new();

    for (i, target) in project["targets"].members().enumerate() {
        println!(
            "[{}/{}] Compiling code for {}...",
            i + 1,
            project["targets"].len(),
            target["name"]
        );
        targets.push(generate_target(target, &block_reference)?);
        get_target_assets(target, &output)?;

        target_clone_fns.push(format!(
            "\"{name}\" => clone_{name}(target.clone(),stage.clone()),",
            name = target["name"]
        ));
    }

    let output = format!(
        "
        // This is the static Sprite, Stage, and block definitions
        {lib}
        //########################################
        // Below this is generated code.

        fn main(){{
            let sdl_context = sdl2::init().unwrap();
            let video_subsystem = sdl_context.video().unwrap();

            let window = video_subsystem.window(\"rusty-scratch\", 600,400)
                .opengl()
                .build_glium()
                .unwrap();


            let mut event_pump = sdl_context.event_pump().unwrap();


            let mut keyboard = Keyboard::new();
            let mut mouse = Mouse::new();


            let mut program=Program::new(&window);
            let mut sprites: Vec<Rc<Mutex<Sprite>>> = Vec::new();

            {targets}
            // (Sprite1.blocks.function)(&mut Sprite1);
            
            // program.add_threads(Sprite1.blocks);
            // program.add_all_threads();


            fn clone_sprite(sprite: String, target:Rc<Mutex<Sprite>>, stage:Rc<Mutex<Stage>>) -> Vec<Thread>{{
                match &*sprite{{
                    {clone_content}
                    _ => panic!(\"Should always have sprite to clone\")
                }}
            }}

            {{Stage.lock().unwrap().sprites.sort_by(|a,b| a.lock().unwrap().layer.cmp(&b.lock().unwrap().layer));}}

            program.click_flag();
            'running: loop{{
                program.tick(Stage.clone());
                program.render(Stage.clone());

                for event in event_pump.poll_iter(){{
                    use sdl2::event::Event;

                    match event{{
                        Event::Quit {{..}} => {{break 'running;}},
                        Event::KeyDown {{keycode: Some(key), ..}} => {{
                            let mut s = Stage.lock().unwrap();
                            s.keyboard.press_key(key);
                        }},
                        Event::KeyUp {{keycode: Some(key), ..}} => {{
                            let mut s = Stage.lock().unwrap();
                            s.keyboard.release_key(key)
                        }},
                        Event::MouseMotion{{mousestate: m, x, y, ..}} => {{
                            let mut s = Stage.lock().unwrap();
                            s.mouse.set_sdl_position([x as f64, y as f64], &window);
                        }},
                        Event::MouseButtonDown{{mouse_btn: m, ..}} => {{
                            let mut s = Stage.lock().unwrap();
                            s.mouse.set_button_down(m);
                        }}
                        Event::MouseButtonUp{{mouse_btn: m, ..}} => {{
                            let mut s = Stage.lock().unwrap();
                            s.mouse.set_button_up(m);
                        }}
                        _ => ()
                    }}
                }}
            }}


            // music::start_context::<Value,Value,_>(&sdl, 16, || {{

            // initialize_sounds(Stage.clone());

            // let mut events = Events::new(EventSettings::new());
            // events.set_max_fps(30);
            // events.set_ups(30);
            // program.click_flag();
            // while let Some(e) = events.next(&mut window){{
            //     if let Some(args) = e.render_args(){{
            //         program.render(&args,Stage.clone(),window.size());
            //     }}
            //     if let Some(args) = e.update_args(){{
            //         program.tick(Stage.clone());
            //
            //     }}
            //     if let Some(Button::Keyboard(key)) = e.press_args(){{
            //         let mut s=Stage.lock().unwrap();
            //         s.keyboard.press_key(key);
            //         //println!(\"Pressed {{:?}}\",key);
            //         //println!(\"{{:?}}\",s.keyboard);
            //     }}
            //     if let Some(Button::Keyboard(key)) = e.release_args(){{
            //         let mut s=Stage.lock().unwrap();
            //         s.keyboard.release_key(key);
            //         //println!(\"Released {{:?}}\",key);
            //     }}
            //     if let Some(pos) = e.mouse_cursor_args(){{
            //         let mut s = Stage.lock().unwrap();
            //         s.mouse.set_piston_position(pos,window.size());
            //         //mouse.set_piston_position(pos,window.size());
            //         //println!(\"Mouse moved to {{}}\",s.mouse);
            //     }}
            // }}

         // }}

         // );

        }}
        ",
        lib = lib,
        targets = targets.join("\n"),
        clone_content = target_clone_fns.join("\n"),
        // sprite1 = generate_target(&project["targets"][1], &block_reference)
    );

    fs::write(&filename, output)?;
    format_file(&filename)?;

    // ###############################################

    // write_to_file(
    //     (
    //         "l;hbH%/NGXC+[%R,/D9_",
    //         &project["targets"][1]["blocks"]["l;hbH%/NGXC+[%R,/D9_"],
    //     ),
    //     &project["targets"][1]["blocks"],
    //     &block_reference,
    // )
    Ok(())
}

/// Get an online scratch project.
fn get_project_online(id: u32) -> Result<JsonValue, Box<dyn Error>> {
    // Create the project url.
    let token = fetch_project_token(id)?;
    let url = format!("https://projects.scratch.mit.edu/{id}?token={token}");

    // Get the project (`fetch_sb3_file` creates a file called "project.json".
    // I have not yet figured out how to get the project assets.)
    let project = json::parse(fetch_sb3_file(url).as_str())?;

    Ok(project)
}

/// Turn 1 block into a rust function.  If the block
/// has a substack(a block such as a loop, or an if-statement),
/// then the substack will also be returned inside the main block.
fn get_block(
    block: (&str, &JsonValue),
    blocks: &JsonValue,
    block_reference: &HashMap<&str, &str>,
) -> Result<String, String> {
    let (_id, data) = block;
    let opcode = data["opcode"].to_string();
    // println!("{}", block.1);
    let mut function;
    if opcode == "procedures_call" {
        let mut cblock = data["mutation"]["proccode"].to_string();
        cblock = cblock.replace(" %s", "_percent_s");
        cblock = cblock.replace(" %b", "_percent_b");
        cblock = cblock.replace(' ', "_");
        cblock = cblock.to_lowercase();

        let mut arguments = "".to_string();

        let argument_names =
            json::parse(data["mutation"]["argumentids"].as_str().unwrap()).unwrap();

        for arg in argument_names.members() {
            arguments += format!(", {}", arg).as_str();
        }

        function = format!(
            "stack_procedures_definition_{}(sprite.clone(),stage.clone(){arguments}).await;",
            cblock
        );
    } else {
        function = match block_reference.get(&opcode as &str) {
            Some(x) => x.to_string(),
            None => {
                return Err(format!("Error: unknown block (opcode {})", opcode));
            }
        };
    }

    // iterate over each input
    for input in data["inputs"].entries() {
        if input.1[1].is_array() {
            // If the input is an array, it must be a single value.
            //println!("{}", input.1[1]);

            function = match input.1[1][0].as_u32().unwrap() {
                4|5|6|7|8 => function.replacen(
                    input.0,
                    &format!(
                        "Value::from({})",
                        &input.1[1][1].as_str().unwrap().to_string()
                    ),
                    1,
                ), // Number
                // 5 => function.replacen(input.0, &input.1[1][1].as_str().unwrap().to_string(), 1), // Positive number
                // 6 => function.replacen(input.0, &input.1[1][1].as_str().unwrap().to_string(), 1), // Positive integer
                // 7 => function.replacen(input.0, &input.1[1][1].as_str().unwrap().to_string(), 1), // Integer
                // 8 => function.replacen(input.0, &input.1[1][1].as_str().unwrap().to_string(), 1), // Angle
                9 => function.replacen(input.0, input.1[1][1].as_str().unwrap(), 1), // Color
                10 => function.replacen(
                    input.0,
                    &format!(
                        "Value::from(String::from(r###\"{}\"###))",
                        input.1[1][1].as_str().unwrap()
                    ),
                    1,
                ), // String
                11 => todo!(),
                12 => function.replacen(
                    // Variable
                    input.0,
                    format!(
                        "get_variable(sprite.clone(),stage.clone(),\"{}\")",
                        input.1[1][2].as_str().unwrap()
                    )
                    .as_str(),
                    1,
                ),
                // list
                13 => function.replacen(
                    input.0,
                    &format!("get_list_contents(sprite.clone(),stage.clone(),(\"{}\".to_string(),\"{}\".to_string()))",input.1[1][1],input.1[1][2]),
                    1,
                ),
                _ => {
                    unreachable!()
                }
            };
            function = function.replacen(input.0, input.1[1][1].as_str().unwrap(), 1);
        } else if input.1[1].is_string() {
            // otherwise, it must be a substack.
            // TODO get more than the first block

            // Recursively follow the substack...
            let subfunc = follow_stack(
                (
                    input.1[1].as_str().unwrap(), // The id of the block we want to go to
                    &blocks[input.1[1].as_str().unwrap()], //The object corresponding to the id
                ),
                blocks,          // list of blocks
                block_reference, //block reference
            )?
            .join("\n");
            // ... and insert the subfunction in the main function variable.
            function = function.replacen(input.0, &subfunc, 1);
        }
    }

    for field in data["fields"].entries() {
        match &*opcode {
            "data_setvariableto"
            | "data_changevariableby"
            | "data_hidevariable"
            | "data_showvariable"
            | "data_addtolist"
            | "data_deleteoflist"
            | "data_deletealloflist"
            | "data_insertatlist"
            | "data_replaceitemoflist"
            | "data_itemoflist"
            | "data_itemnumoflist"
            | "data_lengthoflist"
            | "data_listcontainsitem"
            | "data_hidelist"
            | "data_showlist" => {
                function = function.replacen(
                    field.0,
                    &format!(
                        "(String::from(\"{}\"),String::from(\"{}\"))",
                        field.1[0], // name
                        field.1[1]  // id
                    ),
                    1,
                );
            }
            "argument_reporter_string_number" => {
                function = function.replacen(field.0, &format!("{}.clone()", field.1[0]), 1);
                function = function.to_lowercase();
            }

            _ => {
                function = function.replacen(
                    field.0,
                    &format!(
                        "Value::from(String::from(\"{}\"))",
                        field.1[0].as_str().unwrap()
                    ),
                    1,
                );
            }
        }
    }

    // Replace any remaining placeholder values (eg "SUBSTACK" for empty loops)
    // so we don't get errors.
    // let re = Regex::new("[A-Z_-]{2,}").unwrap();
    let re = Regex::new("SUBSTACK").unwrap();
    function = re.replace_all(&function, "").to_string();

    // Return the completed function
    Ok(function)
}

/// Follow a stack of scratch blocks.
fn follow_stack(
    block: (&str, &JsonValue),
    blocks: &JsonValue,
    block_reference: &HashMap<&str, &str>,
) -> Result<Vec<String>, String> {
    // Create a list of functions
    let mut stack: Vec<String> = Vec::new();

    let mut current_block = block;

    loop {
        // push the current block onto the stack.
        stack.push(get_block(current_block, blocks, block_reference)?);

        // If there is no next block, break out of the loop.
        if current_block.1["next"].is_null() {
            break;
        }
        current_block = (
            current_block.1["next"].as_str().unwrap(),
            &blocks[current_block.1["next"].as_str().unwrap()],
        );
    }
    Ok(stack)
}
fn handle_custom_block(
    block: (&str, &JsonValue),
    blocks: &JsonValue,
    block_reference: &HashMap<&str, &str>,
) -> Option<(String, String)> {
    if block.1["opcode"] != "procedures_definition" {
        return None;
    }

    // the "inner" block definition
    let prototype = (
        block.1["inputs"]["custom_block"][1].as_str().unwrap(),
        &blocks[block.1["inputs"]["custom_block"][1].as_str().unwrap()],
    );

    let input_names = json::parse(&prototype.1["mutation"]["argumentnames"].to_string()).unwrap();
    let input_ids = json::parse(&prototype.1["mutation"]["argumentids"].to_string()).unwrap();
    let mut inputs = HashMap::new();

    for (id, name) in input_ids.members().zip(input_names.members()) {
        inputs.insert(id.to_string(), name.to_string());
    }

    let mut function = follow_stack(
        (
            &block.1["next"].to_string(),
            &blocks[block.1["next"].to_string()],
        ),
        blocks,
        block_reference,
    )
    .ok()?
    .join("\n");

    // If run without screen refresh is enabled, remove all
    // yields.
    //
    let warp = &prototype.1["mutation"]["warp"];
    if (warp.is_boolean() && warp.as_bool().expect("Must be bool"))
        || (warp.is_string() && warp.as_str().expect("Must be str") == "true")
    {
        function = function.replace("yield_!(Some(object));", "");
    }

    let proccode = prototype.1["mutation"]["test_block_1"].to_string();

    // format!("{}", function);

    /*
     * let input_1=XKCD;
     * move_steps(input_1);
     * change_size(input_1);
     */

    Some((proccode, function))
}

/// Create a hat block definition function.
fn create_hat(
    block: (&str, &JsonValue),
    blocks: &JsonValue,
    block_reference: &HashMap<&str, &str>,
    sprite_name: String,
) -> Result<(String, StartType, String, String, bool), String> {
    // Make sure the block is a top level block.
    if !block.1["topLevel"].as_bool().unwrap() {
        return Err(String::from("Not a top level block"));
    }
    // Make sure the block has no parent.
    if !block.1["parent"].is_null() {
        return Err(String::from("Block has a parent"));
    }

    // Make sure the block is a hat block
    match block.1["opcode"].as_str().unwrap() {
        "event_whenflagclicked"
        | "event_whenkeypressed"
        | "event_whenthisspriteclicked"
        | "event_whentouchingobject"
        | "event_whenstageclicked"
        | "event_whenbackdropswitchesto"
        | "event_whengreaterthan"
        | "event_whenbroadcastrecieved"
        | "control_start_as_clone"
        | "procedures_definition"
        | "procedures_prototype" => {}
        _ => return Err(String::from("Not a hat block")),
    }

    let start_type = match block.1["opcode"].as_str().unwrap() {
        "procedures_call" => return Err(String::from("Custom block")),
        "event_whenflagclicked" => StartType::FlagClicked,
        "control_start_as_clone" => StartType::StartAsClone(format!("{}_clone", sprite_name)),
        "procedures_define" => StartType::NoStart,
        _ => StartType::NoStart,
    };

    // if let Some(x) = handle_custom_block(block, blocks, block_reference) {
    //     return Ok(x);
    // }

    // Get the contents of the stack
    let contents = follow_stack(
        (
            &block.1["next"].to_string(),         // the ID of the next block
            &blocks[block.1["next"].to_string()], // the next block
        ),
        blocks,
        block_reference,
    )?;

    let mut function = contents.join("\n");

    let mut arguments = "".to_string();

    let mut rng = rand::thread_rng();
    let mut custom_block = false;
    let name = if block.1["opcode"] == "procedures_definition" {
        let prototype = &blocks[block.1["inputs"]["custom_block"][1].to_string()];
        custom_block = true;
        if prototype["mutation"]["warp"] == "true" {
            function = function.replace("Yield::Start.await;", ""); // remove all yields
        }

        // the argument list is stored as an array _inside_ a string, so we have to parse it.
        let argument_names = json::parse(
            &prototype["mutation"]["argumentnames"]
                .as_str()
                .unwrap()
                .to_lowercase(),
        )
        .unwrap();

        for arg in argument_names.members() {
            arguments += format!(", {}: Value", arg).as_str();
        }

        let mut proccode = prototype["mutation"]["proccode"].to_string();
        proccode = proccode.replace(" %s", "_percent_s");
        proccode = proccode.replace(" %b", "_percent_b");
        proccode = proccode.replace(' ', "_");
        proccode = proccode.to_lowercase();
        format!("procedures_definition_{}", proccode)
    } else {
        format!(
            "{}{}",
            block.1["opcode"].as_str().unwrap(),
            rng.gen_range(0..99999999999999999u64)
        )
    };

    // TODO Remove this
    Ok((function, start_type, name, arguments, custom_block))
}

/// Returns all stacks of blocks.
fn create_all_hats(
    blocks: &JsonValue,
    block_reference: &HashMap<&str, &str>,
    name: String,
) -> Result<String, String> {
    let mut contents: String = String::new();

    // Get all the custom blocks
    let mut custom_blocks: HashMap<String, String> = HashMap::new();
    for block in blocks.entries() {
        let cblock = handle_custom_block(block, blocks, block_reference);

        if let Some((procode, func)) = cblock {
            custom_blocks.insert(procode, func);
        };
    }

    let custom_block_reference = custom_blocks.clone();
    // Expand all custom blocks in the definitions of the custom blocks
    for definition in custom_blocks.values_mut() {
        expand_custom_blocks(definition, &custom_block_reference);
    }

    let name_arg = if name == "Stage" {
        "None".to_string()
    } else {
        format!("Some(stage.sprites[{}_index].clone())", name)
    };

    let uuid = if name == "Stage" {
        "None".to_string()
    } else {
        format!("Some(stage.sprites[{}_index].lock().unwrap().uuid)", name)
    };

    let mut stacks = Vec::new();

    for block in blocks.entries() {
        let hat = create_hat(block, blocks, block_reference, name.clone());
        match hat {
            Ok((function, start_type, function_name, arguments, custom_block)) => {
                stacks.push(format!(
                    "v.push(Thread::new(stack_{}(Some(sprite.clone()),stage.clone()),{start_type},Some(sprite.lock().unwrap().uuid)));",
                    function_name
                ));
                match custom_block{
                    false => contents.push_str(format!(
                    // "program.add_thread(Thread{{function:{},obj_index:Some(program.objects.len()),complete:false}});\n",
                    "async fn stack_{function_name}(sprite: Option<Rc<Mutex<Sprite>>>, stage: Rc<Mutex<Stage>> {arguments}){{{function}}}
                    {{
                        let stage = Stage.lock().unwrap();
                        program.add_thread(Thread::new(
                            stack_{function_name}({name_arg},Stage.clone()),{start_type}, {uuid}
                        ));\n
                    }}
",
                    // sprite_name = name_arg,
                ) .as_str()),
                    true => contents.push_str(format!("async fn stack_{function_name}(sprite:Option<Rc<Mutex<Sprite>>>,stage:Rc<Mutex<Stage>> {arguments}){{{function}}}").as_str())
                }
            }
            Err(x) => match &*x {
                "Not a top level block" => continue,
                "Block has a parent" => continue,
                "Not a hat block" => continue,
                _ => return Err(x),
            },
        }
    }

    contents.push_str(&format!(
        "

        fn clone_{name}(sprite: Rc<Mutex<Sprite>>, stage:Rc<Mutex<Stage>>) -> Vec<Thread>{{
            let mut v = Vec::new();
            {threads}
            v
        }}",
        threads = stacks.join("\n")
    ));

    Ok(contents)
}

/// Expand all the custom blocks in a function.
fn expand_custom_blocks(function: &mut String, custom_blocks: &HashMap<String, String>) {
    for (name, definition) in custom_blocks {
        *function = function.replacen(name, definition, 1);
    }
}

/// Get the variables from a target.
///
/// The string constructs a new HashMap with the variables
/// in it.
fn get_variables(target: &JsonValue) -> Result<String, &str> {
    // let mut to_return = String::from("HashMap::from([");
    let mut to_return = String::new();
    for (key, value) in target["variables"].entries() {
        // cloud variables are not supported
        if let Some(true) = value[2].as_bool() {
            return Err("Does not support cloud variables");
        }

        // if the value is a string, include quotation marks                    v        v
        if value[1].is_string() {
            to_return.push_str(&format!(
                ".add_variable(String::from(r###\"{key}\"###),(String::from(r###\"{name}\"###),Value::from(r###\"{value}\"###)))\n",
                name = value[0],
                value = value[1],
            ))
        } else {
            //otherwise don't include quotation marks.
            to_return.push_str(&format!(
                ".add_variable(String::from(\"{key}\"),(String::from(\"{name}\"),Value::from({value})))\n",
                name = value[0],
                value = value[1],
            ));
        }
    }

    // to_return.push_str("])");

    // return Ok(to_return);
    Ok(to_return)
}

fn get_lists(target: &JsonValue) -> Result<String, &str> {
    let mut to_return = String::new();
    for (key, value) in target["lists"].entries() {
        let mut list = String::from("vec![");

        for item in value[1].members() {
            if item.is_string() {
                list.push_str(&format!("Value::from(\"{}\")", item));
            } else {
                list.push_str(&format!("Value::from({})", item));
            }
            list.push(',');
        }
        list.push(']');

        to_return.push_str(&format!(
            ".add_list(String::from(\"{key}\"),(String::from(\"{name}\"),{list}))",
            name = value[0],
        ))
    }
    Ok(to_return)
}

/// Generate a new target(sprite or stage) from json.
fn generate_target(
    target: &JsonValue,
    block_reference: &HashMap<&str, &str>,
) -> Result<String, String> {
    // If the target is the stage
    if target["isStage"].as_bool().unwrap() {
        let function = create_all_hats(
            &target["blocks"],
            block_reference,
            target["name"].to_string(),
        )?;
        Ok(format!(
            "/*let mut {name}=Rc::new(Mutex::new(Stage{{
                tempo:{tempo},
                video_state:{videoState},
                video_transparency:{videoTransparency},
                text_to_speech_language:String::from(\"{textToSpeechLanguage}\"),
                variables:{variables},
                costume:0,
                costumes:Vec::new(),
            }}));*/
            //let mut tempo={tempo};
            //let mut video_state={videoState};
            //let mut video_transparency={videoTransparency};
            //let mut text_to_speech_language=String::from(\"{textToSpeechLanguage}\");
            //let mut global_variables:HashMap<String,Value> =HashMap::new();
            //let mut currentCostume:usize=0;

            let mut {name}=Rc::new(Mutex::new(
                StageBuilder::new()
                    .tempo({tempo})
                    .video_state({videoState})
                    .video_transparency({videoTransparency})
                    {costume}
                    {sounds}
                    {variables}
                    {lists}
                    .set_volume({volume}f32)
                    .build()
            ));
            {function}
",
            name = target["name"],
            tempo = target["tempo"],
            videoState = VideoState::from_str(target["videoState"].as_str().unwrap())
                .unwrap()
                .to_str(),
            textToSpeechLanguage = target["textToSpeechLanguage"],
            videoTransparency = target["videoTransparency"],
            variables = get_variables(target).expect("There are no cloud variables"),
            lists = get_lists(target).unwrap(),
            costume = target_costumes(target),
            sounds = target_sounds(target),
            volume = target["volume"],
        ))
    } else {
        /* let function = create_hat(
            (
                ";R9G|C|f#(g@5F[3Im)I",
                &target["blocks"][";R9G|C|f#(g@5F[3Im)I"],
            ),
            &target["blocks"],
            &block_reference,
        )
        .unwrap(); */
        let function = create_all_hats(
            &target["blocks"],
            block_reference,
            target["name"].to_string(),
        )?;

        Ok(format!(
            "/*let mut {name}=Rc::new(Mutex::new(Sprite{{
                visible:{visible},
                x:{x}f32,
                y:{y}f32,
                size:{size}f32,
                direction:{direction}f32,
                draggable:{draggable},
                rotation_style:{rotationStyle},
                name:\"{name}\".to_string(),
                variables:{variables},
                costume:0,
                costumes:Vec::new(),
            }}));*/


            let {name}_index = Stage.lock().unwrap().sprites.len();
            Stage.lock().unwrap().add_sprite(Rc::new(Mutex::new(
                SpriteBuilder::new(\"{name}\".to_string())
                    .visible({visible})
                    .position({x}f32,{y}f32)
                    .direction({direction}f32)
                    .draggable({draggable})
                    .rotation_style({rotationStyle})
                    .layer({layer})
                    {costumes}
                    {sounds}
                    {variables}
                    {lists}
                    .set_volume({volume}f32)
                    .build()
            )));

            // {{
            //     let mut stage = Stage.lock().unwrap();
            //     stage.add_sprite({name});
            // }}

            //sprites.push({name}.clone());

            {function}",
            name = target["name"],
            visible = target["visible"],
            x = target["x"],
            y = target["y"],
            size = target["size"],
            layer = target["layerOrder"],
            direction = target["direction"],
            variables = get_variables(target).expect("There should be no cloud variables"),
            lists = get_lists(target).unwrap(),
            draggable = target["draggable"],
            rotationStyle = RotationStyle::from_str(target["rotationStyle"].as_str().unwrap())
                .unwrap()
                .to_str(),
            costumes = target_costumes(target),
            sounds = target_sounds(target),
            volume = target["volume"],
        ))
    }
}

/// Fetch a scratch sb3 file.
fn fetch_sb3_file(url: String) -> String {
    // fetch the file,
    let response = ureq::get(&url).call().expect("Could not get file"); // TODO better error handling

    // create a new file,
    //let mut out=File::create("project.sv3").expect("Could not create file");
    // and copy the contents of the response to the new file.
    //response.copy_to(&mut out);
    // io::copy(&mut response,&mut out)?;
    response.into_string().unwrap()
}

/// Get the project token for a scratch project.
fn fetch_project_token(id: u32) -> Result<String, &'static str> {
    let url = format!("https://api.scratch.mit.edu/projects/{id}");
    let text = match ureq::get(&url).call() {
        Ok(res) => res
            .into_string()
            .expect("Response can be converted to string"),
        Err(_) => return Err("Could not get from api"),
    };

    let json = json::parse(&text).or(Err("Cannot parse json"))?;

    let token = &json["project_token"];

    if token.is_null() {
        Err("Token does not exist")
    } else {
        Ok(token.to_string())
    }
}

/// Formats the given filename with rustfmt.
fn format_file(filename: &PathBuf) -> io::Result<Output> {
    return Command::new("rustfmt").arg(filename).output();
    //.expect("Could not execute rustfmt");
}

fn create_project(path: &PathBuf) -> Result<(), io::Error> {
    Command::new("cargo").arg("new").arg(path).output()?;

    let toml = "
    [package]
    name=\"output\"
    version=\"1.0.0\"
    edition=\"2021\"
    
    [dependencies]
    rand=\"0.8.5\"
    resvg = \"0.25.0\"
    chrono = \"0.4.23\"
    uuid = {version = \"1.4.1\", features = [\"v4\",\"fast-rng\"]}

    sdl2 = \"0.36\"
    glium = \"0.34\"
    image = \"0.25.1\"

    ";

    let toml_path = {
        let mut p = path.clone();
        p.push("Cargo.toml");
        p
    };
    fs::write(toml_path, toml)?;

    Ok(())
}

/// Download the assets for a target.
fn get_target_assets(target: &JsonValue, path: &Path) -> Result<(), Box<dyn Error>> {
    // create the asset directory
    fs::create_dir_all({
        let mut p = path.to_path_buf();
        p.push("assets");
        p.push(target["name"].to_string());
        p
    })?;

    // iterate through all costumes
    for costume in target["costumes"].members() {
        let response = ureq::get(&format!(
            "https://assets.scratch.mit.edu/{}",
            costume["md5ext"]
        ))
        .call()?;

        let mut file = std::fs::File::create({
            let mut p = path.to_path_buf();
            p.push("assets");
            p.push(target["name"].to_string());
            p.push(format!("{}.{}", costume["name"], costume["dataFormat"]));
            p
        })?;

        let mut content = response.into_reader();
        std::io::copy(&mut content, &mut file)?;
    }

    // Iterate through all sounds and download them.
    for sound in target["sounds"].members() {
        let response = ureq::get(&format!(
            "https://assets.scratch.mit.edu/{}",
            sound["md5ext"]
        ))
        .call()?;

        let mut file = std::fs::File::create({
            let mut p = path.to_path_buf();
            p.push("assets");
            p.push(target["name"].to_string());
            p.push(format!("{}.{}", sound["name"], sound["dataFormat"]));
            p
        })?;

        let mut content = response.into_reader();
        std::io::copy(&mut content, &mut file)?;
    }

    Ok(())
}

/// Get all the target costumes
fn target_costumes(target: &JsonValue) -> String {
    let mut to_return = String::new();
    let name = &target["name"];
    for costume in target["costumes"].members() {
        let costumename = &costume["name"];
        let format = &costume["dataFormat"];
        let costume_name = costume["name"].to_string();

        // TODO handle png files properly
        if format != "svg" {
            continue;
        }

        // to_return.push_str(&format!("program.add_costume_{stage_or_sprite}(
        //                                 Costume::new(PathBuf::from(\"assets/{name}/{costumename}.{format}\"),1.0).unwrap(),
        //                                 &mut {name}
        //                             );\n"));
        to_return.push_str(&format!(".add_costume(Costume::new(&window, String::from(\"{costume_name}\"),PathBuf::from(\"assets/{name}/{costumename}.{format}\"),1.0).unwrap())\n"))
    }

    to_return
}

fn target_sounds(target: &JsonValue) -> String {
    let mut to_return = String::new();

    for sound in target["sounds"].members() {
        let sound_name = &sound["name"];
        let format = &sound["dataFormat"];
        let rate = &sound["rate"];
        let sample_count = &sound["sampleCount"];

        to_return.push_str(&format!(
            ".add_sound(
                Sound::new(
                    String::from(\"{sound_name}\"),
                    String::from(\"{format}\"),
                    {rate},
                    {sample_count}
                )
             )"
        ));
    }

    to_return
}

/// Create the readme for a given scratch project
fn create_readme(json: &json::JsonValue) -> Result<String, Box<dyn Error>> {
    let title = &json["title"].as_str().unwrap();
    let description = &json["description"].as_str().unwrap();
    let instructions = &json["instructions"].as_str().unwrap();

    Ok(format!(
        "
# {title}
## Instructions
{instructions}
## Notes and Credits
{description}
    "
    ))
}

fn get_project_details(id: u64) -> Result<JsonValue, Box<dyn Error>> {
    let response = ureq::get(&format!("https://api.scratch.mit.edu/projects/{}", id)).call()?;
    let json: JsonValue = json::parse(&response.into_string()?)?;
    Ok(json)
}
