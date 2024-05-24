 
# Rusty-Scratch
Rusty-Scratch is a transpiler that turns [scratch](https://scratch.mit.edu)
projects into rust so that they will run faster. It is similar to
[[https://turbowarp.org/][turbowarp]].

## Installation
1. Make sure rust is installed on your machine. ([install
   rust](https://www.rust-lang.org/tools/install))
2. Clone the repo.
``` sh
git clone https://github.com/Tallented-Code-bot/rusty-scratch.git
```
3. Build.

``` sh
cd rusty-scratch
cargo build
```

# Resources

## Blocks
Here are all the currently implemented blocks.


- [X] Move (10) steps
- [X] turn right (15) degrees
- [X] Turn left (15) degrees
- [X] Go to [random position]
- [X] go to x()y()
- [ ] glide (1) secs to [random position]
- [ ] glide 1 secs to x:(0) y: (0)
- [X] point in direction (90)
- [ ] point towards [mouse pointer]
- [X] change x by (10)
- [X] set x to (0)
- [X] change y by (10)
- [X] set y to (0)
- [ ] if on edge, bounce
- [X] set rotation style [left-right]
- [X] (x position)
- [X] (y position)
- [X] (direction)
- [ ] say (hello) for (2) seconds
- [X] say (hello)
- [ ] think (hmmn) for (2) seconds
- [ ] think (hmm)
- [ ] switch costume to (v costume2)
- [ ] next costume
- [ ] switch backdrop to (v backdrop1)
- [ ] next backdrop
- [ ] change size by (10)
- [ ] set size to (100)%
- [ ] change [v color] effect by (25)
- [ ] set [v color] effect to 0
- [ ] clear graphic effects
- [X] show
- [X] hide
- [ ] go to [front] layer
- [ ] go [forward] (1) layers
- [ ] (costume [number])
- [ ] (backdrop [number])
- [ ] (size)
- [ ] play sound (v meow) until done
- [ ] start sound (v meow)
- [ ] stop all sounds
- [ ] change [pitch v] effect by (10)
- [ ] set [pitch v] effect to (100)
- [ ] clear sound effects
- [ ] change volume by (-10)
- [ ] set volume to (100)%
- [ ] (volume)
- [ ] when gf clicked
- [ ] When [space] key pressed
- [ ] when this sprite clicked
- [ ] when backdrop switches to [backdrop1]
- [ ] When [loudness] > (10)
- [ ] when I recieve [message1]
- [ ] broadcast (message1 v)
- [ ] broadcast (message1 v) and wait
- [ ] wait (1) second
- [X] repeat (10) {}
- [X] forever {}
- [X] if <> then{}
- [X] if <> then{}else{}
- [ ] wait until <>
- [ ] repeat until <>{}
- [ ] stop [all v]
- [ ] when I start as a clone
- [ ] create clone of (myself v)
- [ ] delete this clone
- [ ] <touching (mouse-pointer v)>
- [ ] <touching color (orange)>
- [ ] <color (blue) is touching (orange)?>
- [ ] (distance to (mouse pointer v))
- [ ] ask (what's your name) and wait
- [ ] (answer)
- [ ] <key (space v) pressed?>
- [ ] <mouse down?>
- [ ] (mouse x)
- [ ] (mouse y)
- [ ] set drag mode [draggable v]
- [ ] (loudness)
- [ ] (timer)
- [ ] reset timer
- [ ] ([backdrop #] of (stage v))
- [ ] (current [year v])
- [-] (days since 2000)
- [-] (username)
- [X] (()+())
- [X] (()-())
- [X] (()*())
- [X] (()/())
- [X] (pick random (1) to (10))
- [X] <()>()>
- [X] <()<()>
- [X] <()=()>
- [X] <<>and<>>
- [X] <<>or<>>
- [X] <not <>>
- [X] (join (apple)(bannana))
- [X] (letter (1) of (apple))
- [X] (length of (apple))
- [X] <(apple) contains (a)?>
- [ ] (() mod ())
- [X] (round ())
- [ ] ([abs v] of ())
- [X] set [my variable] to (0)
- [X] change [my variable] by (1)
- [ ] show variable [my variable]
- [ ] hide variable [my variable]
- [X] add (thing) to [test v]
- [X] delete (1) of [test]
- [X] insert (thing) at (1) of [test]
- [X] replace item (1) of [test] with (thing)
- [X] (item (1) of [test])
- [X] (item \# of (thing) in [test])
- [X] (length of [test])
- [X] <[test] contains (thing)?>
- [ ] show list [test]
- [ ] hide list [test]
